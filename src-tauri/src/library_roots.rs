use crate::*;
use notify::{RecursiveMode,Watcher};
use std::collections::{HashMap,HashSet};
use std::sync::{Mutex,atomic::{AtomicBool,Ordering}};
use std::time::{Duration,Instant};

const KEY:&str="library-roots.v1";
fn emit(app:&tauri::AppHandle){use tauri::Emitter;let _=app.emit("smart-photo-indexer:library-roots-changed",());}
#[derive(Clone,Serialize,Deserialize,PartialEq)]
#[serde(rename_all="camelCase")]
pub struct Root { id:String,path:String,excluded_paths:Vec<String>,auto_update:bool,revision:u64 }
#[derive(Clone,Default,Serialize)]
#[serde(rename_all="camelCase")]
struct RootStatus { state:String,error:Option<String>,checked:usize,skipped:usize,updated_at_unix:Option<u64>,watching:bool }
struct Dirty { first:Instant,last:Instant,paths:HashSet<String>,full:bool }
impl Dirty {fn new()->Self{Self{first:Instant::now(),last:Instant::now(),paths:HashSet::new(),full:false}}}
#[derive(Default)]
pub struct LibraryRuntime { gate:Mutex<()>,statuses:Mutex<HashMap<String,RootStatus>>,requested:Mutex<HashSet<String>>,refresh:AtomicBool }

