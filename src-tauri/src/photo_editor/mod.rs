//! Separate native-owned editor sessions. Archive/viewer state is never replaced.
mod spatial;
mod lifecycle;
mod cleanup;
pub mod contracts;
pub mod store;
pub mod migration;
pub mod geometry;
pub mod local;
pub mod outpaint;
pub mod masks;
pub mod library;
pub mod catalog;
pub mod estimates;
pub mod pricing;
pub mod adapter_parameters;
pub mod instructions;
pub mod references;
pub mod storage;
pub mod jobs;
mod result_download;
pub mod packages;
pub mod source_url;
pub mod replacement;
use crate::*;
use serde_json::{json,Value};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter,WebviewUrl,WebviewWindowBuilder};

#[derive(Clone)]
struct Session {project_id:String,origin_label:String,epoch:String,hydrated:bool,requested_step:Option<String>}
#[derive(Default)]
pub struct EditorSessions {sessions:Mutex<HashMap<String,Session>>,opening:tokio::sync::Mutex<()>}
impl EditorSessions{pub(super) fn has_open_sessions(&self)->Result<bool,String>{Ok(!self.sessions.lock().map_err(|_|"Editor sessions lock failed")?.is_empty())}}
fn session(app:&tauri::AppHandle,window:&tauri::WebviewWindow,epoch:Option<&str>)->Result<Session,String>{
    let state=app.state::<EditorSessions>();let sessions=state.sessions.lock().map_err(|_|"Editor session lock failed.")?;
    let s=sessions.get(window.label()).cloned().ok_or("Этому окну не принадлежит сессия редактора.")?;
    if epoch.is_some_and(|e|e!=s.epoch){return Err("Сессия редактора уже сменилась.".into());}Ok(s)
}
fn payload(app:&tauri::AppHandle,label:&str,s:&Session)->Result<Value,String>{
    let mut value=store::project_snapshot(app,&s.project_id)?;
    value["version"]=json!(3);value["label"]=json!(label);value["epoch"]=json!(s.epoch);
    value["requestedStepId"]=json!(s.requested_step);value["originKind"]=json!(if s.origin_label.starts_with("smart-photo-viewer-"){"viewer"}else{"archive"});Ok(value)
}
#[tauri::command]
pub async fn open_photo_editor(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,source_path:String)->Result<String,String>{
    let runtime=app_handle.state::<EditorSessions>();let _opening=runtime.opening.lock().await;
    let origin_label=window.label().to_string();
    if origin_label!="main"&&!origin_label.starts_with("smart-photo-viewer-"){return Err("Открывайте редактор из архива или окна просмотра.".into());}
    let path=validate_existing_image_file(&source_path)?.canonicalize().map_err(|e|e.to_string())?;
    if origin_label!="main"&&!viewer_sessions::owns_active_path(&app_handle,&origin_label,&path)?{return Err("Выбранный файл уже не является активным кадром просмотрщика.".into());}
    let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
    let app=app_handle.clone();
    let project_id=tauri::async_runtime::spawn_blocking(move||{let _permit=permit;store::open_source(&app,&path)}).await.map_err(|e|e.to_string())??;
    open_project_window(&app_handle,project_id,origin_label,None)
}
#[tauri::command]
pub async fn open_help_working_window(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,context:String,source_path:String)->Result<String,String>{
 if window.label()!="main"{return Err("Переход к другому рабочему окну доступен из главной справки.".into());}
 let path=validate_existing_image_file(&source_path)?.canonicalize().map_err(|e|e.to_string())?;
 if context=="viewer"{let app=app_handle.clone();let target=tauri::async_runtime::spawn_blocking(move||viewer_sessions::help_target(&app,&path)).await.map_err(|e|e.to_string())??;if let Some(label)=target{if let Some(window)=app_handle.get_webview_window(&label){crate::window_appearance::present(&window)?;return Ok(label);}}let app=app_handle.clone();return tauri::async_runtime::spawn_blocking(move||viewer_sessions::open(&app,vec![source_path.clone()],source_path,false)).await.map_err(|e|e.to_string())?;}
 if context!="editor"{return Err("Неизвестное рабочее окно помощи.".into());}
 let runtime=app_handle.state::<EditorSessions>();let _opening=runtime.opening.lock().await;
 let app=app_handle.clone();let existing_path=path.clone();let candidates=tauri::async_runtime::spawn_blocking(move||{let db=store::connection(&app)?;let mut q=db.prepare("SELECT id,source_path FROM editor_projects WHERE id NOT IN(SELECT project_id FROM editor_project_tombstones)").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;let mut ids=Vec::new();for row in rows{let(id,text)=row.map_err(|e|e.to_string())?;if Path::new(&text).canonicalize().ok().as_deref()==Some(existing_path.as_path()){ids.push(id);}}Ok::<_,String>(ids)}).await.map_err(|e|e.to_string())??;
 let target={let sessions=runtime.sessions.lock().map_err(|_|"Session lock failed")?;let mut labels:Vec<_>=sessions.iter().filter(|(label,s)|candidates.contains(&s.project_id)&&app_handle.get_webview_window(label).is_some()).map(|(label,_)|label.clone()).collect();labels.sort();labels.into_iter().next()};
 if let Some(label)=target{if let Some(window)=app_handle.get_webview_window(&label){crate::window_appearance::present(&window)?;return Ok(label);}}
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;let app=app_handle.clone();let project_id=tauri::async_runtime::spawn_blocking(move||{let _permit=permit;store::open_source(&app,&path)}).await.map_err(|e|e.to_string())??;open_project_window(&app_handle,project_id,window.label().into(),None)
}
fn open_project_window(app_handle:&tauri::AppHandle,project_id:String,origin_label:String,requested_step:Option<String>)->Result<String,String>{
 let runtime=app_handle.state::<EditorSessions>();
    let label={let sessions=runtime.sessions.lock().map_err(|_|"Session lock failed")?;if let Some((label,_))=sessions.iter().find(|(_,s)|s.project_id==project_id){label.clone()}else{let base=format!("smart-photo-editor-{}",project_id.trim_start_matches("editor-project:"));if sessions.contains_key(&base){format!("{base}-{}",unix_now())}else{base}}};
    let epoch=stable_text_id("editor-session",&format!("{label}|{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    let s={let state=app_handle.state::<EditorSessions>();let mut sessions=state.sessions.lock().map_err(|_|"Editor session lock failed.")?;
        if let Some(existing)=sessions.get_mut(&label){existing.origin_label=origin_label;if requested_step.is_some(){existing.requested_step=requested_step;}existing.clone()}else{let s=Session{project_id,origin_label,epoch,hydrated:false,requested_step};sessions.insert(label.clone(),s.clone());s}};
    if let Some(existing)=app_handle.get_webview_window(&label){if let Some(step)=&s.requested_step{existing.emit("smart-photo-indexer:editor-open-result",json!({"stepId":step})).map_err(|e|e.to_string())?;}crate::window_appearance::present(&existing)?;existing.emit("smart-photo-indexer:editor-origin",json!({"originKind":if s.origin_label.starts_with("smart-photo-viewer-"){"viewer"}else{"archive"}})).map_err(|e|e.to_string())?;return Ok(label);}
    let created=WebviewWindowBuilder::new(app_handle,&label,WebviewUrl::App("index.html?window=editor".into())).title("СМАРТ ФОТО · Редактор").inner_size(1280.0,860.0).min_inner_size(500.0,420.0).resizable(true).decorations(true).maximized(true).theme(Some(crate::window_appearance::theme(app_handle))).visible(false).prevent_overflow().center().build();
    if let Err(e)=created{remove(&app_handle,&label);return Err(format!("Не удалось открыть редактор: {e}"));}Ok(label)
}
#[tauri::command]
pub async fn get_editor_session(app_handle:tauri::AppHandle,window:tauri::WebviewWindow)->Result<Value,String>{
    let s=session(&app_handle,&window,None)?;let label=window.label().to_string();let app=app_handle.clone();let record=s.clone();let target=label.clone();
    let value=tauri::async_runtime::spawn_blocking(move||payload(&app,&target,&record)).await.map_err(|e|e.to_string())??;
    if let Some(record)=app_handle.state::<EditorSessions>().sessions.lock().map_err(|_|"Editor session lock failed.")?.get_mut(&label){if record.epoch==s.epoch{record.hydrated=true;}}
    Ok(value)
}
#[tauri::command]
pub async fn save_editor_draft(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,expected_revision:u64,draft:contracts::Draft)->Result<Value,String>{
    let s=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;store::save_draft(&app_handle,&s.project_id,expected_revision,draft)}).await.map_err(|e|e.to_string())?
}
fn focus_origin(app:&tauri::AppHandle,s:&Session,archive:bool)->Result<(),String>{
    if crate::window_appearance::hidden_test_windows(){return Ok(());}
    if !archive{if let Some(origin)=app.get_webview_window(&s.origin_label){origin.unminimize().map_err(|e|e.to_string())?;origin.show().map_err(|e|e.to_string())?;origin.set_focus().map_err(|e|e.to_string())?;return Ok(());}}
    if archive{viewer_sessions::show_main(app)}else{Ok(())}
}
#[tauri::command]
pub async fn apply_editor_local_operation(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,expected_revision:u64,draft:contracts::Draft,operation:local::LocalOperation)->Result<Value,String>{
    let s=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{let _permit=permit;local::apply(&app_handle,&s.project_id,expected_revision,draft,operation)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn save_editor_mask(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,expected_revision:u64,draft:contracts::Draft,annotation_id:String,strokes:Vec<masks::Stroke>)->Result<Value,String>{
    let s=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{let _permit=permit;masks::save(&app_handle,&s.project_id,expected_revision,draft,annotation_id,strokes)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn save_editor_checkpoint(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,revision:u64)->Result<Value,String>{let s=session(&app_handle,&window,Some(&epoch))?;tauri::async_runtime::spawn_blocking(move||lifecycle::checkpoint(&app_handle,&s.project_id,revision)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn get_editor_lifecycle(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String)->Result<Value,String>{let s=session(&app_handle,&window,Some(&epoch))?;tauri::async_runtime::spawn_blocking(move||{let mut db=store::connection(&app_handle)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let value=lifecycle::observe(&tx,&s.project_id)?;tx.commit().map_err(|e|e.to_string())?;Ok(value)}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn finish_editor_navigation(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>,revision:u64,action:String,discard_unsaved:Option<bool>)->Result<(),String>{
    let s=session(&app_handle,&window,epoch.as_deref())?;
    if !["return","archive","close"].contains(&action.as_str()){return Err("Неизвестное действие редактора.".into());}
    if action=="archive"{focus_origin(&app_handle,&s,true)?;return Ok(());}
    let app=app_handle.clone();let project=s.project_id.clone();let label=window.label().to_string();let closing_epoch=s.epoch.clone();
    tauri::async_runtime::spawn_blocking(move||processing::cloud_fence(&app,||{
      let state=app.state::<EditorSessions>();let mut sessions=state.sessions.lock().map_err(|_|"Session lock failed")?;
      if sessions.get(&label).is_none_or(|current|current.epoch!=closing_epoch){return Err("Сессия редактора уже сменилась.".into());}
      let mut db=store::connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
      let current:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[&project],|r|r.get(0)).map_err(|e|e.to_string())?;
      if current!=revision{return Err("Проект изменился перед выходом. Проверьте его ещё раз.".into());}
      if discard_unsaved.unwrap_or(false){lifecycle::discard(&tx,&project)?;}else{let value=lifecycle::observe(&tx,&project)?;if value["dirty"]==true{return Err("Сохраните проект или подтвердите выход без новых изменений.".into());}}
      tx.execute("UPDATE editor_job_activation SET state='revoked' WHERE project_id=?1 AND state='eligible'",[&project]).map_err(|e|e.to_string())?;
      tx.commit().map_err(|e|e.to_string())?;
      // Revoke the epoch before releasing admission's fence. Destroyed removes the record.
      if let Some(current)=sessions.get_mut(&label){current.epoch=format!("closed-{}",current.epoch);}
      Ok(())
    })).await.map_err(|e|e.to_string())??;
    if action=="return"{focus_origin(&app_handle,&s,false)?;}
    if let Err(e)=window.destroy(){if let Ok(mut sessions)=app_handle.state::<EditorSessions>().sessions.lock(){if let Some(record)=sessions.get_mut(window.label()){record.epoch=s.epoch;}}return Err(e.to_string());}Ok(())
}
pub fn close_requested(window:&tauri::Window,api:&tauri::CloseRequestApi){
    if window.label().starts_with("smart-photo-editor-"){
        let s=window.app_handle().state::<EditorSessions>().sessions.lock().ok().and_then(|map|map.get(window.label()).cloned());
        if let Some(s)=s{if !s.hydrated{return;}}
        api.prevent_close();let _=window.emit("smart-photo-indexer:editor-close-requested",json!({}));
    }
}
pub fn remove(app:&tauri::AppHandle,label:&str){if let Ok(mut sessions)=app.state::<EditorSessions>().sessions.lock(){sessions.remove(label);}}
#[tauri::command]
pub async fn editor_library_command(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,action:String,expected_revision:Option<u64>,id:Option<String>,text:Option<String>,discard_changes:Option<bool>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;let app=app_handle.clone();let label=window.label().to_string();
 let permit=if action=="gc"{Some(file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?)}else{None};
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;match action.as_str(){
  "listProjects"=>library::list(&app),"projectPreview"=>library::preview_project(&app,id.as_deref().ok_or("Project ID required")?,text.as_deref().unwrap_or("0").parse::<usize>().map_err(|_|"Invalid history offset")?),
  "deleteListedProject"=>{let target=id.as_deref().ok_or("Не указан проект")?;if target==s.project_id{return Err("Удалите текущий проект через его собственную команду.".into());}processing::cloud_fence(&app,||{let state=app.state::<EditorSessions>();let sessions=state.sessions.lock().map_err(|_|"Session lock failed")?;if sessions.values().any(|record|record.project_id==target){return Err("Закройте этот проект в его редакторе перед удалением.".into());}library::delete_project_locked(&app,target)})},"select"=>{let result=library::select(&app,&s.project_id,expected_revision.ok_or("Не указана revision")?,id.as_deref().ok_or("Не указан кадр")?)?;if let Some(record)=app.state::<EditorSessions>().sessions.lock().map_err(|_|"Session lock failed")?.get_mut(&label){record.requested_step=None;}Ok(result)},
  "rename"=>library::rename(&app,&s.project_id,text.as_deref().ok_or("Не указано название")?),"restoreStep"=>library::restore_step(&app,&s.project_id,id.as_deref().ok_or("Не указан кадр")?),"deleteStep"=>library::delete_step(&app,&s.project_id,id.as_deref().ok_or("Не указан кадр")?),
  "deleteSteps"=>library::delete_steps(&app,&s.project_id,expected_revision.ok_or("Не указана revision")?,&serde_json::from_str::<Vec<String>>(text.as_deref().ok_or("Не выбраны кадры")?).map_err(|_|"Неверный список кадров")?),
  "selectAsset"=>library::select_asset(&app,&s.project_id,expected_revision.ok_or("Не указана revision")?,id.as_deref().ok_or("Не указан asset")?),"clear"=>library::clear(&app,&s.project_id,expected_revision.ok_or("Не указана revision")?),"deleteProject"=>{let result=library::delete_project(&app,&s.project_id)?;if let Some(record)=app.state::<EditorSessions>().sessions.lock().map_err(|_|"Session lock failed")?.get_mut(&label){record.hydrated=false;}Ok(result)},"gc"=>library::gc(&app),
  "duplicate"|"openProject"|"newProject"=>{
   let state=app.state::<EditorSessions>();
   let mut sessions=state.sessions.lock().map_err(|_|"Session lock failed")?;
   if sessions.get(&label).is_none_or(|current|current.epoch!=s.epoch){return Err("Сессия редактора сменилась. Повторите действие в текущем проекте.".into());}
   if action=="openProject"{store::project_snapshot(&app,id.as_deref().ok_or("Не указан проект")?)?;}
   let mut db=store::connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
   let current:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[&s.project_id],|r|r.get(0)).map_err(|e|e.to_string())?;if Some(current)!=expected_revision{return Err("Проект изменился перед переходом.".into());}
   let value=lifecycle::observe(&tx,&s.project_id)?;let abandoned=value["dirty"]==true&&value["retained"]!=true;
   if value["dirty"]==true{if discard_changes!=Some(true){return Err("Сохраните проект или подтвердите переход без новых изменений.".into());}lifecycle::discard(&tx,&s.project_id)?;}
   let project=if action=="openProject"{id.ok_or("Не указан проект")?}else{let revision:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[&s.project_id],|r|r.get(0)).map_err(|e|e.to_string())?;if action=="newProject"||abandoned{library::start_again_in_transaction(&tx,&s.project_id,revision)?}else{let snapshot=lifecycle::capture(&tx,&s.project_id)?;let mut history=portable_history::export(&app,&s.project_id)?;let jobs=snapshot["steps"].as_array().unwrap().iter().filter_map(|s|s["jobId"].as_str()).collect::<std::collections::HashSet<_>>();history.retain(|r|r["version"]==2||jobs.contains(r["job"]["id"].as_str().unwrap_or("")));library::duplicate_in_transaction(&tx,&s.project_id,&snapshot,&history)?}};
   if project!=s.project_id{tx.execute("UPDATE editor_job_activation SET state='revoked' WHERE project_id=?1 AND state='eligible'",[&s.project_id]).map_err(|e|e.to_string())?;}
   lifecycle::observe(&tx,&project)?;tx.commit().map_err(|e|e.to_string())?;
   let next_epoch=stable_text_id("editor-session",&format!("{project}|{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
   if project!=s.project_id{library::revoke_result_activation(&app,&s.project_id)?;}
   let record=Session{project_id:project,epoch:next_epoch,origin_label:s.origin_label,hydrated:true,requested_step:None};
   sessions.insert(label.clone(),record.clone());drop(sessions);
   payload(&app,&label,&record)
  },
  _=>Err("Неизвестная команда библиотеки.".into())
 }}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn import_editor_file(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,path:Option<String>, ui_locale:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;
 let path=if let Some(path)=path{PathBuf::from(path)}else{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.87ad2302bae459ac")).add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.0a6258a5f30f27c8"),&["png","jpg","jpeg","webp","bmp","tif","tiff"]).pick_file().await{Some(file)=>file.path().to_path_buf(),None=>return Ok(Value::Null)}};
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;library::import_file(&app_handle,&s.project_id,&path)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn export_editor_image(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,asset_id:String,working_turn:u8,mirrored:Option<bool>,format:String,quality:u8,target:Option<String>,use_default:Option<bool>, ui_locale:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;
 let target=if target.is_none()&&use_default==Some(true){storage::output_target(&app_handle,&s.project_id,&format)? .map(|p|path_to_string(&p))}else{target};
 let target=if let Some(path)=target{PathBuf::from(path)}else{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.6f6b1ea32fcb36fd")).set_file_name(format!("edited.{}",if format=="jpeg"{"jpg"}else{&format})).save_file().await{Some(file)=>file.path().to_path_buf(),None=>return Ok(Value::Null)}};
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;library::export_image(&app_handle,&s.project_id,&asset_id,working_turn,mirrored.unwrap_or(false),&target,&format,quality)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn preview_editor_request(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,draft:contracts::Draft)->Result<Value,String>{let s=session(&app_handle,&window,Some(&epoch))?;tauri::async_runtime::spawn_blocking(move||if draft.form_values.get("__maskStrokes").and_then(Value::as_array).is_some_and(|s|!s.is_empty()){draft.validate()?;Ok(json!({"kind":"needsMaskPreparation"}))}else{catalog::plan(&app_handle,&s.project_id,&draft)}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn start_editor_job(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,id:String,expected_revision:u64,draft:contracts::Draft,credential_ref:String,consent:bool,budget_micros:u64,concurrency:u32,source_step_id:Option<String>,expected_plan_hash:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;let label=window.label().to_string();
 tauri::async_runtime::spawn_blocking(move||{
  processing::cloud_fence(&app_handle,||{
   let state=app_handle.state::<EditorSessions>();let sessions=state.sessions.lock().map_err(|_|"Session lock failed")?;
   if sessions.get(&label).is_none_or(|current|current.epoch!=s.epoch){return Err("Сессия редактора сменилась. Повторите действие в текущем проекте.".into());}
   jobs::create(&app_handle,&s.project_id,&id,expected_revision,draft,&credential_ref,consent,budget_micros,concurrency,source_step_id.as_deref(),expected_plan_hash.as_deref())
  })
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn get_editor_jobs(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>)->Result<Value,String>{let project=if window.label()=="main"{None}else{Some(session(&app_handle,&window,epoch.as_deref())?.project_id)};tauri::async_runtime::spawn_blocking(move||jobs::list(&app_handle,project.as_deref())).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn get_editor_workbench(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,project_id:Option<String>)->Result<Value,String>{
 if window.label()!="main"{session(&app_handle,&window,None)?;}
 tauri::async_runtime::spawn_blocking(move||{
  if let Some(id)=project_id{return library::preview_latest(&app_handle,&id);}
  Ok(json!({"projects":library::list(&app_handle)?,"jobs":jobs::list(&app_handle,None)?,"publications":publication::list(&app_handle)?,"storage":cleanup::summary(&app_handle)?}))
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn preview_project_cleanup(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,scope:cleanup::Scope)->Result<Value,String>{if window.label()!="main"{session(&app_handle,&window,None)?;}tauri::async_runtime::spawn_blocking(move||cleanup::preview(&app_handle,scope)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn confirm_project_cleanup(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,token:String)->Result<Value,String>{if window.label()!="main"{session(&app_handle,&window,None)?;}tauri::async_runtime::spawn_blocking(move||cleanup::confirm(&app_handle,&token)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn probe_editor_model(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,draft:contracts::Draft)->Result<Value,String>{let s=session(&app_handle,&window,Some(&epoch))?;tauri::async_runtime::spawn_blocking(move||catalog::model_probe(&app_handle,&s.project_id,&draft)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn editor_job_command(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>,id:String,action:String)->Result<Value,String>{let project=if window.label()=="main"{None}else{Some(session(&app_handle,&window,epoch.as_deref())?.project_id)};tauri::async_runtime::spawn_blocking(move||jobs::command(&app_handle,project.as_deref(),&id,&action)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn reconcile_editor_request(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>,id:String,request_id:String,confirmed:bool)->Result<Value,String>{let project=if window.label()=="main"{None}else{Some(session(&app_handle,&window,epoch.as_deref())?.project_id)};tauri::async_runtime::spawn_blocking(move||jobs::reconcile_request(&app_handle,project.as_deref(),&id,&request_id,confirmed)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn editor_project_package(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,import:bool,path:Option<String>, ui_locale:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;
 let path=if let Some(path)=path{PathBuf::from(path)}else if import{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.845af32efae7db1f")).add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.38e1f3ca576770fd"),&["json"]).pick_file().await{Some(f)=>f.path().to_path_buf(),None=>return Ok(Value::Null)}}else{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.7d4dccbe83ec14a2")).pick_folder().await{Some(f)=>f.path().to_path_buf(),None=>return Ok(Value::Null)}};
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;if import{let project=packages::import(&app_handle,&path)?;Ok(json!({"projectId":project}))}else{packages::export(&app_handle,&s.project_id,&path)}}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn import_editor_url(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,url:String)->Result<Value,String>{let s=session(&app_handle,&window,Some(&epoch))?;source_url::import(app_handle,s.project_id,url).await}

pub mod publication;
pub mod portable_history;
mod finish;
mod results;
#[tauri::command]
pub async fn publish_editor_copy(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,id:String,asset_id:String,working_turn:u8,mirrored:Option<bool>,format:String,quality:u8,target:Option<String>, ui_locale:Option<String>)->Result<Value,String>{
 let record=session(&app_handle,&window,Some(&epoch))?;let target=if let Some(target)=target{PathBuf::from(target)}else{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.58090fda80513b25")).set_file_name(format!("edited.{}",if format=="jpeg"{"jpg"}else{&format})).save_file().await{Some(file)=>file.path().to_path_buf(),None=>return Ok(Value::Null)}};let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;publication::create(&app_handle,&record.project_id,&id,&asset_id,working_turn,mirrored.unwrap_or(false),&target,&format,quality)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn get_editor_publications(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>)->Result<Value,String>{let owner=if window.label()=="main"{None}else{Some(session(&app_handle,&window,epoch.as_deref())?.project_id)};tauri::async_runtime::spawn_blocking(move||{let mut value=publication::list(&app_handle)?;if let Some(owner)=owner{value.as_array_mut().unwrap().retain(|row|row["projectId"]==owner);}Ok(value)}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn retry_editor_publication(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>,id:String)->Result<Value,String>{if window.label()!="main"{let owner=session(&app_handle,&window,epoch.as_deref())?.project_id;let belongs:bool=store::connection(&app_handle)?.query_row("SELECT project_id=?2 FROM editor_publications WHERE id=?1",params![id,owner],|r|r.get(0)).map_err(|e|e.to_string())?;if !belongs{return Err("Публикация принадлежит другому проекту.".into());}}let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;publication::reconcile(&app_handle,&id)}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn cancel_editor_publication(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:Option<String>,id:String)->Result<Value,String>{if window.label()!="main"{let owner=session(&app_handle,&window,epoch.as_deref())?.project_id;let belongs:bool=store::connection(&app_handle)?.query_row("SELECT project_id=?2 FROM editor_publications WHERE id=?1",params![id,owner],|r|r.get(0)).map_err(|e|e.to_string())?;if !belongs{return Err("Публикация принадлежит другому проекту.".into());}}let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;publication::cancel(&app_handle,&id)}).await.map_err(|e|e.to_string())?}

#[tauri::command]
pub async fn open_editor_project(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,project_id:String,asset_id:Option<String>,step_id:Option<String>)->Result<String,String>{if window.label()!="main"{return Err("Открывайте задачу проекта из архива.".into());}let runtime=app_handle.state::<EditorSessions>();let _opening=runtime.opening.lock().await;let app=app_handle.clone();let project=project_id.clone();let snapshot=tauri::async_runtime::spawn_blocking(move||store::project_snapshot(&app,&project)).await.map_err(|e|e.to_string())??;let step=if let Some(id)=step_id{if !snapshot["steps"].as_array().is_some_and(|steps|steps.iter().any(|s|s["id"]==id)){return Err("Кадр отсутствует в этом проекте".into());}Some(id)}else if let Some(asset)=asset_id{let matches=snapshot["steps"].as_array().ok_or("Нет готовых кадров")?.iter().filter(|s|s["assetId"]==asset&&!s["hidden"].as_bool().unwrap_or(false)).collect::<Vec<_>>();if matches.len()>1{return Err("К одному изображению относятся несколько кадров. Откройте нужный результат из карточки задачи.".into());}Some(matches.first().and_then(|s|s["id"].as_str()).ok_or("Готовый результат не принадлежит этому проекту")?.to_string())}else{None};open_project_window(&app_handle,project_id,"main".into(),step)}

pub mod review;
#[tauri::command]
pub async fn get_editor_catalog(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String)->Result<Value,String>{session(&app_handle,&window,Some(&epoch))?;tauri::async_runtime::spawn_blocking(move||review::active(&app_handle)).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn editor_catalog_command(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,action:String,value:Option<Value>)->Result<Value,String>{let owner=session(&app_handle,&window,Some(&epoch))?.project_id;tauri::async_runtime::spawn_blocking(move||match action.as_str(){"preview"=>review::preview(&app_handle,&value.ok_or("Нет предложения")?),"apply"=>review::apply(&app_handle,&value.ok_or("Нет предложения")?),"versions"=>review::versions(&app_handle),"rollback"=>review::rollback(&app_handle,value.as_ref().and_then(Value::as_str).ok_or("Нет ID версии")?),"notes"=>review::notes(&app_handle,&owner),"saveNote"=>{let v=value.ok_or("Нет заметки")?;review::save_note(&app_handle,&owner,v["modelId"].as_str(),v["note"].as_str().ok_or("Нет текста заметки")?)},_=>Err("Неизвестная команда рекомендаций".into())}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn check_editor_catalog_sources(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String)->Result<Value,String>{session(&app_handle,&window,Some(&epoch))?;let catalog=review::active(&app_handle)?;let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(30)).build().map_err(|e|e.to_string())?;let mut checks=Vec::new();for route in catalog["routes"].as_array().ok_or("Нет маршрутов")?{let url=route["schemaUrl"].as_str().ok_or("Нет URL схемы")?;let trusted=catalog::route(route["id"].as_str().ok_or("Нет ID")?)?;if route["schemaUrl"]!=trusted["schemaUrl"]{return Err("Адрес схемы не совпадает с проверенным источником.".into());}let response=client.get(url).send().await.map_err(|_|"Не удалось проверить официальный каталог Fal")?;if !response.status().is_success(){return Err("Fal временно не вернул схему; прежние рекомендации сохранены.".into());}let schema=provider_transport::bounded_json(response,8*1024*1024).await?;let input=&schema["paths"][format!("/{}",route["id"].as_str().unwrap())]["post"]["requestBody"]["content"]["application/json"]["schema"];let name=input["$ref"].as_str().and_then(|s|s.strip_prefix("#/components/schemas/"));let current=name.map(|name|&schema["components"]["schemas"][name]).unwrap_or(input);checks.push(json!({"modelId":route["id"],"contractMatches":current==&route["inputSchema"],"checkedAtUnix":unix_now(),"priceCheck":"Тариф не меняется автоматически; сверяйте страницу модели"}));}Ok(json!(checks))}

pub mod backup;
#[tauri::command]
pub async fn editor_full_backup(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,action:String,path:Option<String>, ui_locale:Option<String>)->Result<Value,String>{if window.label()!="main"{return Err("Резервная копия всего приложения доступна в архиве.".into());}if !["export","inspect","stage"].contains(&action.as_str()){return Err("Неизвестная команда резервной копии.".into());}let path=if let Some(path)=path{PathBuf::from(path)}else if action=="export"{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.aff6d6cbb33c32f8")).pick_folder().await{Some(file)=>file.path().to_path_buf(),None=>return Ok(Value::Null)}}else{match rfd::AsyncFileDialog::new().set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.a53454afbe92ab78")).add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.6dd7b5fe2f9bd72a"),&["json"]).pick_file().await{Some(file)=>file.path().to_path_buf(),None=>return Ok(Value::Null)}};let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;match action.as_str(){"export"=>backup::export(&app_handle,&path),"inspect"=>{let mut info=backup::inspect(&path)?;info["path"]=json!(path_to_string(&path));Ok(info)},_=>backup::stage(&app_handle,&path)}}).await.map_err(|e|e.to_string())?}

#[tauri::command]
pub async fn attach_editor_references(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,expected_revision:u64,annotation_id:String,replace:bool,paths:Option<Vec<String>>,asset_ids:Option<Vec<String>>,step_id:Option<String>, ui_locale:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;
 if let Some(step)=step_id {
  if paths.is_some()||asset_ids.is_some(){return Err("Выберите один источник дополнительной картинки.".into());}
  let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
  return tauri::async_runtime::spawn_blocking(move||{let _permit=permit;references::attach_step(&app_handle,&s.project_id,expected_revision,&annotation_id,replace,&step,||{session(&app_handle,&window,Some(&epoch))?;Ok(())})}).await.map_err(|e|e.to_string())?;
 }
 if let Some(ids)=asset_ids.as_ref(){let ids=ids.clone();let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;return tauri::async_runtime::spawn_blocking(move||{let _permit=permit;references::attach_existing(&app_handle,&s.project_id,expected_revision,&annotation_id,replace,&ids,||{session(&app_handle,&window,Some(&epoch))?;Ok(())})}).await.map_err(|e|e.to_string())?;}
 {let preflight=references::claim(&app_handle,&s.project_id,expected_revision,&annotation_id,replace,paths.as_ref().map_or_else(||asset_ids.as_ref().map_or(1,Vec::len),Vec::len))?;drop(preflight);}
 let paths=if let Some(ids)=asset_ids{ids.iter().map(|id|{let a=local::project_asset(&app_handle,&s.project_id,id)?;store::owned_path(&app_handle,a["relativePath"].as_str().ok_or("Нет пути дополнительной картинки")?)}).collect::<Result<Vec<_>,String>>()?}else if let Some(paths)=paths{paths.into_iter().map(PathBuf::from).collect::<Vec<_>>()}else{match rfd::AsyncFileDialog::new().set_title(if annotation_id=="image"{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.6b01898c0c37e799")}else{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.d4e40b0715c7cb59")}).add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.0a6258a5f30f27c8"),&["jpg","jpeg","png","webp","bmp","tiff","tif"]).pick_files().await{Some(files)=>files.into_iter().map(|f|f.path().to_path_buf()).collect(),None=>return Ok(Value::Null)}};
 session(&app_handle,&window,Some(&epoch))?;
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;references::attach(&app_handle,&s.project_id,expected_revision,&annotation_id,replace,&paths,||{session(&app_handle,&window,Some(&epoch))?;Ok(())})}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn editor_reference_capacity(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,draft:contracts::Draft,annotation_id:Option<String>)->Result<Value,String>{
 let s=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;tauri::async_runtime::spawn_blocking(move||{let _permit=permit;draft.validate()?;let limit=references::route_limit(&app_handle,&draft)?;let used=references::capacity(&app_handle,&s.project_id,&draft,annotation_id.as_deref())?.len();Ok(json!({"used":used,"max":limit,"remaining":limit.saturating_sub(used)}))}).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn editor_storage_command(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,action:String,mode:Option<String>,path:Option<String>, ui_locale:Option<String>)->Result<Value,String>{
 if window.label()!="main"{return Err("Настройки хранения доступны в основном окне".into());}
 if !["info","save","migrate","cache","chooseOutput"].contains(&action.as_str()){return Err("Неизвестная команда хранения".into());}
 let path=if path.is_some()||["info","save"].contains(&action.as_str()){path}else{match rfd::AsyncFileDialog::new().set_title(if action=="chooseOutput"{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.7209e55a616dcd16")}else{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.839cda653445bf95")}).pick_folder().await{Some(folder)=>Some(path_to_string(folder.path())),None=>return Ok(Value::Null)}};
 tauri::async_runtime::spawn_blocking(move||match action.as_str(){"info"=>storage::info(&app_handle),"save"=>storage::save_preferences(&app_handle,mode.as_deref().ok_or("Нет режима")?,path.as_deref()),"chooseOutput"=>storage::save_preferences(&app_handle,"folder",path.as_deref()),"cache"=>storage::migrate_cache(&app_handle,&PathBuf::from(path.ok_or("Нет папки")?)),_=>storage::migrate(&app_handle,&PathBuf::from(path.ok_or("Нет папки")?))}).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn show_editor_reconciliation(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String)->Result<(),String>{session(&app_handle,&window,Some(&epoch))?;viewer_sessions::show_main(&app_handle)?;app_handle.emit_to("main","smart-photo-indexer:editor-reconciliation",json!({"provider":"fal"})).map_err(|e|e.to_string())}

#[tauri::command]
pub async fn get_editor_model_pricing(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,credential_ref:String)->Result<Value,String>{session(&app_handle,&window,Some(&epoch))?;pricing::refresh(&app_handle,&credential_ref).await}

#[tauri::command]
pub async fn preview_editor_replacement(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,asset_id:String,working_turn:u8,mirrored:bool,quality:u8)->Result<Value,String>{
 let owner=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;session(&app_handle,&window,Some(&epoch))?;replacement::preview(&app_handle,&owner.project_id,&asset_id,working_turn,mirrored,quality)}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn replace_editor_original(app_handle:tauri::AppHandle,window:tauri::WebviewWindow,epoch:String,id:String,confirmed:bool)->Result<Value,String>{
 if !confirmed{return Err("Original replacement requires explicit confirmation.".into());}let owner=session(&app_handle,&window,Some(&epoch))?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;session(&app_handle,&window,Some(&epoch))?;replacement::apply(&app_handle,&owner.project_id,&id)}).await.map_err(|e|e.to_string())?
}
