use crate::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration,Instant};

const MAX_SCAN_PAGE_ITEMS:usize=256;
#[derive(Default)]
pub struct ScanSessions { sessions:Mutex<HashMap<String,Scan>> }
struct Frame { entries:fs::ReadDir }
struct Scan { root:PathBuf,frames:Vec<Frame>,recursive:bool,images_only:bool,read_metadata:bool,exclusions:Vec<PathBuf>,directories:usize,skipped:usize,touched:Instant }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ScanPage { #[serde(flatten)] pub(crate) result:DirectoryScanResultDto,pub(crate) next_cursor:Option<String> }

#[tauri::command]
pub async fn scan_directory(app:tauri::AppHandle,path:Option<String>,include_subfolders:bool,images_only:bool,excluded_paths:Option<Vec<String>>,read_metadata:Option<bool>)->Result<ScanPage,String>{
    tauri::async_runtime::spawn_blocking(move||begin_scan_with_metadata(&app,path,include_subfolders,images_only,excluded_paths.unwrap_or_default(),read_metadata.unwrap_or(false))).await.map_err(|e|e.to_string())?
}
pub(crate) fn begin_scan(app:&tauri::AppHandle,path:Option<String>,include_subfolders:bool,images_only:bool,excluded_paths:Vec<String>)->Result<ScanPage,String>{
    begin_scan_with_metadata(app,path,include_subfolders,images_only,excluded_paths,false)
}
fn begin_scan_with_metadata(app:&tauri::AppHandle,path:Option<String>,include_subfolders:bool,images_only:bool,excluded_paths:Vec<String>,read_metadata:bool)->Result<ScanPage,String>{
        let root=fs::canonicalize(resolve_scan_root(path)?).map_err(|e|format!("Cannot resolve folder: {e}"))?;
        source_paths::require_photo_source(&root)?;
        if !root.is_dir(){return Err("Source is not a directory.".into());}
        if excluded_paths.len()>128{return Err("Too many scan exclusions.".into());}
        let exclusions=excluded_paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        if exclusions.iter().any(|path|!path.is_absolute() || path.components().any(|c|matches!(c,std::path::Component::ParentDir)) || !path_is_under_rescan_root(&path_to_string(path),&Some(root.clone()))){return Err("Scan exclusions must be absolute paths inside the source.".into());}
        let entries=fs::read_dir(&root).map_err(|e|format!("Cannot read folder: {e}"))?;
        app.asset_protocol_scope().allow_directory(&root,true).map_err(|e|e.to_string())?;
        let nonce=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
        let id=format!("scan-{}-{nonce}",std::process::id());
        let scan=Scan{root:root.clone(),frames:vec![Frame{entries}],recursive:include_subfolders,images_only,read_metadata,exclusions,directories:1,skipped:0,touched:Instant::now()};
        let sessions=app.state::<ScanSessions>();let mut state=sessions.sessions.lock().map_err(|_|"Scan session lock failed.")?;
        state.retain(|_,s|s.touched.elapsed()<Duration::from_secs(180));
        if state.len()>=8{return Err("Too many folder scans are open; close a previous scan.".into());}
        state.insert(id.clone(),scan);page(app,&mut state,&id)
}
pub(crate) fn next_page(app:&tauri::AppHandle,id:&str)->Result<ScanPage,String>{let sessions=app.state::<ScanSessions>();let mut state=sessions.sessions.lock().map_err(|_|"Scan session lock failed.")?;page(app,&mut state,id)}
#[tauri::command]
pub async fn next_directory_scan_page(app:tauri::AppHandle,cursor:String)->Result<ScanPage,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let sessions=app.state::<ScanSessions>();let mut state=sessions.sessions.lock().map_err(|_|"Scan session lock failed.")?;page(&app,&mut state,&cursor)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub fn close_directory_scan(app:tauri::AppHandle,cursor:String)->Result<(),String>{
    app.state::<ScanSessions>().sessions.lock().map_err(|_|"Scan session lock failed.")?.remove(&cursor);Ok(())
}
fn page(app:&tauri::AppHandle,state:&mut HashMap<String,Scan>,id:&str)->Result<ScanPage,String>{
    let observation_db=index_database_path(app).ok().and_then(|path|index_backups::open_snapshot(&path).ok());
    let scan=state.get_mut(id).ok_or("Folder scan expired; open it again.")?;scan.touched=Instant::now();let mut items=Vec::new();let deadline=Instant::now()+Duration::from_millis(40);
    while items.len()<MAX_SCAN_PAGE_ITEMS && Instant::now()<deadline {
        let Some(frame)=scan.frames.last_mut() else{break;};
        let Some(entry)=frame.entries.next() else{scan.frames.pop();continue;};
        let entry=match entry{Ok(entry)=>entry,Err(_)=>{scan.skipped+=1;continue;}};
        let path=entry.path();if source_paths::excluded(&path){continue;}let meta=match entry.metadata(){Ok(meta)=>meta,Err(_)=>{scan.skipped+=1;continue;}};
        if scan.exclusions.iter().any(|excluded|path_is_under_rescan_root(&path_to_string(&path),&Some(excluded.clone()))){continue;}
        let link=meta.file_type().is_symlink();
        #[cfg(windows)]let link={use std::os::windows::fs::MetadataExt;link || meta.file_attributes()&0x400!=0};
        if link{scan.skipped+=1;continue;}
        if meta.is_dir(){
            if scan.frames.len()==1 {
                // Count is deliberately unknown here; scanning every child to fill a folder badge delays navigation.
                items.push(DirectoryScanEntryDto{source_revision:None,file_metadata:None,id:stable_id("folder",&path),kind:"folder".into(),name:file_name(&path),path:path_to_string(&path),extension:None,size_bytes:None,modified_at_unix:modified_at_unix(&meta),child_count:None,is_image:false});
            }
            if scan.recursive{
                if scan.frames.len()>=256{return Err("Folder nesting exceeds the supported depth; scan a narrower root.".into());}
                match fs::read_dir(&path){Ok(entries)=>{scan.frames.push(Frame{entries});scan.directories+=1;},Err(_)=>scan.skipped+=1}
            }
        }else if meta.is_file(){
            let image=is_supported_image(&path);if image || !scan.images_only{let mut entry=file_entry(&path,&meta,image);if image && scan.read_metadata{entry.file_metadata=serde_json::to_value(index_observations::preview_metadata(observation_db.as_ref(),&path)).ok();}items.push(entry);}
        }
    }
    items.sort_by(|a,b|(a.kind!="folder").cmp(&(b.kind!="folder")).then_with(||a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    let more=!scan.frames.is_empty();
    let result=DirectoryScanResultDto{current_path:path_to_string(&scan.root),parent_path:scan.root.parent().map(path_to_string),path_segments:path_segments(&scan.root),breadcrumbs:path_breadcrumbs(&scan.root),items,scanned_directories:scan.directories,skipped_entries:scan.skipped,truncated:false};
    if !more{state.remove(id);}
    Ok(ScanPage{result,next_cursor:more.then(||id.to_string())})
}
