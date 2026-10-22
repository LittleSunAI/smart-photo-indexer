use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub(super) struct ViewerSessions(Mutex<HashMap<String, ViewerSession>>);

const MAX_VIEWER_PAGE: usize = 51;
fn viewer_size_label(bytes:u64)->String{
    if bytes<1024{format!("{bytes} Б")}
    else if bytes<1_048_576{format!("{:.1} КБ",bytes as f64/1024.)}
    else if bytes<1_073_741_824{format!("{:.1} МБ",bytes as f64/1_048_576.)}
    else{format!("{:.1} ГБ",bytes as f64/1_073_741_824.)}
}
struct ViewerSession {
    paths: Arc<Vec<PathBuf>>,
    context_key: String,
    payload: serde_json::Value,
}
impl std::ops::Deref for ViewerSession {
    type Target = serde_json::Value;
    fn deref(&self) -> &Self::Target { &self.payload }
}
impl std::ops::DerefMut for ViewerSession {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.payload }
}

fn page_indices(total: usize, active: usize) -> Vec<usize> {
    if total <= MAX_VIEWER_PAGE { return (0..total).collect(); }
    let start = (active + total - MAX_VIEWER_PAGE / 2) % total;
    (0..MAX_VIEWER_PAGE).map(|offset| (start + offset) % total).collect()
}

fn load_page(app: &tauri::AppHandle, record: &mut ViewerSession, active: usize, connection: &Connection) -> Result<(),String> {
    let indices = page_indices(record.paths.len(), active);
    let old = record["images"].as_array().cloned().unwrap_or_default();
    let mut images = Vec::with_capacity(indices.len());
    for index in indices {
        let path = &record.paths[index];
        let text = path_to_string(path);
        if let Some(existing) = old.iter().find(|item| index != active && item["path"].as_str() == Some(text.as_str())) {
            images.push(existing.clone());
        } else {
            images.push(viewer_item(app,path,connection).unwrap_or_else(|error|serde_json::json!({
                "id":format!("image:{text}"),"kind":"image","path":text,"name":file_name(path),
                "status":"error","description":"","tags":[],"collections":[],"dimensions":error
            })));
        }
    }
    record["images"] = serde_json::json!(images);
    record["activeImageId"] = record.paths.get(active).map(|path|serde_json::json!(format!("image:{}",path_to_string(path)))).unwrap_or(serde_json::Value::String(String::new()));
    record["totalImages"] = serde_json::json!(record.paths.len());
    record["activePosition"] = serde_json::json!(active);
    record["pageLimit"] = serde_json::json!(MAX_VIEWER_PAGE);
    Ok(())
}

fn normalized_file(path: &str) -> Result<PathBuf, String> {
    let path = validate_existing_image_file(path)?;
    path.canonicalize().map_err(|error| format!("Cannot open image path: {error}"))
}

fn viewer_item(app: &tauri::AppHandle, path: &Path, connection: &Connection) -> Result<serde_json::Value, String> {
    let text = path_to_string(path);
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    app.asset_protocol_scope().allow_file(path).map_err(|error| format!("Cannot authorize image preview: {error}"))?;
    let indexed: Option<(String, Option<String>, String, bool)> = connection.query_row(
        "SELECT i.id,i.description,i.status,i.is_favorite FROM images i JOIN image_paths p ON i.id=p.image_id WHERE p.path=?1",
        params![&text], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))
    ).optional().map_err(|error| error.to_string())?;
    let dimensions = image::ImageReader::open(path).ok().and_then(|reader| reader.into_dimensions().ok())
        .map(|(width,height)| format!("{width} × {height}")).unwrap_or_else(|| "Размер недоступен".to_string());
    let (image_id, description, status, favorite, tags, collections) = if let Some((id, description, status, favorite)) = indexed {
        let tags = read_tags_for_image_from_connection(connection, &id)?;
        let collections = read_collection_names_for_image_from_connection(connection, &id)?;
        (Some(id),description,status,favorite,tags,collections)
    } else { (None,None,"notProcessed".into(),false,Vec::new(),Vec::new()) };
    Ok(serde_json::json!({
        "id": format!("image:{text}"), "kind":"image", "name":file_name(path), "path":text,
        "indexedImageId":image_id, "description":description.unwrap_or_default(), "status":status,
        "sizeBytes":metadata.len(), "sizeLabel":viewer_size_label(metadata.len()),"sourceRevision":index_observations::content_revision(path).ok(),
        "dimensions":dimensions, "modifiedAtUnix":modified_at_unix(&metadata), "extension":extension(path),
        "isFavorite":favorite, "tags":tags, "collections":collections,
    }))
}

