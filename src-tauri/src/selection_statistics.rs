//! One replaceable background request; file metadata only, never image decoding.
use crate::*;
use serde_json::{json,Value};
use std::collections::BTreeSet;
use std::sync::{Arc,Mutex,OnceLock,atomic::{AtomicBool,Ordering}};
use std::time::{Duration,Instant};

#[derive(Default,Serialize)]
#[serde(rename_all="camelCase")]
pub(super) struct Statistics{pub files:u64,pub bytes:u64,pub unknown_sizes:u64,pub partial:bool,pub formats:BTreeSet<String>}
impl Statistics{
    pub fn add(&mut self,path:&Path,size:Option<u64>){self.files+=1;if let Some(size)=size{self.bytes=self.bytes.saturating_add(size);}else{self.unknown_sizes+=1;}if self.formats.len()<8{if let Some(ext)=path.extension().and_then(|s|s.to_str()){self.formats.insert(ext.to_uppercase());}}}
}
struct Request{app:tauri::AppHandle,paths:Vec<String>,query_id:Option<String>,excluded:Vec<String>,cancel:Arc<AtomicBool>,reply:tokio::sync::oneshot::Sender<Result<Value,String>>}
#[derive(Default)]
struct WorkState{running:bool,pending:Option<Request>,active:Option<Arc<AtomicBool>>}
fn state()->&'static Mutex<WorkState>{static STATE:OnceLock<Mutex<WorkState>>=OnceLock::new();STATE.get_or_init(||Mutex::new(WorkState::default()))}

#[tauri::command]
pub async fn get_selection_statistics(app:tauri::AppHandle,paths:Vec<String>,query_id:Option<String>,excluded_paths:Option<Vec<String>>)->Result<Value,String>{
    let (reply,receive)=tokio::sync::oneshot::channel();
    let start={
        let mut state=state().lock().map_err(|_|"Selection worker lock failed")?;
        if let Some(cancel)=&state.active{cancel.store(true,Ordering::SeqCst);}
        if let Some(old)=state.pending.take(){let _=old.reply.send(Err("Selection changed".into()));}
        if paths.is_empty()&&query_id.is_none(){return Ok(json!(Statistics::default()));}
        if paths.iter().any(|p|p.len()>32768||!Path::new(p).is_absolute()){return Err("Выделение содержит неверный путь.".into());}
        state.pending=Some(Request{app,paths,query_id,excluded:excluded_paths.unwrap_or_default(),cancel:Arc::new(AtomicBool::new(false)),reply});
        let start=!state.running;state.running=true;start
    };
    if start{tauri::async_runtime::spawn_blocking(worker);}
    receive.await.map_err(|_|"Selection worker closed".to_string())?
}
fn worker(){loop{
    let request={let Ok(mut state)=state().lock()else{return;};match state.pending.take(){Some(request)=>{state.active=Some(Arc::clone(&request.cancel));request},None=>{state.running=false;state.active=None;return;}}};
    let result=if let Some(id)=&request.query_id{search::selection_statistics(&request.app,id,&request.excluded,&request.cancel)}else{scan(&request.paths,&request.cancel)};
    let _=request.reply.send(result.and_then(|s|serde_json::to_value(s).map_err(|e|e.to_string())));
}}
fn key(path:&Path)->String{path.to_string_lossy().replace('/',"\\").trim_end_matches('\\').to_lowercase()}
fn link(metadata:&fs::Metadata)->bool{
    #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if metadata.file_attributes()&0x400!=0{return true;}}
    metadata.file_type().is_symlink()
}
fn scan(paths:&[String],cancel:&AtomicBool)->Result<Statistics,String>{
    let start=Instant::now();let mut out=Statistics::default();let mut roots=Vec::new();
    let mut paths:Vec<PathBuf>=paths.iter().map(PathBuf::from).collect();paths.sort_by_key(|p|key(p).len());
    for path in &paths{let k=key(path);if roots.iter().any(|(parent,folder):&(String,bool)|*parent==k||*folder&&k.starts_with(&(parent.clone()+"\\"))){continue;}let meta=fs::symlink_metadata(path);let folder=meta.as_ref().is_ok_and(|m|m.is_dir());roots.push((k,folder));}
    // Reconstruct original spelling from the request; key normalization is only for deduplication.
    let mut stack=Vec::new();for (k,_)in roots{if let Some(path)=paths.iter().find(|p|key(p)==k){stack.push(path.clone());}}
    let mut visited=0;
    while let Some(path)=stack.pop(){
        if cancel.load(Ordering::SeqCst){return Err("Selection changed".into());}
        if visited>=100_000||start.elapsed()>Duration::from_secs(2){out.partial=true;break;}visited+=1;
        let meta=match fs::symlink_metadata(&path){Ok(m)=>m,Err(_)=>{out.partial=true;continue;}};
        if link(&meta){out.partial=true;continue;}
        if meta.is_file(){out.add(&path,Some(meta.len()));continue;}
        if meta.is_dir(){match fs::read_dir(path){Ok(entries)=>{for entry in entries{if cancel.load(Ordering::SeqCst){return Err("Selection changed".into());}if start.elapsed()>Duration::from_secs(2){out.partial=true;break;}match entry{Ok(entry)=>{if stack.len()>=100_000{out.partial=true;break;}stack.push(entry.path());},Err(_)=>out.partial=true}}},Err(_)=>out.partial=true}}
    }Ok(out)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn selected_roots_deduplicate_nested_paths_but_keep_physical_copies_and_missing_is_partial(){
        let temporary=std::env::temp_dir();let root=temporary.join(format!("smart-photo-selection-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(root.join("nested")).unwrap();fs::write(root.join("a.png"),[1,2,3]).unwrap();fs::write(root.join("copy.png"),[1,2,3]).unwrap();fs::write(root.join("nested/b.txt"),[1,2,3,4,5]).unwrap();
        let paths=vec![path_to_string(&root.join("a.png")),path_to_string(&root),path_to_string(&root.join("nested"))];
        let s=scan(&paths,&AtomicBool::new(false)).unwrap();assert_eq!((s.files,s.bytes,s.partial),(3,11,false));assert_eq!(s.formats.len(),2);
        let missing=scan(&[path_to_string(&root.join("missing"))],&AtomicBool::new(false)).unwrap();assert_eq!(missing.files,0);assert!(missing.partial);
        assert!(scan(&paths,&AtomicBool::new(true)).is_err());
        assert!(fs::canonicalize(&root).unwrap().starts_with(fs::canonicalize(&temporary).unwrap()));fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn junction_is_not_traversed(){
        let temporary=std::env::temp_dir();let root=temporary.join(format!("smart-photo-junction-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));let target=root.join("target");let link_path=root.join("link");fs::create_dir_all(&target).unwrap();fs::write(target.join("photo.png"),[1,2,3]).unwrap();
        let output=std::process::Command::new("cmd.exe").args(["/c","mklink","/J"]).arg(&link_path).arg(&target).output().unwrap();assert!(output.status.success());let s=scan(&[path_to_string(&link_path)],&AtomicBool::new(false)).unwrap();assert_eq!(s.files,0);assert!(s.partial);assert!(target.join("photo.png").exists());fs::remove_dir(&link_path).unwrap();assert!(fs::canonicalize(&root).unwrap().starts_with(fs::canonicalize(&temporary).unwrap()));fs::remove_dir_all(root).unwrap();
    }
}
