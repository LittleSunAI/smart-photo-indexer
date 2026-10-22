//! Bind a destructive file stage to an open Windows file identity.
use crate::*;
pub(crate) struct GuardedFile {file:fs::File}
impl GuardedFile{
 pub(crate) fn metadata(&self)->Result<fs::Metadata,String>{self.file.metadata().map_err(|e|e.to_string())}
 pub(crate) fn identity(&self)->Result<String,String>{
  #[cfg(windows)]{use std::os::windows::io::AsRawHandle;use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle,BY_HANDLE_FILE_INFORMATION};let mut info:BY_HANDLE_FILE_INFORMATION=unsafe{std::mem::zeroed()};if unsafe{GetFileInformationByHandle(self.file.as_raw_handle(),&mut info)}==0{return Err(std::io::Error::last_os_error().to_string());}Ok(format!("{}:{}:{}:{}:{}",info.dwVolumeSerialNumber,info.nFileIndexHigh,info.nFileIndexLow,info.ftCreationTime.dwHighDateTime,info.ftCreationTime.dwLowDateTime))}
  #[cfg(not(windows))]{Err("Идентификатор защищённого файла доступен только в Windows.".into())}
 }
 pub(crate) fn create_new(path:&Path)->Result<Self,String>{
  #[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;use windows_sys::Win32::Storage::FileSystem::{FILE_GENERIC_READ,FILE_GENERIC_WRITE,DELETE,FILE_SHARE_READ};Ok(Self{file:fs::OpenOptions::new().read(true).write(true).create_new(true).access_mode(FILE_GENERIC_READ|FILE_GENERIC_WRITE|DELETE).share_mode(FILE_SHARE_READ).open(path).map_err(|e|e.to_string())?})}
  #[cfg(not(windows))]{let _=path;Err("Защищённое создание доступно только в Windows.".into())}
 }
 pub(crate) fn copy_to(&mut self,output:&mut Self)->Result<(),String>{use std::io::{Seek,SeekFrom};self.file.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;std::io::copy(&mut self.file,&mut output.file).map_err(|e|e.to_string())?;output.file.sync_all().map_err(|e|e.to_string())}
 pub(crate) fn remove(self)->Result<(),String>{
  #[cfg(windows)]{use std::os::windows::io::AsRawHandle;use windows_sys::Win32::Storage::FileSystem::{SetFileInformationByHandle,FileDispositionInfo,FILE_DISPOSITION_INFO};let info=FILE_DISPOSITION_INFO{DeleteFile:1};if unsafe{SetFileInformationByHandle(self.file.as_raw_handle(),FileDispositionInfo,&info as *const _ as *const _,std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32)}==0{return Err(std::io::Error::last_os_error().to_string());}Ok(())}
  #[cfg(not(windows))]{Err("Защищённое удаление доступно только в Windows.".into())}
 }
 pub(crate) fn open(path:&Path,rename:bool)->Result<Self,String>{
  Self::open_with_sharing(path,rename,false)
 }
 /// Selection capture observes identity/attributes without blocking an existing writer's rename.
 pub(crate) fn observe(path:&Path)->Result<Self,String>{Self::open_with_sharing(path,false,true)}
 fn open_with_sharing(path:&Path,rename:bool,observation:bool)->Result<Self,String>{
  let metadata=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
  #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if metadata.file_attributes()&0x400!=0{return Err("Ссылки и перенаправления не участвуют в очистке копий.".into());}}
  if !metadata.is_file()||metadata.file_type().is_symlink(){return Err("Ожидается обычный файл изображения.".into());}
  #[cfg(windows)]{use std::os::windows::fs::OpenOptionsExt;use windows_sys::Win32::Storage::FileSystem::{FILE_GENERIC_READ,FILE_READ_ATTRIBUTES,DELETE,FILE_SHARE_READ,FILE_SHARE_WRITE,FILE_SHARE_DELETE};let access=if observation{FILE_READ_ATTRIBUTES}else{FILE_GENERIC_READ|if rename{DELETE}else{0}};let share=if observation{FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE}else{FILE_SHARE_READ};let file=fs::OpenOptions::new().access_mode(access).share_mode(share).open(path).map_err(|e|format!("Не удалось защитить файл от изменений: {e}"))?;Ok(Self{file})}
  #[cfg(not(windows))]{let _=(rename,observation);Err("Защищённая очистка доступна только в Windows.".into())}
 }
 pub(crate) fn sha(&mut self)->Result<String,String>{use std::io::{Read,Seek,SeekFrom};self.file.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;let mut hash=Sha256::new();let mut buffer=[0u8;64*1024];loop{let size=self.file.read(&mut buffer).map_err(|e|e.to_string())?;if size==0{break;}hash.update(&buffer[..size]);}Ok(format!("{:x}",hash.finalize()))}
 pub(crate) fn rename(&self,target:&Path)->Result<(),String>{
  #[cfg(windows)]{use std::os::windows::{ffi::OsStrExt,io::AsRawHandle};use windows_sys::Win32::Storage::FileSystem::{SetFileInformationByHandle,FileRenameInfo,FILE_RENAME_INFO};
   // Rust's file APIs support long Windows paths, but this direct Win32 call
   // does not add their extended prefix. Resolve only the existing parent:
   // the destination must stay new and ReplaceIfExists must stay false.
   let name=target.file_name().ok_or("Нет имени файла назначения")?;
   let parent=target.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
   let absolute=fs::canonicalize(parent).map_err(|e|e.to_string())?.join(name);
   // FileNameLength excludes the terminator, but Win32's path conversion also
   // requires a NUL-terminated name. Do not rely on allocator/alignment padding.
   let text:Vec<u16>=absolute.as_os_str().encode_wide().chain(std::iter::once(0)).collect();let bytes=std::mem::offset_of!(FILE_RENAME_INFO,FileName)+text.len()*2;let mut aligned=vec![0usize;bytes.div_ceil(std::mem::size_of::<usize>())];let info=aligned.as_mut_ptr() as *mut FILE_RENAME_INFO;unsafe{(*info).Anonymous.ReplaceIfExists=0;(*info).RootDirectory=std::ptr::null_mut();(*info).FileNameLength=((text.len()-1)*2) as u32;std::ptr::copy_nonoverlapping(text.as_ptr(),(*info).FileName.as_mut_ptr(),text.len());if SetFileInformationByHandle(self.file.as_raw_handle(),FileRenameInfo,info as *const _,bytes as u32)==0{return Err(format!("Не удалось переместить защищённый файл: {}",std::io::Error::last_os_error()));}}Ok(())}
  #[cfg(not(windows))]{let _=target;Err("Защищённое перемещение доступно только в Windows.".into())}
 }
}

