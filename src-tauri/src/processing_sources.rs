//! Streaming folder traversal. The live explorer is never used as a job's file list.
use crate::*;
pub fn visit(roots:&[String],recursive:bool,cancel:&std::sync::atomic::AtomicBool,mut accept:impl FnMut(&Path)->Result<(),String>,mut issue:impl FnMut(&Path,&str)->Result<(),String>)->Result<(),String>{
 use std::sync::atomic::Ordering;
 for root in roots{
  if cancel.load(Ordering::SeqCst){return Err("Подготовка отменена.".into());}
  let root=Path::new(root);if source_paths::excluded(root){issue(root,"Служебный каталог Windows исключён из обработки.")?;continue;}let metadata=match fs::symlink_metadata(root){Ok(v)=>v,Err(e)=>{issue(root,&e.to_string())?;continue;}};
  if linked(&metadata){issue(root,"Ссылки и точки повторного анализа не обходятся автоматически.")?;continue;}
  if metadata.is_file(){if is_supported_image(root){accept(root)?;}continue;}
  if !metadata.is_dir(){continue;}
  let mut stack=match fs::read_dir(root){Ok(v)=>vec![v],Err(e)=>{issue(root,&e.to_string())?;continue;}};
  while let Some(directory)=stack.last_mut(){
   if cancel.load(Ordering::SeqCst){return Err("Подготовка отменена.".into());}
   let entry=match directory.next(){Some(Ok(v))=>v,Some(Err(e))=>{issue(root,&e.to_string())?;continue;},None=>{stack.pop();continue;}};
   let path=entry.path();if source_paths::excluded(&path){continue;}let metadata=match fs::symlink_metadata(&path){Ok(v)=>v,Err(e)=>{issue(&path,&e.to_string())?;continue;}};
   if linked(&metadata){continue;}
   if metadata.is_file()&&is_supported_image(&path){accept(&path)?;}
   else if recursive&&metadata.is_dir(){match fs::read_dir(&path){Ok(v)=>stack.push(v),Err(e)=>issue(&path,&e.to_string())?,}}
  }
 }
 Ok(())
}
fn linked(metadata:&fs::Metadata)->bool{
 #[cfg(windows)]{use std::os::windows::fs::MetadataExt;metadata.file_attributes()&0x400!=0}
 #[cfg(not(windows))]{metadata.file_type().is_symlink()}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn visits_every_depth_without_total_cap_and_cancels(){
  let root=std::env::temp_dir().join(format!("smart-photo-source-walk-{}",std::process::id()));fs::create_dir_all(root.join("a/b/c")).unwrap();
  for i in 0..137{fs::write(root.join(format!("{i}.png")),b"fixture").unwrap();}fs::write(root.join("a/b/c/deep.jpg"),b"fixture").unwrap();fs::write(root.join("ignore.txt"),b"fixture").unwrap();
  let cancel=std::sync::atomic::AtomicBool::new(false);let mut found=0;visit(&[path_to_string(&root)],false,&cancel,|_|{found+=1;Ok(())},|_,_|Ok(())).unwrap();assert_eq!(found,137);
  found=0;visit(&[path_to_string(&root)],true,&cancel,|_|{found+=1;Ok(())},|_,_|Ok(())).unwrap();assert_eq!(found,138);
  cancel.store(true,std::sync::atomic::Ordering::SeqCst);assert!(visit(&[path_to_string(&root)],true,&cancel,|_|panic!("cancel must prevent visitation"),|_,_|Ok(())).is_err());fs::remove_dir_all(root).unwrap();
 }
 #[test]fn skips_service_subtrees_but_keeps_hidden_and_dollar_i_user_files(){
  let root=std::env::temp_dir().join(format!("smart-photo-service-walk-{}",std::process::id()));
  for dir in ["$RECYCLE.BIN/S-1","System Volume Information",".hidden"]{fs::create_dir_all(root.join(dir)).unwrap();}
  for name in ["$RECYCLE.BIN/S-1/$I.png","System Volume Information/a.png",".hidden/photo.png","$I-user.png","bad.png"]{fs::write(root.join(name),b"fixture").unwrap();}
  let cancel=std::sync::atomic::AtomicBool::new(false);let mut paths=Vec::new();visit(&[path_to_string(&root)],true,&cancel,|p|{paths.push(path_to_string(p));Ok(())},|_,_|Ok(())).unwrap();assert_eq!(paths.len(),3);assert!(paths.iter().all(|p|!source_paths::excluded(Path::new(p))));
  let mut issues=0;visit(&[path_to_string(&root.join("$RECYCLE.BIN/S-1/$I.png"))],true,&cancel,|_|panic!("service file"),|_,_|{issues+=1;Ok(())}).unwrap();assert_eq!(issues,1);fs::remove_dir_all(root).unwrap();
 }

}