pub(super) fn open(app: &tauri::AppHandle, paths: Vec<String>, active_path: String, adjacent: bool) -> Result<String, String> {
    let active = normalized_file(&active_path)?;
    let mut files = Vec::<PathBuf>::new();
    if adjacent {
        let parent = active.parent().ok_or("Image directory is unavailable.")?;
        let entries = fs::read_dir(parent).map_err(|error| error.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.is_file() && is_supported_image(&path) { files.push(path); }
        }
        files.sort_by_key(|path| file_name(path).to_lowercase());
    } else {
        for path in paths {
            let path=PathBuf::from(path);
            if !path.is_absolute() || !is_supported_image(&path){return Err("Viewer context requires absolute image paths.".into());}
            files.push(path);
        }
    }
    let active_key=path_to_string(&active).replace('/',"\\").to_lowercase();
    if let Some(file)=files.iter_mut().find(|file|path_to_string(file).replace('/',"\\").to_lowercase()==active_key){*file=active.clone();}
    else {files.push(active.clone());}
    let mut seen = std::collections::HashSet::new();
    files.retain(|path| seen.insert(path_to_string(path).to_lowercase()));
    let mut context_hash = Sha256::new();
    for path in &files { context_hash.update(path_to_string(path).to_lowercase().as_bytes()); context_hash.update([0]); }
    let context_key = format!("{:x}",context_hash.finalize());
    let label = format!("smart-photo-viewer-{:x}",Sha256::digest(format!("{}\n{context_key}",path_to_string(&active).to_lowercase()).as_bytes()));
    if let Some(window) = app.get_webview_window(&label) {
        crate::window_appearance::present(&window)?;
        return Ok(label);
    }
    let mut connection = open_index_connection(app)?;
    run_index_migrations(&mut connection)?;
    let position = files.iter().position(|path|path == &active).ok_or("Active image is outside the viewer context.")?;
    let shared_paths = {
        let state=app.state::<ViewerSessions>();
        let sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
        sessions.values().find(|record|record.context_key == context_key).map(|record|Arc::clone(&record.paths))
    }.unwrap_or_else(||Arc::new(files));
    let mut session = ViewerSession {paths:shared_paths,context_key,payload:serde_json::json!({"images":[],"openedAtUnix":unix_now()})};
    load_page(app,&mut session,position,&connection)?;
    app.state::<ViewerSessions>().0.lock().map_err(|_| "Viewer session lock failed.")?
        .insert(label.clone(),session);
    let (width,height) = window_size(app);
    let result = WebviewWindowBuilder::new(app,&label,WebviewUrl::App(
        format!("index.html?viewer=1&viewerLabel={label}").into()))
        .title(file_name(&active)).inner_size(width,height).min_inner_size(480.0,320.0)
        .resizable(true).decorations(false).maximized(true)
        .theme(Some(crate::window_appearance::theme(app))).visible(false)
        .prevent_overflow().center().build();
    if let Err(error) = result {
        remove(app,&label);
        return Err(format!("Cannot create image viewer: {error}"));
    }
    Ok(label)
}

fn window_size(app: &tauri::AppHandle) -> (f64,f64) {
    app.primary_monitor().ok().flatten().map(|monitor| {
        let scale = monitor.scale_factor();
        ((monitor.work_area().size.width as f64 / scale - 64.0).clamp(480.0,1280.0),
         (monitor.work_area().size.height as f64 / scale - 96.0).clamp(320.0,860.0))
    }).unwrap_or((1000.0,700.0))
}

