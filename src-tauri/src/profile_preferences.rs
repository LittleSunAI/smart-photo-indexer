//! Application-owned preferences, outside portable projects and index backups.
use crate::*;
use tauri::Emitter;
use std::sync::Mutex;
static WRITER:Mutex<()>=Mutex::new(());
#[derive(Debug,Clone,Serialize,Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct AnalysisCopy{pub enabled:bool,pub destination:String,pub folder:String}
#[derive(Debug,Clone,Serialize,Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Preferences{pub version:u32,pub revision:u64,pub cloud_consent:bool,pub recursive_processing:bool,pub analysis_copy:AnalysisCopy}
impl Default for Preferences{fn default()->Self{Self{version:1,revision:0,cloud_consent:false,recursive_processing:false,analysis_copy:AnalysisCopy{enabled:false,destination:"beside".into(),folder:String::new()}}}}
#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Patch{cloud_consent:Option<bool>,recursive_processing:Option<bool>,analysis_copy:Option<AnalysisCopy>}
fn read_file(path:&Path)->Result<Preferences,String>{if !path.exists(){return Ok(Preferences::default());}if fs::metadata(path).map_err(|e|e.to_string())?.len()>65536{return Err("Файл настроек профиля повреждён.".into());}let value:Preferences=serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|_|"Не удалось прочитать настройки профиля.")?;if value.version!=1{return Err("Версия настроек профиля не поддерживается.".into());}Ok(value)}
pub fn read(app:&tauri::AppHandle)->Result<Preferences,String>{let _guard=WRITER.lock().map_err(|_|"Profile lock failed")?;read_file(&application_data_directory(app)?.join("profile-preferences.json"))}
pub fn require_cloud(app:&tauri::AppHandle)->Result<(),String>{if !read(app)?.cloud_consent{return Err("Отправка изображений во внешний сервис отключена в настройках.".into());}Ok(())}
fn save(path:&Path,value:&Preferences)->Result<(),String>{let parent=path.parent().ok_or("Нет папки настроек")?;fs::create_dir_all(parent).map_err(|e|e.to_string())?;let temp=path.with_extension(format!("{}-{}.partial",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));use std::io::Write;let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e|e.to_string())?;file.write_all(&serde_json::to_vec(value).map_err(|e|e.to_string())?).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;drop(file);
 #[cfg(windows)]{use std::os::windows::ffi::OsStrExt;use windows_sys::Win32::Storage::FileSystem::{MoveFileExW,MOVEFILE_REPLACE_EXISTING,MOVEFILE_WRITE_THROUGH};let from:Vec<u16>=temp.as_os_str().encode_wide().chain(Some(0)).collect();let to:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();if unsafe{MoveFileExW(from.as_ptr(),to.as_ptr(),MOVEFILE_REPLACE_EXISTING|MOVEFILE_WRITE_THROUGH)}==0{let error=std::io::Error::last_os_error();let _=fs::remove_file(&temp);return Err(error.to_string());}}
 #[cfg(not(windows))]fs::rename(&temp,path).map_err(|e|e.to_string())?;Ok(())}
#[tauri::command]
pub async fn get_profile_preferences(app:tauri::AppHandle)->Result<Preferences,String>{tauri::async_runtime::spawn_blocking(move||read(&app)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn initialize_profile_preferences(app:tauri::AppHandle,window:tauri::WebviewWindow,legacy_consent:bool)->Result<Preferences,String>{if window.label()!="main"{return read(&app);}tauri::async_runtime::spawn_blocking(move||processing::cloud_fence(&app,||{let _guard=WRITER.lock().map_err(|_|"Profile lock failed")?;let path=application_data_directory(&app)?.join("profile-preferences.json");let mut value=read_file(&path)?;if value.revision==0{value.cloud_consent=legacy_consent;value.revision=1;save(&path,&value)?;let _=app.emit("smart-photo-indexer:profile-preferences-changed",&value);}Ok(value)})).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn patch_profile_preferences(app:tauri::AppHandle,expected_revision:u64,patch:Patch)->Result<Preferences,String>{let owner=app.clone();let value=tauri::async_runtime::spawn_blocking(move||processing::cloud_fence(&owner,||{
 let _guard=WRITER.lock().map_err(|_|"Profile lock failed")?;let path=application_data_directory(&owner)?.join("profile-preferences.json");let mut value=read_file(&path)?;if value.revision!=expected_revision{return Err("Настройки изменены в другом окне. Обновите состояние и повторите действие.".into());}
 if let Some(consent)=patch.cloud_consent{value.cloud_consent=consent;}if let Some(recursive)=patch.recursive_processing{value.recursive_processing=recursive;}if let Some(copy)=patch.analysis_copy{if !["beside","folder"].contains(&copy.destination.as_str())||copy.folder.len()>32768||!copy.folder.is_empty()&&!Path::new(&copy.folder).is_absolute(){return Err("Выберите абсолютный путь папки для копий.".into());}value.analysis_copy=copy;}
 value.revision=value.revision.checked_add(1).ok_or("Предел версии настроек")?;save(&path,&value)?;Ok(value)
})).await.map_err(|e|e.to_string())??;app.emit("smart-photo-indexer:profile-preferences-changed",&value).map_err(|e|e.to_string())?;Ok(value)}