fn read(app:&tauri::AppHandle)->Result<Vec<Root>,String>{
    let mut db=open_index_connection(app)?;run_index_migrations(&mut db)?;
    let raw:Option<String>=db.query_row("SELECT value FROM settings_kv WHERE key=?1",[KEY],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let roots:Vec<Root>=raw.map(|s|serde_json::from_str(&s).map_err(|e|e.to_string())).transpose()?.unwrap_or_default();
    if roots.len()>64{return Err("Слишком много подключённых источников.".into());}Ok(roots)
}
fn write(app:&tauri::AppHandle,roots:&[Root])->Result<(),String>{
    let db=open_index_connection(app)?;
    db.execute("INSERT INTO settings_kv(key,value,updated_at_unix) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at_unix=excluded.updated_at_unix",params![KEY,serde_json::to_string(roots).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
    app.state::<LibraryRuntime>().refresh.store(true,Ordering::Release);emit(app);Ok(())
}
fn normalized(path:&str)->String{path.replace('\\',"/").trim_end_matches('/').to_lowercase()}
fn safe_path(path:&str)->Result<PathBuf,String>{
    let path=PathBuf::from(path.trim());if !path.is_absolute() || path.components().any(|c|matches!(c,Component::ParentDir)){return Err("Нужен полный путь без переходов ..".into());}
    source_paths::require_photo_source(&path)?;Ok(path)
}
#[tauri::command]
pub async fn get_library_roots(app:tauri::AppHandle)->Result<serde_json::Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let roots=read(&app)?;let runtime=app.state::<LibraryRuntime>();let statuses=runtime.statuses.lock().map_err(|_|"Library status lock failed.")?;
        Ok(serde_json::json!({"roots":roots.iter().map(|root|serde_json::json!({"id":root.id,"path":root.path,"excludedPaths":root.excluded_paths,"autoUpdate":root.auto_update,"revision":root.revision,"status":statuses.get(&root.id).cloned().unwrap_or_default()})).collect::<Vec<_>>()}))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn add_library_root(app:tauri::AppHandle,path:String)->Result<String,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<LibraryRuntime>();let _guard=runtime.gate.lock().map_err(|_|"Library lock failed.")?;
        let path=fs::canonicalize(safe_path(&path)?).map_err(|e|format!("Источник недоступен: {e}"))?;source_paths::require_photo_source(&path)?;fs::read_dir(&path).map_err(|e|e.to_string())?;
        let path=path_to_string(&path);let mut roots=read(&app)?;
        if let Some(root)=roots.iter().find(|r|normalized(&r.path)==normalized(&path)){return Ok(root.id.clone());}
        if roots.len()>=64{return Err("Можно подключить не больше 64 источников.".into());}
        let id=format!("root-{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
        roots.push(Root{id:id.clone(),path,excluded_paths:Vec::new(),auto_update:false,revision:1});write(&app,&roots)?;Ok(id)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn update_library_root(app:tauri::AppHandle,root_id:String,excluded_paths:Vec<String>,auto_update:bool,expected_revision:u64)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<LibraryRuntime>();let _guard=runtime.gate.lock().map_err(|_|"Library lock failed.")?;
        let mut roots=read(&app)?;let root=roots.iter_mut().find(|r|r.id==root_id).ok_or("Источник уже отключён.")?;
        if root.revision!=expected_revision{return Err("Настройки источника уже изменены; обновите список.".into());}
        if excluded_paths.len()>128{return Err("Не больше 128 исключений на источник.".into());}
        let mut exclusions=Vec::new();for raw in excluded_paths{let path=safe_path(&raw)?;let path=path.canonicalize().unwrap_or(path);let path=path_to_string(&path);
            if !path_is_under_rescan_root(&path,&Some(PathBuf::from(&root.path))){return Err("Исключение должно находиться внутри источника.".into());}
            if normalized(&path)==normalized(&root.path){return Err("Чтобы отключить весь источник, выключите автообновление или удалите подключение.".into());}
            if !exclusions.iter().any(|old:&String|normalized(old)==normalized(&path)){exclusions.push(path);}
        }
        root.excluded_paths=exclusions;root.auto_update=auto_update;root.revision+=1;write(&app,&roots)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn remove_library_root(app:tauri::AppHandle,root_id:String)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move||{let runtime=app.state::<LibraryRuntime>();let _guard=runtime.gate.lock().map_err(|_|"Library lock failed.")?;
        let mut roots=read(&app)?;roots.retain(|r|r.id!=root_id);write(&app,&roots)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn refresh_library_root(app:tauri::AppHandle,root_id:String)->Result<(),String>{
    // This command schedules work; the IPC reply never waits for disk traversal.
    tauri::async_runtime::spawn_blocking(move||{
    if !read(&app)?.iter().any(|root|root.id==root_id){return Err("Источник не найден.".into());}
    let runtime=app.state::<LibraryRuntime>();let mut pending=runtime.requested.lock().map_err(|_|"Library queue lock failed.")?;
    if pending.len()>=64 && !pending.contains(&root_id){return Err("Очередь обновления источников заполнена.".into());}pending.insert(root_id);Ok(())
    }).await.map_err(|e|e.to_string())?
}
fn current(app:&tauri::AppHandle,root:&Root)->Result<(),String>{
    if read(app)?.iter().any(|r|r==root){Ok(())}else{Err("Настройки источника изменены; проход отменён.".into())}
}
fn scan(app:&tauri::AppHandle,root:&Root)->Result<(usize,usize),String>{
    current(app,root)?;let mut page=scanning::begin_scan(app,Some(root.path.clone()),true,true,root.excluded_paths.clone())?;
    let mut cursor=page.next_cursor.clone();let mut indexed=0;let mut skipped=0;let mut scan_skipped=0;
    let result=(||{
        loop{
            current(app,root)?;skipped+=page.result.skipped_entries.saturating_sub(scan_skipped);scan_skipped=page.result.skipped_entries;
            let inputs:Vec<_>=page.result.items.iter().filter(|entry|entry.kind=="image").map(|entry|IndexImageInputDto{path:entry.path.clone(),file_name:entry.name.clone(),extension:entry.extension.clone(),size_bytes:entry.size_bytes,modified_at_unix:entry.modified_at_unix,width:None,height:None}).collect();
            // Short batches yield to interactive writers and bound buffers. A vanished file cannot discard its neighbours.
            for batch in inputs.chunks(16){current(app,root)?;
                let values=batch.iter().map(|x|IndexImageInputDto{path:x.path.clone(),file_name:x.file_name.clone(),extension:x.extension.clone(),size_bytes:x.size_bytes,modified_at_unix:x.modified_at_unix,width:None,height:None}).collect();
                match sync_scanned_images_to_index_internal(app.clone(),values){Ok(result)=>indexed+=result.received_images,Err(_)=>{
                    for x in batch{current(app,root)?;let value=IndexImageInputDto{path:x.path.clone(),file_name:x.file_name.clone(),extension:x.extension.clone(),size_bytes:x.size_bytes,modified_at_unix:x.modified_at_unix,width:None,height:None};
                        match sync_scanned_images_to_index_internal(app.clone(),vec![value]){Ok(_)=>indexed+=1,Err(_)=>skipped+=1,}
                    }
                }}std::thread::sleep(Duration::from_millis(5));
            }
            match cursor.as_ref(){Some(id)=>{page=scanning::next_page(app,id)?;cursor=page.next_cursor.clone();},None=>break}
        }
        let mut after=None;loop{current(app,root)?;
            let result=rescan_index_paths_internal(app.clone(),IndexRescanPathsInputDto{root_path:Some(root.path.clone()),limit:Some(256),after_path:after,excluded_paths:Some(root.excluded_paths.clone()),only_paths:None})?;
            skipped+=result.skipped_paths;after=result.next_after_path;if after.is_none(){break;}std::thread::sleep(Duration::from_millis(5));
        }Ok((indexed,skipped))
    })();
    if let Some(id)=cursor{let _=scanning::close_directory_scan(app.clone(),id);}result
}
fn link_metadata(meta:&fs::Metadata)->bool{
    let link=meta.file_type().is_symlink();
    #[cfg(windows)]{use std::os::windows::fs::MetadataExt;return link || meta.file_attributes()&0x400!=0;}
    #[cfg(not(windows))]link
}
fn scan_changed(app:&tauri::AppHandle,root:&Root,paths:&HashSet<String>)->Result<(usize,usize),String>{
    current(app,root)?;source_paths::require_photo_source(Path::new(&root.path))?;fs::read_dir(&root.path).map_err(|e|format!("Источник недоступен: {e}"))?;
    let mut selected=Vec::new();let mut skipped=0;
    for raw in paths{current(app,root)?;let path=PathBuf::from(raw);
        if source_paths::excluded(&path) || !path_is_under_rescan_root(raw,&Some(PathBuf::from(&root.path))) || root.excluded_paths.iter().any(|excluded|path_is_under_rescan_root(raw,&Some(PathBuf::from(excluded)))){continue;}
        match fs::symlink_metadata(&path){
            Ok(meta) if meta.is_file() && !link_metadata(&meta)=>{
                let safe=path.ancestors().skip(1).take_while(|parent|normalized(&path_to_string(parent))!=normalized(&root.path)).all(|parent|fs::symlink_metadata(parent).is_ok_and(|meta|!link_metadata(&meta)));
                if !safe{skipped+=1;continue;}
                if !is_supported_image(&path){continue;}
                let value=IndexImageInputDto{path:path_to_string(&path),file_name:file_name(&path),extension:path.extension().map(|s|s.to_string_lossy().to_lowercase()),size_bytes:Some(meta.len()),modified_at_unix:modified_at_unix(&meta),width:None,height:None};
                if sync_scanned_images_to_index_internal(app.clone(),vec![value]).is_err(){skipped+=1;}
                selected.push(path_to_string(&path));
            },
            Ok(meta) if meta.is_dir()=>return scan(app,root),
            Ok(_)=>skipped+=1,
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>selected.push(path_to_string(&path)),
            Err(_)=>skipped+=1,
        }std::thread::sleep(Duration::from_millis(5));
    }
    let mut after=None;loop{current(app,root)?;
        let result=rescan_index_paths_internal(app.clone(),IndexRescanPathsInputDto{root_path:Some(root.path.clone()),limit:Some(256),after_path:after,excluded_paths:Some(root.excluded_paths.clone()),only_paths:Some(selected.clone())})?;
        skipped+=result.skipped_paths;after=result.next_after_path;if after.is_none(){break;}
    }Ok((selected.len(),skipped))
}
pub fn start(app:tauri::AppHandle){std::thread::spawn(move||{
    let dirty=std::sync::Arc::new(Mutex::new(HashMap::<String,Dirty>::new()));
    let roots_shared=std::sync::Arc::new(Mutex::new(Vec::<Root>::new()));
    let event_dirty=dirty.clone();let event_roots=roots_shared.clone();
    let mut watcher=notify::recommended_watcher(move|event:notify::Result<notify::Event>|{
        let Ok(roots)=event_roots.lock()else{return;};let Ok(mut dirty)=event_dirty.lock()else{return;};
        // Read/access notifications from our own hashing are not changes.
        if matches!(&event,Ok(e) if matches!(e.kind,notify::EventKind::Access(_))){return;}
        for root in roots.iter().filter(|r|r.auto_update){let relevant=match &event{Err(_)=>true,Ok(event)=>event.paths.iter().any(|path|!source_paths::excluded(path) && path_is_under_rescan_root(&path_to_string(path),&Some(PathBuf::from(&root.path))) && !root.excluded_paths.iter().any(|excluded|path_is_under_rescan_root(&path_to_string(path),&Some(PathBuf::from(excluded)))))};
            if relevant{let change=dirty.entry(root.id.clone()).or_insert_with(Dirty::new);change.last=Instant::now();
                match &event{Err(_)=>change.full=true,Ok(event)=>{
                    if event.need_rescan() || matches!(event.kind,notify::EventKind::Any|notify::EventKind::Other|notify::EventKind::Create(notify::event::CreateKind::Folder)|notify::EventKind::Remove(notify::event::RemoveKind::Folder)|notify::EventKind::Modify(notify::event::ModifyKind::Name(_))){change.full=true;}
                    if !change.full{for path in &event.paths{if !source_paths::excluded(path) && path_is_under_rescan_root(&path_to_string(path),&Some(PathBuf::from(&root.path))){change.paths.insert(path_to_string(path));if change.paths.len()>256{change.full=true;change.paths.clear();break;}}}}
                }}
            }
        }
    }).ok();
    let mut known:Vec<Root>=Vec::new();let mut watched=HashSet::new();let mut last=HashMap::<String,Instant>::new();let mut loaded=false;
    loop{
        std::thread::sleep(Duration::from_millis(250));
        // Opening a photo as a standalone viewer does not start archive traversal.
        if app.get_webview_window("main").is_none(){continue;}
        let runtime=app.state::<LibraryRuntime>();let refresh=runtime.refresh.swap(false,Ordering::AcqRel);
        let roots=if refresh || !loaded {match read(&app){Ok(roots)=>{loaded=true;roots},Err(_)=>{runtime.refresh.store(true,Ordering::Release);continue}}}else{known.clone()};
        if roots!=known || refresh{
            if let Ok(mut statuses)=runtime.statuses.lock(){statuses.retain(|id,_|roots.iter().any(|r|&r.id==id));}
            if let Ok(mut pending)=runtime.requested.lock(){pending.retain(|id|roots.iter().any(|r|&r.id==id));}
            last.retain(|id,_|roots.iter().any(|r|&r.id==id));
            if let Ok(mut dirty)=dirty.lock(){dirty.retain(|id,_|roots.iter().any(|r|&r.id==id));}
            if let Some(watcher)=watcher.as_mut(){for path in watched.drain(){let _=watcher.unwatch(Path::new(&path));}
                for root in roots.iter().filter(|r|r.auto_update){if watcher.watch(Path::new(&root.path),RecursiveMode::Recursive).is_ok(){watched.insert(root.path.clone());}}
            }
            if let Ok(mut shared)=roots_shared.lock(){*shared=roots.clone();}
            if let Ok(mut dirty)=dirty.lock(){for root in roots.iter().filter(|r|r.auto_update){dirty.entry(root.id.clone()).or_insert_with(Dirty::new).full=true;}}
            known=roots.clone();
        }
        for root in roots{
            let manual=runtime.requested.lock().map(|mut p|p.remove(&root.id)).unwrap_or(false);
            let control_due=root.auto_update && last.get(&root.id).is_none_or(|t|t.elapsed()>=Duration::from_secs(300));
            let event_due=root.auto_update && dirty.lock().map(|p|p.get(&root.id).is_some_and(|change|change.last.elapsed()>=Duration::from_secs(2) || change.first.elapsed()>=Duration::from_secs(10))).unwrap_or(false);
            if !manual && !control_due && !event_due{continue;}
            let change=dirty.lock().ok().and_then(|mut p|p.remove(&root.id));
            let watching=watched.contains(&root.path);if let Ok(mut statuses)=runtime.statuses.lock(){statuses.insert(root.id.clone(),RootStatus{state:"scanning".into(),watching,..Default::default()});}
            emit(&app);
            let full=manual || control_due || change.as_ref().is_none_or(|change|change.full);
            let result=if full{scan(&app,&root)}else{scan_changed(&app,&root,&change.unwrap().paths)};
            if full || result.is_err(){last.insert(root.id.clone(),if result.is_err(){Instant::now()-Duration::from_secs(290)}else{Instant::now()});}
            if result.is_ok() && root.auto_update && !watched.contains(&root.path){if let Some(watcher)=watcher.as_mut(){if watcher.watch(Path::new(&root.path),RecursiveMode::Recursive).is_ok(){watched.insert(root.path.clone());}}}
            if let Ok(mut statuses)=runtime.statuses.lock(){let status=match result{Ok((checked,skipped))=>RootStatus{state:if skipped>0{"partial"}else{"ready"}.into(),checked,skipped,updated_at_unix:Some(unix_now()),watching,error:None},Err(error)=>RootStatus{state:"unavailable".into(),error:Some(error),watching,..Default::default()}};statuses.insert(root.id,status);}
            emit(&app);
        }
    }
});}