pub(super) fn show_main(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        return crate::window_appearance::present(&window);
    }
    let (width,height) = window_size(app);
    WebviewWindowBuilder::from_config(app,&app.config().app.windows[0]).map_err(|error| error.to_string())?
        .inner_size(width,height).theme(Some(crate::window_appearance::theme(app)))
        .visible(false).prevent_overflow().center().build().map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn remove(app: &tauri::AppHandle, label: &str) {
    if let Ok(mut sessions) = app.state::<ViewerSessions>().0.lock() { sessions.remove(label); }
}
pub(super) fn owns_active_path(app:&tauri::AppHandle,label:&str,path:&Path)->Result<bool,String>{
    let state=app.state::<ViewerSessions>();let sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
    let session=sessions.get(label).ok_or("Окно просмотра уже закрыто.")?;
    let active=session["activeImageId"].as_str().unwrap_or("");
    let text=session["images"].as_array().and_then(|rows|rows.iter().find(|row|row["id"].as_str()==Some(active))).and_then(|row|row["path"].as_str()).ok_or("Активный кадр недоступен.")?;
    Ok(normalized_file(text)?==path)
}
pub(super) fn help_target(app:&tauri::AppHandle,path:&Path)->Result<Option<String>,String>{
 let state=app.state::<ViewerSessions>();let sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
 let mut labels:Vec<_>=sessions.iter().filter_map(|(label,session)|{let active=session["activeImageId"].as_str()?;let text=session["images"].as_array()?.iter().find(|row|row["id"].as_str()==Some(active))?["path"].as_str()?;if Path::new(text).canonicalize().ok().as_deref()==Some(path)&&app.get_webview_window(label).is_some(){Some(label.clone())}else{None}}).collect();labels.sort();Ok(labels.into_iter().next())
}

#[tauri::command]
pub(super) async fn open_viewer_window(app_handle: tauri::AppHandle, paths: Vec<String>, active_path: String) -> Result<String,String> {
    tauri::async_runtime::spawn_blocking(move || open(&app_handle,paths,active_path,false))
        .await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub(super) async fn get_viewer_session(app_handle: tauri::AppHandle, window: tauri::WebviewWindow, label: String) -> Result<serde_json::Value,String> {
    if window.label() != label { return Err("Viewer sessions belong to their own native window.".into()); }
    tauri::async_runtime::spawn_blocking(move || {
    app_handle.state::<ViewerSessions>().0.lock().map_err(|_| "Viewer session lock failed.")?
        .get(&label).map(|record|record.payload.clone()).ok_or_else(|| "Viewer session was closed.".into())
    }).await.map_err(|error|error.to_string())?
}

#[tauri::command]
pub(super) async fn get_viewer_session_statistics(app_handle:tauri::AppHandle)->Result<serde_json::Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let state=app_handle.state::<ViewerSessions>();
        let sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
        let contexts=sessions.values().map(|record|Arc::as_ptr(&record.paths)).collect::<std::collections::HashSet<_>>();
        Ok(serde_json::json!({"windows":sessions.len(),"sharedPathLists":contexts.len(),"hydratedImages":sessions.values().map(|record|record["images"].as_array().map_or(0,Vec::len)).sum::<usize>(),"pageLimit":MAX_VIEWER_PAGE}))
    }).await.map_err(|error|error.to_string())?
}

pub(super) fn launch_arguments(app: &tauri::AppHandle, args: Vec<String>, cwd: String) -> Result<(), String> {
    let mut paths = Vec::new();
    for arg in args.into_iter().skip(1) {
        if arg == "--viewer" { continue; }
        let path = PathBuf::from(&arg);
        let path = if path.is_absolute() { path } else { Path::new(&cwd).join(path) };
        if path.is_file() && is_supported_image(&path) { paths.push(path_to_string(&path)); }
        else { return Err(format!("Image argument is not a supported file: {arg}")); }
    }
    if let Some(active) = paths.first().cloned() { open(app,paths.clone(),active,paths.len()==1)?; }
    else { show_main(app)?; }
    Ok(())
}

pub(super) fn remove_path(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    let connection=open_index_connection(app)?;
    let state = app.state::<ViewerSessions>();
    let mut sessions = state.0.lock().map_err(|_| "Viewer session lock failed.")?;
    for (label, session) in sessions.iter_mut() {
        let old_position=session["activePosition"].as_u64().unwrap_or(0) as usize;
        Arc::make_mut(&mut session.paths).retain(|candidate|path_to_string(candidate)!=path);
        session.context_key.clear();
        if let Some(images) = session["images"].as_array_mut() {
            images.retain(|image| image["path"].as_str() != Some(path));
        }
        let active = session["activeImageId"].as_str().unwrap_or_default();
        if !session["images"].as_array().is_some_and(|images| images.iter().any(|image| image["id"].as_str() == Some(active))) {
            session["activeImageId"] = session["images"][0]["id"].clone();
        }
        let active_id=session["activeImageId"].as_str().unwrap_or_default().to_string();
        let position=session.paths.iter().position(|candidate|format!("image:{}",path_to_string(candidate))==active_id).unwrap_or(old_position.min(session.paths.len().saturating_sub(1)));
        load_page(app,session,position,&connection)?;
        app.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|error| error.to_string())?;
    }
    app.emit("smart-photo-indexer:viewer-image-deleted", serde_json::json!({ "itemId":"","path":path,"sourceWindowLabel":"backend" }))
        .map_err(|error| error.to_string())
}