#[cfg(all(test,windows))]mod tests{
 use super::*;
 fn fixture()->PathBuf{crate::test_support::temp_dir("smart-photo-guard")}
 #[test]fn guarded_copy_publishes_and_identity_cleanup_preserves_source(){let root=fixture();let source=root.join("original.png");let staged=root.join("prepared");let target=root.join("copy.png");fs::write(&source,b"unchanged original").unwrap();let mut original=GuardedFile::open(&source,false).unwrap();let mut copy=GuardedFile::create_new(&staged).unwrap();original.copy_to(&mut copy).unwrap();assert_eq!(copy.sha().unwrap(),original.sha().unwrap());assert!(GuardedFile::create_new(&staged).is_err());copy.rename(&target).unwrap();assert_eq!(fs::read(&source).unwrap(),b"unchanged original");copy.remove().unwrap();assert!(!target.exists());drop(original);fs::remove_file(source).unwrap();fs::remove_dir(root).unwrap();}
 #[test]fn guard_denies_external_write_and_delete_and_moves_its_own_identity(){let root=fixture();let source=root.join("source.png");let target=root.join("target.png");fs::write(&source,b"original pixels").unwrap();let mut guard=GuardedFile::open(&source,true).unwrap();assert!(fs::write(&source,b"changed").is_err());assert!(fs::remove_file(&source).is_err());let hash=guard.sha().unwrap();guard.rename(&target).unwrap();assert!(!source.exists());assert!(target.exists());assert_eq!(guard.sha().unwrap(),hash);drop(guard);assert_eq!(fs::read(&target).unwrap(),b"original pixels");fs::remove_file(target).unwrap();fs::remove_dir(root).unwrap();}
 #[test]fn handle_rename_never_overwrites_an_existing_target(){let root=fixture();let source=root.join("source.png");let target=root.join("target.png");fs::write(&source,b"source").unwrap();fs::write(&target,b"existing").unwrap();let guard=GuardedFile::open(&source,true).unwrap();assert!(guard.rename(&target).is_err());drop(guard);assert_eq!(fs::read(&target).unwrap(),b"existing");assert_eq!(fs::read(&source).unwrap(),b"source");fs::remove_file(source).unwrap();fs::remove_file(target).unwrap();fs::remove_dir(root).unwrap();}
 #[test]fn handle_rename_supports_long_unicode_paths(){
  let root=fixture();let first=root.join("a".repeat(80));let second=first.join("b".repeat(80));let parent=second.join("c".repeat(80));fs::create_dir_all(&parent).unwrap();
  let source=parent.join("исходник.png");let target=parent.join("готово.png");assert!(target.as_os_str().len()>260);fs::write(&source,b"unchanged long-path pixels").unwrap();
  let mut guard=GuardedFile::open(&source,true).unwrap();let hash=guard.sha().unwrap();guard.rename(&target).unwrap();assert_eq!(guard.sha().unwrap(),hash);assert!(!source.exists());drop(guard);assert_eq!(fs::read(&target).unwrap(),b"unchanged long-path pixels");
  fs::remove_file(target).unwrap();for directory in [parent,second,first,root]{fs::remove_dir(directory).unwrap();}
 }
 #[test]fn handle_rename_keeps_exact_names_at_every_buffer_alignment(){let root=fixture();let mut expected=Vec::new();for length in 0..8{let source=root.join("source.png");let target=root.join(format!("имя{}.png","n".repeat(length)));fs::write(&source,b"exact name").unwrap();let guard=GuardedFile::open(&source,true).unwrap();guard.rename(&target).unwrap();drop(guard);assert_eq!(fs::read(&target).unwrap(),b"exact name");expected.push(target.file_name().unwrap().to_owned());}let mut actual=fs::read_dir(&root).unwrap().map(|entry|entry.unwrap().file_name()).collect::<Vec<_>>();actual.sort();expected.sort();assert_eq!(actual,expected);for name in expected{fs::remove_file(root.join(name)).unwrap();}let _=fs::remove_dir(root);}
}