pub(super) fn update_favorite(app: &tauri::AppHandle, result: &SetImageFavoriteResultDto) -> Result<(),String> {
    let state = app.state::<ViewerSessions>();
    let mut sessions = state.0.lock().map_err(|_| "Viewer session lock failed.")?;
    for (label,session) in sessions.iter_mut() {
        if let Some(images) = session["images"].as_array_mut() {
            for image in images {
                if image["indexedImageId"].as_str() == Some(result.image_id.as_str()) || image["path"].as_str() == Some(result.path.as_str()) {
                    image["indexedImageId"] = serde_json::Value::String(result.image_id.clone());
                    image["isFavorite"] = serde_json::Value::Bool(result.is_favorite);
                }
            }
        }
        app.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|error| error.to_string())?;
    }
    app.emit("smart-photo-indexer:image-favorite-updated",serde_json::json!({ "result":result,"sourceWindowLabel":"backend" }))
        .map_err(|error| error.to_string())
}

pub(super) fn update_metadata(app:&tauri::AppHandle,result:&IndexMetadataUpdateResultDto)->Result<(),String>{
    let state=app.state::<ViewerSessions>();let mut sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
    for (label,session) in sessions.iter_mut(){
        let mut changed=false;
        if let Some(images)=session["images"].as_array_mut(){for image in images{
            if image["indexedImageId"].as_str()==Some(result.image_id.as_str()) || image["path"].as_str()==Some(result.path.as_str()){
                image["indexedImageId"]=serde_json::json!(result.image_id);image["description"]=serde_json::json!(result.description);
                image["tags"]=serde_json::json!(result.tags);image["status"]=serde_json::json!(result.status);
                image["descriptionVersion"]=serde_json::json!(result.description_version);image["tagsVersion"]=serde_json::json!(result.tags_version);changed=true;
            }
        }}
        if changed{app.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|e|e.to_string())?;}
    }Ok(())
}

pub(super) fn rename_path(app:&tauri::AppHandle,old_path:&str,new_path:&str)->Result<(),String>{
    rename_paths(app,&[(old_path.to_string(),new_path.to_string())])
}
fn rename_payload_paths(payload:&mut serde_json::Value,mapping:&std::collections::HashMap<&str,&str>)->bool{
    let mut active=payload["activeImageId"].as_str().unwrap_or_default().to_string();let mut changed=false;
    if let Some(images)=payload["images"].as_array_mut(){for image in images{
        if let Some(new_path)=image["path"].as_str().and_then(|path|mapping.get(path)).copied(){
            let new_id=format!("image:{new_path}");if image["id"].as_str()==Some(active.as_str()){active=new_id.clone();}
            image["id"]=serde_json::json!(new_id);image["path"]=serde_json::json!(new_path);image["name"]=serde_json::json!(file_name(Path::new(new_path)));
            image["previewUrl"]=serde_json::Value::Null;changed=true;
        }
    }}
    if changed{payload["activeImageId"]=serde_json::json!(active);}changed
}
pub(super) fn rename_paths(app:&tauri::AppHandle,mappings:&[(String,String)])->Result<(),String>{
    let mapping:std::collections::HashMap<&str,&str>=mappings.iter().map(|(old,new)|(old.as_str(),new.as_str())).collect();
    for (_,new_path) in mappings { app.asset_protocol_scope().allow_file(new_path).map_err(|e|e.to_string())?; }
    let state=app.state::<ViewerSessions>();let mut sessions=state.0.lock().map_err(|_|"Viewer session lock failed.")?;
    for (label,session) in sessions.iter_mut(){
        if session.paths.iter().any(|path|mapping.contains_key(path_to_string(path).as_str())) {
            for path in Arc::make_mut(&mut session.paths) { if let Some(new_path)=mapping.get(path_to_string(path).as_str()) { *path=PathBuf::from(new_path); } }
            session.context_key.clear();
        }
        let changed=rename_payload_paths(&mut session.payload,&mapping);
        if changed{app.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|e|e.to_string())?;}
    }Ok(())
}

#[tauri::command]
pub(super) async fn set_viewer_active(app_handle: tauri::AppHandle, window: tauri::WebviewWindow, item_id: String, navigation_revision: Option<u64>) -> Result<(),String> {
    let label=window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
    let connection=open_index_connection(&app_handle)?;
    let state = app_handle.state::<ViewerSessions>();
    let mut sessions = state.0.lock().map_err(|_| "Viewer session lock failed.")?;
    let session = sessions.get_mut(&label).ok_or("Viewer session was closed.")?;
    let previous=session["navigationRevision"].as_u64().unwrap_or(0);
    let revision=navigation_revision.unwrap_or(previous.saturating_add(1));
    if revision<=previous {return Ok(());}
    if !session["images"].as_array().is_some_and(|images| images.iter().any(|image| image["id"].as_str() == Some(item_id.as_str()))) {
        return Err("Image is outside this viewer session.".into());
    }
    let position=session.paths.iter().position(|path|format!("image:{}",path_to_string(path))==item_id).ok_or("Image is outside this viewer context.")?;
    load_page(&app_handle,session,position,&connection)?;
    session["navigationRevision"]=serde_json::json!(revision);
    app_handle.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|error|error.to_string())
    }).await.map_err(|error|error.to_string())?
}

#[tauri::command]
pub(super) async fn open_search_viewer_window(app_handle:tauri::AppHandle,input:IndexSearchInputDto,active_path:String)->Result<String,String>{
    let query_id=crate::search::fork_search_snapshot(app_handle.clone(),input.query_id.ok_or("Сначала выполните поиск.")?).await?;
    let worker_app=app_handle.clone();let worker_id=query_id.clone();
    let result=tauri::async_runtime::spawn_blocking(move||{
        let paths=crate::search::path_context(&worker_app,&worker_id)?;
        if !paths.iter().any(|path|path.replace('/',"\\").to_lowercase()==active_path.replace('/',"\\").to_lowercase()){
            return Err("Выбранное фото не входит в снимок поиска.".into());
        }
        open(&worker_app,paths,active_path,false)
    }).await.map_err(|error|error.to_string()).and_then(|result|result);
    crate::search::close_search_snapshot(app_handle,query_id).await?;result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renamed_active_file_keeps_a_valid_path_id_after_content_refresh(){
        let mut payload=serde_json::json!({"activeImageId":"image:old.png","activePosition":7,"navigationRevision":23,"images":[{"id":"image:old.png","path":"old.png"},{"id":"image:other.png","path":"other.png"}]});
        assert!(rename_payload_paths(&mut payload,&std::collections::HashMap::from([("old.png","new.png")])));
        assert_eq!(payload["activeImageId"],"image:new.png");assert_eq!(payload["images"][0]["id"],"image:new.png");
        payload["images"][0]=serde_json::json!({"id":"image:new.png","path":"new.png","sourceRevision":"fresh"});
        assert!(payload["images"].as_array().unwrap().iter().any(|image|image["id"]==payload["activeImageId"]));
        assert_eq!(payload["activePosition"],7);assert_eq!(payload["navigationRevision"],23);
        assert!(!rename_payload_paths(&mut payload,&std::collections::HashMap::from([("absent.png","unused.png")])));
    }
    #[test]
    fn viewer_pages_keep_global_neighbors_and_wrap_without_copying_the_archive() {
        for total in [0usize,1,3,51,52,501,100_001] {
            for active in [0,total/2,total.saturating_sub(1)] {
                let page=page_indices(total,active);
                assert_eq!(page.len(),total.min(MAX_VIEWER_PAGE));
                assert_eq!(page.iter().copied().collect::<std::collections::HashSet<_>>().len(),page.len());
                if total>MAX_VIEWER_PAGE {
                    let local=page.iter().position(|index|*index==active).unwrap();
                    assert_eq!(page[local+1],(active+1)%total);
                    assert_eq!(page[local-1],(active+total-1)%total);
                } else { assert_eq!(page,(0..total).collect::<Vec<_>>()); }
            }
        }
    }
}

pub(super) fn refresh_contents(app:&tauri::AppHandle,path:&str)->Result<(),String>{
 let db=open_index_connection(app)?;let fresh=viewer_item(app,Path::new(path),&db)?;
 let sessions=app.state::<ViewerSessions>();let mut sessions=sessions.0.lock().map_err(|_|"Viewer session lock failed")?;
 for(label,session)in sessions.iter_mut(){let mut changed=false;if let Some(images)=session["images"].as_array_mut(){for image in images{if image["path"].as_str().is_some_and(|p|p.eq_ignore_ascii_case(path)){*image=fresh.clone();changed=true;}}}if changed{app.emit_to(label.as_str(),"smart-photo-indexer:viewer-native-session",session.payload.clone()).map_err(|e|e.to_string())?;}}Ok(())
}
