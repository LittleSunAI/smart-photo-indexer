//! Durable Fal jobs outlive every WebView. Submit intent is persisted before HTTP.
use crate::*;
use super::{catalog,contracts::Draft,geometry,local,store};
use serde_json::{json,Value};
use std::sync::{Arc,Mutex,OnceLock};
use std::collections::HashSet;
use std::time::Duration;
use base64::Engine;
#[derive(Default)]pub struct EditorJobs {leased:Mutex<HashSet<String>>,wake:tokio::sync::Notify}
pub(crate) fn wake(app:&tauri::AppHandle){app.state::<EditorJobs>().wake.notify_one();}
impl EditorJobs{pub fn has_leases(&self)->Result<bool,String>{Ok(!self.leased.lock().map_err(|_|"Editor worker lock failed")?.is_empty())}}
fn project_idle(db:&Connection,project:&str,leased:&HashSet<String>)->Result<(),String>{
 store::require_mutable(db,project)?;
 if leased.iter().any(|id|db.query_row("SELECT project_id=?2 FROM editor_jobs WHERE id=?1",params![id,project],|r|r.get::<_,bool>(0)).unwrap_or(true)){return Err("Завершается предыдущая правка этого проекта. Дождитесь освобождения обработки.".into());}Ok(())
}
fn require_project_idle(app:&tauri::AppHandle,db:&Connection,project:&str)->Result<(),String>{
 let runtime=app.state::<EditorJobs>();let leased=runtime.leased.lock().map_err(|_|"Editor worker lock failed")?;project_idle(db,project,&leased)
}
pub(super) fn memory()->Arc<tokio::sync::Semaphore>{static M:OnceLock<Arc<tokio::sync::Semaphore>>=OnceLock::new();M.get_or_init(||Arc::new(tokio::sync::Semaphore::new(96))).clone()}
fn client()->Result<reqwest::Client,String>{let mut builder=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(90));if test_root().is_some(){builder=builder.no_proxy();}builder.build().map_err(|_|"Не удалось создать соединение с Fal.".into())}
fn test_root()->Option<String>{
 if std::env::var("SMART_PHOTO_EDITOR_TEST_TRANSPORT").as_deref()!=Ok("1")||std::env::var_os("SMART_PHOTO_DATA_DIR").is_none(){return None;}
 let raw=std::env::var("SMART_PHOTO_EDITOR_TEST_ENDPOINT").ok()?;let u=reqwest::Url::parse(&raw).ok()?;
 if u.scheme()!="http"||u.host_str()!=Some("127.0.0.1")||u.port().is_none_or(|p|p<1024)||u.username()!=""||u.password().is_some()||u.query().is_some()||u.fragment().is_some()||u.path()!="/"{return None;}Some(raw.trim_end_matches('/').to_string())
}
fn emit(app:&tauri::AppHandle,project:&str){use tauri::Emitter;wake(app);let _=app.emit("smart-photo-indexer:editor-jobs-changed",json!({"projectId":project}));}
pub fn occupied(app:&tauri::AppHandle)->Result<u32,String>{store::connection(app)?.query_row("SELECT COUNT(*) FROM editor_jobs WHERE state IN ('reserved','preparingInputs','ready','submitting','queued','running','cancelRequested','downloading','committingResult')",[],|r|r.get(0)).map_err(|e|e.to_string())}
pub fn require_settled(app:&tauri::AppHandle)->Result<(),String>{let unsettled:bool=store::connection(app)?.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE state NOT IN ('completed','failed','cancelled'))",[],|r|r.get(0)).map_err(|e|e.to_string())?;if unsettled{return Err("Сначала завершите или сверьте незавершённые задачи редактора. Их расход и request ID нельзя потерять при восстановлении.".into());}Ok(())}

pub fn list(app:&tauri::AppHandle,project:Option<&str>)->Result<Value,String>{
 let db=store::connection(app)?;
 let mut q=db.prepare("SELECT j.id,j.project_id,j.state,j.operation,j.model_id,j.source_asset_id,j.request_id,j.reserved_micros,j.spent_micros,j.error,j.result_asset_ids,j.created_at_unix,j.updated_at_unix,j.snapshot,(SELECT MIN(created_at_unix) FROM editor_attempts a WHERE a.job_id=j.id AND a.status<>'notSent'),(SELECT MAX(created_at_unix) FROM editor_steps s WHERE s.job_id=j.id) FROM editor_jobs j WHERE (?1 IS NULL OR j.project_id=?1) ORDER BY j.rowid DESC").map_err(|e|e.to_string())?;
 let rows=q.query_map([project],|r|{
  let snapshot:Value=serde_json::from_str(&r.get::<_,String>(13)?).unwrap_or(Value::Null);
  let first=&snapshot["plan"]["inputs"][0];let turn=snapshot["draft"]["orientation"]["rotation"].as_u64().unwrap_or(0);
  let(w,h)=if turn%2==0{(first["width"].clone(),first["height"].clone())}else{(first["height"].clone(),first["width"].clone())};
  Ok(json!({"id":r.get::<_,String>(0)?,"projectId":r.get::<_,String>(1)?,"state":r.get::<_,String>(2)?,"operation":r.get::<_,String>(3)?,"modelId":r.get::<_,String>(4)?,"sourceAssetId":r.get::<_,String>(5)?,"requestId":r.get::<_,Option<String>>(6)?,"reservedMicros":r.get::<_,u64>(7)?,"spentMicros":r.get::<_,Option<u64>>(8)?,"error":r.get::<_,Option<String>>(9)?,"resultAssetIds":serde_json::from_str::<Value>(&r.get::<_,String>(10)?).unwrap_or(json!([])),"createdAtUnix":r.get::<_,u64>(11)?,"updatedAtUnix":r.get::<_,u64>(12)?,"admission":snapshot["admission"],"startedAtUnix":r.get::<_,Option<u64>>(14)?,"completedAtUnix":r.get::<_,Option<u64>>(15)?,"sourceSize":{"width":w,"height":h},"requestedSize":if snapshot["plan"]["outputTarget"].is_object(){json!({"width":snapshot["plan"]["outputTarget"]["width"],"height":snapshot["plan"]["outputTarget"]["height"]})}else{snapshot["plan"]["fields"]["image_size"].clone()},"requestedResolution":snapshot["plan"]["fields"]["resolution"],"requestedQuality":snapshot["plan"]["fields"]["quality"],"requestedOutputCount":snapshot["plan"]["fields"]["num_images"].as_u64().unwrap_or(1).min(4)}))
 }).map_err(|e|e.to_string())?;
 let mut rows=rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;drop(q);
 let mut assets=db.prepare("SELECT width,height FROM editor_assets WHERE id=?1").map_err(|e|e.to_string())?;
 for row in &mut rows{let count:u64=db.query_row("SELECT COUNT(*) FROM editor_job_outputs WHERE job_id=?1",[row["id"].as_str().unwrap_or("")],|r|r.get(0)).map_err(|e|e.to_string())?;row["providerOutputCount"]=if count>0{json!(count)}else{Value::Null};let mut sizes=Vec::new();for id in row["resultAssetIds"].as_array().into_iter().flatten(){let size=assets.query_row([id.as_str().unwrap_or("")],|r|Ok(json!({"width":r.get::<_,u64>(0)?,"height":r.get::<_,u64>(1)?}))).optional().map_err(|e|e.to_string())?;if let Some(size)=size{sizes.push(size);}}row["resultSizes"]=json!(sizes);let mut q=db.prepare("SELECT o.step_id,o.asset_id,o.output_index,EXISTS(SELECT 1 FROM editor_step_visibility v WHERE v.step_id=o.step_id AND v.hidden=1) FROM editor_job_outputs o WHERE o.job_id=?1 AND o.accepted=1 ORDER BY o.output_index").map_err(|e|e.to_string())?;let rows=q.query_map([row["id"].as_str().unwrap_or("")],|r|Ok(json!({"stepId":r.get::<_,String>(0)?,"assetId":r.get::<_,String>(1)?,"outputIndex":r.get::<_,u64>(2)?,"hidden":r.get::<_,bool>(3)?}))).map_err(|e|e.to_string())?;row["results"]=json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?);}
 for row in &mut rows{let id=row["id"].as_str().unwrap_or("");let unavailable:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs j JOIN editor_project_tombstones t ON t.project_id=j.project_id WHERE j.id=?1) OR EXISTS(SELECT 1 FROM editor_job_outputs o WHERE o.job_id=?1 AND o.accepted=1 AND NOT EXISTS(SELECT 1 FROM editor_working_steps w WHERE w.step_id=o.step_id)) OR EXISTS(SELECT 1 FROM editor_jobs j,json_tree(j.snapshot) t JOIN editor_assets a ON a.id=t.value WHERE j.id=?1 AND NOT EXISTS(SELECT 1 FROM editor_resource_owners own WHERE own.asset_id=a.id AND own.project_id=j.project_id))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;if unavailable{row["metadataOnly"]=json!(true);row["results"]=json!([]);}}
 rows.extend(super::portable_history::list(&db,project)?);rows.sort_by_key(|row|std::cmp::Reverse(row["createdAtUnix"].as_u64().unwrap_or(0)));
 Ok(json!(rows))
}

pub fn create(app:&tauri::AppHandle,project:&str,id:&str,revision:u64,draft:Draft,credential:&str,consent:bool,budget_micros:u64,concurrency:u32,source_step_id:Option<&str>,expected_plan_hash:Option<&str>)->Result<Value,String>{
 profile_preferences::require_cloud(app)?;
 if !consent{return Err("Сначала разрешите отправку выбранных изображений в Fal AI.".into());}if id.len()<8||id.len()>100||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')||budget_micros>1_000_000_000||!(1..=100).contains(&concurrency){return Err("Неверный ID, бюджет или число запросов.".into());}
 let plan=catalog::plan(app,project,&draft)?;if expected_plan_hash.is_some_and(|hash|plan["planHash"].as_str()!=Some(hash)){return Err("Параметры или область изменились после подтверждения. Проверьте правку ещё раз.".into());}if plan["reservedMicros"].as_u64().ok_or("Цена неизвестна")?>budget_micros{return Err("Оценка этой правки превышает лимит одного запуска.".into());}credentials::unlock(app,"fal",credential)?;
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
  let existing:Option<String>=tx.query_row("SELECT snapshot FROM editor_jobs WHERE id=?1 AND project_id=?2",params![id,project],|r|r.get(0)).optional().map_err(|e|e.to_string())?;if let Some(raw)=existing{let frozen:Value=serde_json::from_str(&raw).map_err(|_|"Снимок существующего задания повреждён")?;let historical:Draft=serde_json::from_value(frozen["draft"].clone()).map_err(|_|"Снимок задания повреждён")?;if serde_json::to_value(&historical).map_err(|e|e.to_string())?!=serde_json::to_value(&draft).map_err(|e|e.to_string())?||frozen["budgetMicros"]!=json!(budget_micros)||frozen["concurrency"]!=json!(concurrency){return Err("Этот ID уже закреплён за другим составом или параметрами. Повторная отправка не выполнена.".into());}return Ok(json!({"id":id,"duplicate":true}));}
 require_project_idle(app,&tx,project)?;let current:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if current!=revision{return Err("Черновик изменился после подтверждения отправки.".into());}
 let parent:Option<String>=tx.query_row("SELECT s.id FROM editor_steps s JOIN editor_projects p ON p.active_step_id=s.id WHERE p.id=?1 AND s.asset_id=?2",params![project,draft.asset_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 if parent.is_none()||source_step_id.is_some_and(|id|parent.as_deref()!=Some(id)){return Err("Выбранный шаг и исходный кадр не совпадают. Правка не отправлена.".into());}
 let snapshot=json!({"version":1,"diagnosticTimings":{"accepted":{"firstMs":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis() as u64}},"draft":draft,"plan":plan,"parentId":parent,"budgetMicros":budget_micros,"concurrency":concurrency,"consent":true});
 tx.execute("INSERT INTO editor_jobs(id,project_id,state,operation,model_id,source_asset_id,snapshot,credential_ref,error,created_at_unix,updated_at_unix) VALUES(?1,?2,'validating',?3,?4,?5,?6,?7,'Подготовка запуска',?8,?8)",params![id,project,draft.operation,draft.model_id,draft.asset_id,snapshot.to_string(),credential,unix_now()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_job_activation SET state='revoked' WHERE project_id=?1 AND state='eligible'",[project]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO editor_job_activation(job_id,project_id,source_step_id,state,updated_at_unix) VALUES(?1,?2,?3,'eligible',?4)",params![id,project,parent,unix_now()]).map_err(|e|e.to_string())?;
 store::stash_step_draft(&tx,project,&draft)?;tx.commit().map_err(|e|e.to_string())?;emit(app,project);Ok(json!({"id":id,"queued":true}))
}
fn mark_phase(db:&Connection,id:&str,phase:&str,millis:u64)->Result<(),String>{
 if phase.is_empty()||!phase.bytes().all(|b|b.is_ascii_alphanumeric()){return Err("Invalid diagnostic phase".into());}
 let first=format!("$.diagnosticTimings.{phase}.firstMs");let last=format!("$.diagnosticTimings.{phase}.lastMs");
 db.execute("UPDATE editor_jobs SET snapshot=json_set(json_insert(snapshot,?2,?4),?3,?4) WHERE id=?1",params![id,first,last,millis]).map_err(|e|e.to_string())?;Ok(())
}
fn phase_now(db:&Connection,id:&str,phase:&str)->Result<(),String>{let millis=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis() as u64;mark_phase(db,id,phase,millis)}
fn state(app:&tauri::AppHandle,id:&str,state:&str,error:Option<&str>)->Result<(),String>{let db=store::connection(app)?;if ["failed","cancelled","cancelRequested"].contains(&state){db.execute("UPDATE editor_job_activation SET state='revoked' WHERE job_id=?1 AND state='eligible'",[id]).map_err(|e|e.to_string())?;}let changed=db.execute("UPDATE editor_jobs SET state=CASE WHEN state='cancelRequested' AND ?2 NOT IN ('completed','failed','cancelled','unknownSubmit') THEN state ELSE ?2 END,error=?3,updated_at_unix=?4 WHERE id=?1 AND (state<>?2 OR COALESCE(error,'')<>COALESCE(?3,''))",params![id,state,error,unix_now()]).map_err(|e|e.to_string())?;let project:String=db.query_row("SELECT project_id FROM editor_jobs WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;if changed>0{phase_now(&db,id,state)?;emit(app,&project);}Ok(())}
pub fn start(app:&tauri::AppHandle){let app=app.clone();tauri::async_runtime::spawn(async move{
 let permit=file_safety::image_worker().acquire_owned().await;let Ok(permit)=permit else{return;};
 let initialized=tauri::async_runtime::spawn_blocking({let app=app.clone();move||{let _permit=permit;let db=store::connection(&app)?;super::publication::recover(&app)?;db.execute_batch("BEGIN IMMEDIATE; UPDATE editor_jobs SET state='unknownSubmit',error='Процесс завершился после submit intent. Новый оплачиваемый POST запрещён.' WHERE state='submitting' AND request_id IS NULL; UPDATE editor_attempts SET status='notSent',cost_micros=0,reserved_micros=0 WHERE status='reserved' AND job_id IN (SELECT id FROM editor_jobs WHERE state IN ('validating','reserved','preparingInputs','ready') AND request_id IS NULL); UPDATE editor_jobs SET reserved_micros=0,spent_micros=0,state='interrupted',error='Предыдущая задача не отправлена. Запуск возможен только явной командой из вкладки «Задачи».' WHERE state IN ('validating','reserved','preparingInputs','ready') AND request_id IS NULL; UPDATE editor_jobs SET state='queued',error='Проверяется прежний запрос без повторной отправки' WHERE state IN ('interrupted','unknownSubmit','submitting') AND request_id IS NOT NULL; COMMIT;").map_err(|e|e.to_string())}}).await;
 if !matches!(initialized,Ok(Ok(()))){return;}
 loop{let query=app.clone();let ids=tauri::async_runtime::spawn_blocking(move||{let db=store::connection(&query)?;let mut q=db.prepare("SELECT id FROM editor_jobs WHERE state IN ('validating','queued','running','downloading','committingResult','cancelRequested') ORDER BY rowid LIMIT 100").map_err(|e|e.to_string())?;let values=q.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;Ok::<_,String>(values)}).await;
  let Ok(Ok(ids))=ids else{tokio::time::sleep(Duration::from_secs(5)).await;continue;};let active=!ids.is_empty();for id in ids{let leased={let runtime=app.state::<EditorJobs>();let Ok(mut leased)=runtime.leased.lock()else{continue;};let Ok(db)=store::connection(&app)else{continue;};let Ok(owner)=db.query_row("SELECT project_id FROM editor_jobs WHERE id=?1",[&id],|r|r.get::<_,String>(0))else{continue;};if leased.iter().any(|other|db.query_row("SELECT project_id=?2 FROM editor_jobs WHERE id=?1",params![other,owner],|r|r.get::<_,bool>(0)).unwrap_or(true)){false}else{leased.insert(id.clone())}};if !leased{continue;}let worker=app.clone();tauri::async_runtime::spawn(async move{let result=run(&worker,&id).await;if let Err(e)=result{let db=store::connection(&worker);if let Ok(db)=db{let current:String=db.query_row("SELECT state FROM editor_jobs WHERE id=?1",[&id],|r|r.get(0)).unwrap_or_default();if !["completed","cancelled","failed"].contains(&current.as_str()){let next=if current=="submitting"{"unknownSubmit"}else if ["queued","running","downloading","committingResult","cancelRequested"].contains(&current.as_str()){"interrupted"}else{"failed"};if ["reserved","preparingInputs","ready"].contains(&current.as_str()){let _=db.execute("UPDATE editor_attempts SET status='notSent',cost_micros=0,reserved_micros=0 WHERE job_id=?1 AND status<>'notSent'",[&id]);let _=db.execute("UPDATE editor_jobs SET spent_micros=0,reserved_micros=0 WHERE id=?1",[&id]);}let _=state(&worker,&id,next,Some(&e));}}}if let Ok(mut leased)=worker.state::<EditorJobs>().leased.lock(){leased.remove(&id);}wake(&worker);});}
  if active{tokio::time::sleep(Duration::from_millis(350)).await;}else{app.state::<EditorJobs>().wake.notified().await;}
 }
});}
fn waiting(app:&tauri::AppHandle,id:&str,kind:&str,message:&str,details:Value)->Result<(),String>{
 let admission=json!({"kind":kind,"provider":"fal","details":details});let db=store::connection(app)?;let raw=admission.to_string();let changed=db.execute("UPDATE editor_jobs SET snapshot=json_set(snapshot,'$.admission',json(?2)),error=?3,updated_at_unix=?4 WHERE id=?1 AND state='validating' AND (COALESCE(json_extract(snapshot,'$.admission'),'')<>?2 OR COALESCE(error,'')<>?3)",params![id,raw,message,unix_now()]).map_err(|e|e.to_string())?;if changed>0{let project:String=db.query_row("SELECT project_id FROM editor_jobs WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;emit(app,&project);}Ok(())
}
fn reserve(app:&tauri::AppHandle,id:&str,snapshot:&Value)->Result<bool,String>{processing::cloud_fence(app,||{if index_backups::restore_blocked(app)?{waiting(app,id,"restorePending","Запуск приостановлен: подготовлено восстановление базы. Завершите или отмените его в настройках.",json!({}))?;return Ok(false);}
 let mut db=store::connection(app)?;let own=occupied(app)?;
 let limit=snapshot["concurrency"].as_u64().unwrap_or(1) as u32;let global=processing::active_cloud_limit(app,"fal")?.max(limit);
 if own>=limit||own+processing::occupied_cloud(app,"fal")?>=global{
 let used=own+processing::occupied_cloud(app,"fal")?;
 waiting(app,id,"slotsBusy",&format!("Подготовка запуска: локальный лимит запросов {used}/{global}. Запуск продолжится автоматически."),json!({"used":used,"limit":global}))?;return Ok(false);
 }
 let amount=snapshot["plan"]["reservedMicros"].as_u64().ok_or("Цена неизвестна")?;
 if amount>snapshot["budgetMicros"].as_u64().unwrap_or(0){state(app,id,"failed",Some("Оценка этой правки превышает лимит одного запуска. Измените параметры или лимит."))?;return Ok(false);}
 if let Some(limit)=snapshot["draft"]["formValues"]["__overallBudgetUsd"].as_f64().filter(|v|v.is_finite()&&*v>0.) {
  let since=snapshot["draft"]["formValues"]["__budgetSince"].as_u64().ok_or("Не задано начало периода общего бюджета")?;
  let total:u64=db.query_row("SELECT COALESCE(SUM(COALESCE(cost_micros,reserved_micros)),0) FROM editor_attempts WHERE created_at_unix>=?1",[since],|r|r.get(0)).map_err(|e|e.to_string())?;
  if total.saturating_add(amount)>(limit*1_000_000.).floor() as u64{state(app,id,"failed",Some("Достигнут включённый общий бюджет периода. Расходы и неизвестные резервы сохранены."))?;return Ok(false);}
 }
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let changed=tx.execute("UPDATE editor_jobs SET state='reserved',reserved_micros=?2,error=NULL,snapshot=json_remove(snapshot,'$.admission') WHERE id=?1 AND state='validating'",params![id,amount]).map_err(|e|e.to_string())?;if changed==0{return Ok(false);}
 tx.execute("INSERT INTO editor_attempts(id,job_id,status,reserved_micros,created_at_unix) VALUES(?1,?2,'reserved',?3,?4)",params![format!("{id}-attempt-{}",tx.query_row("SELECT COUNT(*) FROM editor_attempts WHERE job_id=?1",[id],|r|r.get::<_,u64>(0)).map_err(|e|e.to_string())?),id,amount,unix_now()]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(true)
})}
fn tracking(raw:&str,model:&str,id:&str,kind:&str)->Result<reqwest::Url,String>{
 let root=model.split('/').take(2).collect::<Vec<_>>().join("/");let suffix=match kind{"status"=>"/status","cancel"=>"/cancel",_=>""};let expected=format!("/requests/{id}{suffix}");let url=reqwest::Url::parse(raw).map_err(|_|"Некорректная ссылка очереди Fal")?;
 let controlled=test_root().and_then(|root|reqwest::Url::parse(&root).ok()).is_some_and(|root|url.scheme()==root.scheme()&&url.host_str()==root.host_str()&&url.port()==root.port());
 if id.is_empty()||id.len()>160||!id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_')||!(controlled||url.scheme()=="https"&&url.host_str()==Some("queue.fal.run")&&url.port_or_known_default()==Some(443))||url.username()!=""||url.password().is_some()||url.query().is_some()||url.fragment().is_some()||!url.path().starts_with(&format!("/{root}/"))||!url.path().ends_with(&expected){return Err("Недоверенная ссылка очереди отклонена до отправки ключа.".into());}Ok(url)
}
async fn run(app:&tauri::AppHandle,id:&str)->Result<(),String>{
 let (project,model,raw,credential,mut request_id,mut phase):(String,String,String,String,Option<String>,String)=store::connection(app)?.query_row("SELECT project_id,model_id,snapshot,credential_ref,request_id,state FROM editor_jobs WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|e|e.to_string())?;
 let snapshot:Value=serde_json::from_str(&raw).map_err(|_|"Снимок задания повреждён")?;
 if let Some(output)=super::results::cached_response(app,id)? {
  let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
  let saved_app=app.clone();let saved_id=id.to_string();let saved_snapshot=snapshot.clone();let saved_output=output.clone();
  let saved=tauri::async_runtime::spawn_blocking(move||{let _permit=permit;super::results::saved(&saved_app,&saved_id,&saved_snapshot,&saved_output)}).await.map_err(|e|e.to_string())??;
  if let Some(results)=saved{return finish_results(app,id,&project,&snapshot,&results,&output).await;}
 }
 let key=credentials::unlock(app,"fal",&credential)?;let client=client()?;
 if request_id.is_none(){if !reserve(app,id,&snapshot)?{return Ok(());}let changed=store::connection(app)?.execute("UPDATE editor_jobs SET state='preparingInputs' WHERE id=?1 AND state='reserved'",[id]).map_err(|e|e.to_string())?;if changed==0{return Ok(());}emit(app,&project);
  // At most one 24 MiB prepared JSON body (including serialization copies) waits here.
  let memory=memory().acquire_many_owned(72).await.map_err(|e|e.to_string())?;let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;let prepare_app=app.clone();let frozen=snapshot.clone();let project_copy=project.clone();let payload=tauri::async_runtime::spawn_blocking(move||{let _permit=permit;prepare(&prepare_app,&project_copy,&frozen)}).await.map_err(|e|e.to_string())??;
  let payload_bytes=serde_json::to_vec(&payload).map_err(|e|e.to_string())?;if payload_bytes.len()>24*1024*1024{return Err("Подготовленный запрос превышает 24 МиБ. Изображения не отправлены.".into());}drop(payload);
  // Explicit opt-in in an isolated debug profile only; never record credentials.
  #[cfg(debug_assertions)]
  if std::env::var_os("SMART_PHOTO_DATA_DIR").is_some()&&std::env::var("SMART_PHOTO_EDITOR_CAPTURE_INPUT").as_deref()==Ok("1"){
   let diagnostics=store::root(app)?.join("diagnostics");std::fs::create_dir_all(&diagnostics).map_err(|e|e.to_string())?;
   file_safety::atomic_save(&diagnostics.join(format!("{id}-input.json")),|out,_|std::io::Write::write_all(out,&payload_bytes).map_err(|e|e.to_string()))?;
  }
  let request=client.post(format!("{}/{model}",test_root().unwrap_or_else(||"https://queue.fal.run".into()))).header("Authorization",format!("Key {}",key.as_str())).header("X-Fal-No-Retry","1").header("Content-Type","application/json").body(payload_bytes);
  let response=send_with_consent(app,id,request).await?;drop(memory);
  let code=response.status();if matches!(code.as_u16(),401|403){let _=provider_transport::invalidate_readiness(app,"fal",&credential);}if !code.is_success(){let body=provider_transport::bounded_json(response,1024*1024).await.unwrap_or(Value::Null);let details=body["detail"].as_array().map(|items|items.iter().take(4).filter_map(|item|{let field=item["loc"].as_array()?.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(".");let kind=item["type"].as_str().unwrap_or("validation_error");if field.len()>100||kind.len()>80||!field.chars().all(|c|c.is_ascii_alphanumeric()||c=='.'||c=='_')||!kind.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'){return None;}Some(format!("{field}: {kind}"))}).collect::<Vec<_>>().join("; ")).unwrap_or_default();let _=state(app,id,"failed",Some(&format!("Fal отклонил отправку: HTTP {}. {} Новый запуск только явно; резерв расхода сохранён.",code.as_u16(),details)));return Ok(());}
  let remote=provider_transport::bounded_json(response,1024*1024).await?;let remote_id=remote["request_id"].as_str().ok_or("Fal не вернул request ID; автоматический повтор запрещён")?.to_string();
  // Persist the identifier before validating optional returned URLs or polling.
  store::connection(app)?.execute("UPDATE editor_jobs SET request_id=?2 WHERE id=?1",params![id,remote_id]).map_err(|e|e.to_string())?;
  let root=model.split('/').take(2).collect::<Vec<_>>().join("/");for(field,kind,suffix)in[("status_url","status","/status"),("response_url","response",""),("cancel_url","cancel","/cancel")]{let fallback=format!("{}/{root}/requests/{remote_id}{suffix}",test_root().unwrap_or_else(||"https://queue.fal.run".into()));let url=tracking(remote[field].as_str().unwrap_or(&fallback),&model,&remote_id,kind).or_else(|_|tracking(&fallback,&model,&remote_id,kind))?;store::connection(app)?.execute(&format!("UPDATE editor_jobs SET {field}=?2 WHERE id=?1"),params![id,url.as_str()]).map_err(|e|e.to_string())?;}
  store::connection(app)?.execute("UPDATE editor_attempts SET status='remotePending',request_id=?2 WHERE job_id=?1 AND status='submitting'",params![id,remote_id]).map_err(|e|e.to_string())?;request_id=Some(remote_id);state(app,id,"queued",None)?;phase="queued".into();
 }
 let remote_id=request_id.ok_or("Нет идентификатора запроса")?;let db=store::connection(app)?;let(status_url,response_url,cancel_url):(Option<String>,Option<String>,Option<String>)=db.query_row("SELECT status_url,response_url,cancel_url FROM editor_jobs WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;drop(db);
 let resolve=|saved:Option<String>,kind:&str,suffix:&str|->Result<reqwest::Url,String>{let root=model.split('/').take(2).collect::<Vec<_>>().join("/");let fallback=format!("{}/{root}/requests/{remote_id}{suffix}",test_root().unwrap_or_else(||"https://queue.fal.run".into()));let url=tracking(saved.as_deref().unwrap_or(&fallback),&model,&remote_id,kind).or_else(|_|tracking(&fallback,&model,&remote_id,kind))?;store::connection(app)?.execute(&format!("UPDATE editor_jobs SET {kind}_url=?2 WHERE id=?1"),params![id,url.as_str()]).map_err(|e|e.to_string())?;Ok(url)};
 let status_url=resolve(status_url,"status","/status")?;let response_url=resolve(response_url,"response","")?;let cancel_url=resolve(cancel_url,"cancel","/cancel")?;
 for _ in 0..600{
  let local_state:String=store::connection(app)?.query_row("SELECT state FROM editor_jobs WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
  if local_state=="cancelRequested"{let response=client.put(cancel_url.clone()).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Не удалось проверить отмену Fal")?;if response.status().as_u16()!=202&&response.status().as_u16()!=400&&response.status().as_u16()!=404{return Err("Отмена не подтверждена сервисом; request ID и резерв сохранены.".into());}}
  let response=client.get(status_url.clone()).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Не удалось получить статус Fal; продолжение проверит тот же request ID")?;if !response.status().is_success(){return Err("Fal временно не вернул статус; запрос не отправлялся повторно.".into());}let status=provider_transport::bounded_json(response,1024*1024).await?;
  match status["status"].as_str(){Some("IN_QUEUE")|Some("IN_PROGRESS")=>{let next=if local_state=="cancelRequested"{"cancelRequested"}else if status["status"]=="IN_PROGRESS"{"running"}else{"queued"};if phase!=next{state(app,id,next,None)?;phase=next.into();}},Some("CANCELLED")=>{store::connection(app)?.execute("UPDATE editor_attempts SET status='cancelled' WHERE job_id=?1 AND status='remotePending'",[id]).map_err(|e|e.to_string())?;state(app,id,"cancelled",Some("Сервис подтвердил отмену. Списание проверяется отдельно."))?;return Ok(());},Some("COMPLETED")=>{
    if let Some(message)=status["error"].as_str().filter(|s|!s.trim().is_empty()){let safe=message.chars().filter(|c|!c.is_control()).take(1000).collect::<String>();state(app,id,"failed",Some(&format!("Fal завершил запрос с ошибкой: {safe}. Фактический расход неизвестен; резерв сохранён.")))?;return Ok(());}
    let response=client.get(response_url).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Не удалось получить известный результат; новый POST не нужен")?;if !response.status().is_success(){let code=response.status().as_u16();let detail=provider_transport::bounded_json(response,1024*1024).await.unwrap_or(Value::Null);let filtered=detail["detail"].as_array().is_some_and(|items|items.iter().any(|v|v["type"]=="content_policy_violation"));let message=if filtered{"Модель отклонила изображение своим фильтром содержимого. Результата нет; повтор не выполнялся, расход неизвестен.".to_string()}else{format!("Сервис завершил задачу с ошибкой HTTP {code}; результата изображения нет. Расход неизвестен.")};state(app,id,"failed",Some(&message))?;return Ok(());}let billable=response.headers().get("x-fal-billable-units").and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<f64>().ok()).filter(|v|v.is_finite()&&*v>=0.&&*v<=1000000.);let output=provider_transport::bounded_json(response,4*1024*1024).await?;
    if let Some(units)=billable{let price=snapshot["plan"]["route"]["priceMicros"].as_u64().unwrap_or(0);let cost=(units*price as f64).ceil() as u64;let known=snapshot["plan"]["route"]["pricingAuthenticated"]==json!(true);let db=store::connection(app)?;db.execute("UPDATE editor_attempts SET response_meta=json_set(COALESCE(response_meta,'{}'),'$.billableUnits',?2,'$.unitPriceMicros',?3,'$.currency','USD','$.headerCostMicros',?4,'$.actualCostKnown',json(?5)) WHERE job_id=?1 AND status<>'notSent'",params![id,units,price,cost,if known{"true"}else{"false"}]).map_err(|e|e.to_string())?;if known{db.execute("UPDATE editor_attempts SET cost_micros=?2 WHERE job_id=?1 AND status<>'notSent'",params![id,cost]).map_err(|e|e.to_string())?;db.execute("UPDATE editor_jobs SET spent_micros=?2 WHERE id=?1",params![id,cost]).map_err(|e|e.to_string())?;}}

   let descriptors=super::results::descriptors(&output);
   if descriptors.is_empty(){state(app,id,"failed",Some("Сервис завершил запрос без изображения. Повторная генерация не выполнялась."))?;return Ok(());}
   if descriptors.len()>32{return Err("Ответ содержит слишком много outputs для безопасной обработки".into());}
   super::results::cache_response(app,id,&output)?;
   state(app,id,"downloading",None)?;
   let count=super::results::wanted(&snapshot,&output);let mut results=Vec::new();let mut errors=Vec::new();
   for (index,descriptor) in descriptors.iter().take(count).enumerate(){
    let result=async {
     let url=descriptor["url"].as_str().ok_or("Нет URL результата")?;
     let memory=memory().acquire_many_owned(32).await.map_err(|e|e.to_string())?;
     let bytes=download(&client,url).await?;
     let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
     let app=app.clone();let job=id.to_string();let frozen=snapshot.clone();let descriptor=descriptor.clone();
     let result=tauri::async_runtime::spawn_blocking(move||{let _permit=permit;super::results::persist(&app,&job,index,&descriptor,&bytes)?;drop(bytes);super::results::process(&app,&job,index,&frozen)}).await.map_err(|e|e.to_string())?;
     drop(memory);result
    }.await;
    match result{Ok(asset)=>results.push(asset),Err(error)=>errors.push(error)}
   }
   store::connection(app)?.execute("UPDATE editor_attempts SET response_meta=json_set(COALESCE(response_meta,'{}'),'$.outputsFinished',json('true')) WHERE job_id=?1 AND status<>'notSent'",[id]).map_err(|e|e.to_string())?;
   if results.is_empty(){return Err(format!("Не удалось сохранить изображения. Ответ и квитанция сохранены; новый POST не нужен. {}",errors.join("; ")));}
   super::results::partial_warning(&mut results,count);
   if output.get("error").is_some()||output["has_nsfw_concepts"].as_array().is_some_and(|v|v.iter().any(|v|v==&json!(true))){
    let mut warnings=results[0]["warnings"].as_array().cloned().unwrap_or_default();warnings.push(json!("Поставщик сообщил ограничение результата; показаны доступные изображения из ответа."));results[0]["warnings"]=json!(warnings);
   }
   return finish_results(app,id,&project,&snapshot,&results,&output).await;
  },_=>return Err("Неизвестный статус Fal; сохранён прежний request ID.".into())}
  tokio::time::sleep(Duration::from_secs(1)).await;
 }Err("Ожидание сервиса приостановлено. Продолжение проверит прежний запрос без новой оплаты.".into())
}
async fn finish_results(app:&tauri::AppHandle,id:&str,project:&str,snapshot:&Value,results:&[Value],output:&Value)->Result<(),String>{
 state(app,id,"committingResult",None)?;
 let permit=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
 let save_app=app.clone();let save_id=id.to_string();let save_project=project.to_string();let save_snapshot=snapshot.clone();let saved_results=results.to_vec();let saved_output=output.clone();
 tauri::async_runtime::spawn_blocking(move||{let _permit=permit;commit(&save_app,&save_id,&save_project,&save_snapshot,&saved_results,&saved_output)}).await.map_err(|e|e.to_string())??;
 let actual:Option<u64>=store::connection(app)?.query_row("SELECT spent_micros FROM editor_jobs WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
 let mut message=actual.map(|cost|format!("Готово. Расход по данным Fal: ${:.4}.",cost as f64/1_000_000.)).unwrap_or_else(||"Готово. Цена оценочная; фактическое списание сверяется в Fal.".into());
 let mut warnings=Vec::new();for asset in results{for warning in asset["warnings"].as_array().into_iter().flatten().filter_map(Value::as_str){if !warnings.contains(&warning){warnings.push(warning);}}}
 if !warnings.is_empty(){message.push(' ');message.push_str(&warnings.join(" "));}
 state(app,id,"completed",Some(&message))
}
fn uri(image:&image::DynamicImage)->Result<String,String>{let mut data=std::io::Cursor::new(Vec::new());image.write_to(&mut data,image::ImageFormat::Png).map_err(|e|e.to_string())?;let data=data.into_inner();if data.len()>12*1024*1024{return Err("Один вход превышает 12 МиБ после подготовки. Уменьшите рабочую копию явно.".into());}Ok(format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(data)))}
fn prepare(app:&tauri::AppHandle,project:&str,snapshot:&Value)->Result<Value,String>{
 let d:Draft=serde_json::from_value(snapshot["draft"].clone()).map_err(|e|e.to_string())?;let plan=&snapshot["plan"];let mut fields=plan["fields"].as_object().ok_or("Неверные поля снимка")?.clone();let mut inputs=Vec::new();
 for (i,input)in plan["inputs"].as_array().ok_or("Нет входов")?.iter().enumerate(){let a=local::project_asset(app,project,input["assetId"].as_str().ok_or("Неверный source")?)?;let path=store::owned_path(app,a["relativePath"].as_str().ok_or("Неверный owned source")?)?;if compute_file_sha256(&path)?!=a["sha256"].as_str().unwrap_or(""){return Err("Исходный файл проекта изменился.".into());}let mut image=file_safety::decode_bounded(&path)?;if i==0{image=geometry::orient_image(image,d.working_turn,d.mirrored);if !plan["expansion"].is_null()&&!["fal-ai/flux-2-pro/outpaint","fal-ai/bria/expand"].contains(&plan["route"]["id"].as_str().unwrap_or("")){let e=&plan["expansion"];let w=image.width()+e["left"].as_u64().unwrap() as u32+e["right"].as_u64().unwrap() as u32;let h=image.height()+e["top"].as_u64().unwrap() as u32+e["bottom"].as_u64().unwrap() as u32;let mut expanded=image::RgbaImage::new(w,h);image::imageops::overlay(&mut expanded,&image.to_rgba8(),e["left"].as_u64().unwrap() as i64,e["top"].as_u64().unwrap() as i64);image=image::DynamicImage::ImageRgba8(expanded);}}inputs.push(uri(&image)?);}
 let field=plan["route"]["inputField"].as_str().ok_or("Нет image field")?;fields.insert(field.into(),if field=="image_urls"{json!(inputs)}else{json!(inputs[0])});
 if let Some(field)=plan["route"]["maskField"].as_str().filter(|_|plan["masks"].as_array().is_some_and(|v|!v.is_empty())||plan["derivedMask"].is_object()){let a=local::project_asset(app,project,&d.asset_id)?;let mut union=if plan["derivedMask"].is_object(){if plan["masks"].as_array().is_some_and(|m|!m.is_empty()){return Err("Явная и производная маски не объединяются.".into());}super::spatial::raster(&plan["derivedMask"],&a,&d)?}else{image::GrayImage::new(a["width"].as_u64().unwrap() as u32,a["height"].as_u64().unwrap() as u32)};for m in plan["masks"].as_array().ok_or("Нет списка масок")?{let a=local::project_asset(app,project,m["maskAssetId"].as_str().ok_or("Неверная маска")?)?;let path=store::owned_path(app,a["relativePath"].as_str().ok_or("Неверный mask owned path")?)?;let mask=file_safety::decode_bounded(&path)?.into_luma8();if mask.dimensions()!=union.dimensions(){return Err("Размер маски отличается от основного кадра.".into());}for (p,m)in union.pixels_mut().zip(mask.pixels()){p[0]=p[0].max(m[0]);}}if !union.pixels().any(|p|p[0]>0){return Err("Объединённая маска пуста.".into());}let raster=if plan["route"]["maskEncoding"]=="transparent-edits"{image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(union.width(),union.height(),|x,y|image::Rgba({let m=union.get_pixel(x,y)[0];[m,m,m,255-m]})))}else{image::DynamicImage::ImageLuma8(union)};fields.insert(field.into(),json!(uri(&geometry::orient_image(raster,d.working_turn,d.mirrored))?));}
 let payload=Value::Object(fields);catalog::validate_value(&plan["route"]["inputSchema"],&payload,&plan["route"]["definitions"],0)?;Ok(payload)
}
async fn download(client:&reqwest::Client,raw:&str)->Result<Vec<u8>,String>{
 let controlled=test_root();super::result_download::download(client,raw,controlled.as_deref()).await
}

fn commit(app:&tauri::AppHandle,id:&str,project:&str,snapshot:&Value,results:&[Value],output:&Value)->Result<(),String>{
 let draft:Draft=serde_json::from_value(snapshot["draft"].clone()).map_err(|e|e.to_string())?;
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let actual=output["images"].as_array().cloned().unwrap_or_else(||vec![output["image"].clone()]);
 let mut ids=vec![];let mut first:Option<(String,Draft)>=None;
 for asset in results{
  let i=asset["outputIndex"].as_u64().ok_or("Нет output index")? as usize;
  let asset_id=asset["id"].as_str().ok_or("Нет result asset")?;ids.push(asset_id.to_string());let step=format!("{id}-output-{i}");
  if let Some(raw)=asset["providerRawAssetId"].as_str(){tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'reference')",params![project,raw]).map_err(|e|e.to_string())?;}
  tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'result')",params![project,asset_id]).map_err(|e|e.to_string())?;
  tx.execute("INSERT OR IGNORE INTO editor_steps(id,project_id,asset_id,operation,title,parameters,annotations,job_id,created_at_unix) VALUES(?1,?2,?3,?4,?5,?6,'[]',?7,?8)",params![step,project,asset_id,draft.operation,format!("{} · {}",draft.operation,i+1),json!({"fields":snapshot["plan"]["fields"],"schemaSha256":snapshot["plan"]["route"]["schemaSha256"],"modelId":snapshot["plan"]["route"]["id"],"inputOrientation":snapshot["draft"]["orientation"],"providerOutputSha256":asset["providerOutputSha256"],"postProcess":asset["postProcess"],"outputTarget":snapshot["plan"]["outputTarget"],"outputCorrection":asset["outputCorrection"],"warnings":asset["warnings"],"actualSize":{"width":asset["width"],"height":asset["height"]}}).to_string(),id,unix_now()]).map_err(|e|e.to_string())?;
  let working=draft.new_frame(asset_id);store::validate_draft_resources(&tx,project,&working)?;
  tx.execute("INSERT OR IGNORE INTO editor_step_drafts(step_id,draft) VALUES(?1,?2)",params![step,serde_json::to_string(&working).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
  if let Some(parent)=snapshot["parentId"].as_str(){tx.execute("INSERT OR IGNORE INTO editor_edges(step_id,parent_id) VALUES(?1,?2)",params![step,parent]).map_err(|e|e.to_string())?;}
  let mut descriptor=actual.get(i).cloned().unwrap_or_else(||json!({}));descriptor["warnings"]=asset["warnings"].clone();descriptor["actualSize"]=json!({"width":asset["width"],"height":asset["height"]});descriptor["postProcess"]=asset["postProcess"].clone();descriptor["outputCorrection"]=asset["outputCorrection"].clone();
  tx.execute("INSERT INTO editor_job_outputs(job_id,output_index,asset_id,step_id,accepted,provider_descriptor) VALUES(?1,?2,?3,?4,1,?5) ON CONFLICT(job_id,output_index) DO UPDATE SET asset_id=excluded.asset_id,step_id=excluded.step_id,accepted=1,provider_descriptor=excluded.provider_descriptor",params![id,i as u64,asset_id,step,descriptor.to_string()]).map_err(|e|e.to_string())?;
  tx.execute("UPDATE editor_file_operations SET state='committed' WHERE id=?1",[format!("provider-output-{id}-{i}")]).map_err(|e|e.to_string())?;
  if first.is_none(){first=Some((step,working));}
 }
 for(i,descriptor)in actual.iter().enumerate().filter(|(i,_)|!results.iter().any(|asset|asset["outputIndex"].as_u64()==Some(*i as u64))){tx.execute("INSERT OR IGNORE INTO editor_job_outputs(job_id,output_index,accepted,provider_descriptor) VALUES(?1,?2,0,?3)",params![id,i as u64,descriptor.to_string()]).map_err(|e|e.to_string())?;}
 if let Some((step,working))=first{
  let eligible:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_job_activation a JOIN editor_projects p ON p.id=a.project_id JOIN editor_jobs j ON j.id=a.job_id WHERE a.job_id=?1 AND a.project_id=?2 AND a.state='eligible' AND p.active_step_id=a.source_step_id AND j.state NOT IN ('cancelRequested','cancelled','failed') AND NOT EXISTS(SELECT 1 FROM editor_project_tombstones t WHERE t.project_id=p.id))",params![id,project],|r|r.get(0)).map_err(|e|e.to_string())?;
  if eligible{
   tx.execute("UPDATE editor_projects SET active_step_id=?2,draft=?3,revision=revision+1,updated_at_unix=?4 WHERE id=?1",params![project,step,serde_json::to_string(&working).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
   tx.execute("UPDATE editor_job_activation SET state='consumed',activated_step_id=?2,updated_at_unix=?3 WHERE job_id=?1",params![id,step,unix_now()]).map_err(|e|e.to_string())?;
  }
 }
 tx.execute("UPDATE editor_jobs SET result_asset_ids=?2 WHERE id=?1",params![id,json!(ids).to_string()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_attempts SET status='completed',usage=?2 WHERE job_id=?1 AND status<>'notSent'",params![id,json!({"estimatedMicros":snapshot["plan"]["reservedMicros"],"outputCount":ids.len(),"providerOutputCount":actual.len(),"requestedOutputCount":snapshot["plan"]["outputCount"],"providerTimings":output.get("timings")}).to_string()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}

pub fn command(app:&tauri::AppHandle,project:Option<&str>,id:&str,action:&str)->Result<Value,String>{processing::cloud_fence(app,||{let db=store::connection(app)?;let historical:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_imported_jobs WHERE id=?1)",[id],|r|r.get(0)).map_err(|e|e.to_string())?;if historical{return Err("Перенесённая задача сохранена только для просмотра. Новый запуск создаётся из текущего кадра.".into());}let (owner,current,request):(String,String,Option<String>)=db.query_row("SELECT project_id,state,request_id FROM editor_jobs WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;if project.is_some_and(|p|p!=owner){return Err("Задача принадлежит другому проекту.".into());}match action{"cancel"=>{if current=="unknownSubmit"&&request.is_none(){return Err("Идентификатор отправки неизвестен. Проверьте запуск в Fal; автоматическая отмена или повтор не подтверждены.".into());}if request.is_none()&&["validating","reserved","preparingInputs","ready","interrupted"].contains(&current.as_str()){db.execute("UPDATE editor_attempts SET status='notSent',cost_micros=0,reserved_micros=0 WHERE job_id=?1 AND status IN ('reserved','submitting')",[id]).map_err(|e|e.to_string())?;db.execute("UPDATE editor_jobs SET reserved_micros=0,spent_micros=0 WHERE id=?1",[id]).map_err(|e|e.to_string())?;state(app,id,"cancelled",Some("Отменено до отправки."))?;}else if !["completed","failed","cancelled"].contains(&current.as_str()){state(app,id,"cancelRequested",Some("Запрошена отмена; результат и расход пока не подтверждены."))?;}},"resume"=>{if current!="interrupted"&&!(current=="unknownSubmit"&&request.is_some()){return Err("Продолжать можно только прерванную задачу. Неизвестный submit не повторяется.".into());}require_project_idle(app,&db,&owner)?;if request.is_some(){state(app,id,"queued",None)?;}else{db.execute("UPDATE editor_attempts SET status='notSent',cost_micros=0,reserved_micros=0 WHERE job_id=?1 AND status IN ('reserved','submitting')",[id]).map_err(|e|e.to_string())?;db.execute("UPDATE editor_jobs SET reserved_micros=0 WHERE id=?1",[id]).map_err(|e|e.to_string())?;state(app,id,"validating",None)?;}},_=>return Err("Неизвестная команда задачи.".into())}list(app,Some(&owner))})}

pub fn reconcile_request(app:&tauri::AppHandle,project:Option<&str>,id:&str,request:&str,confirmed:bool)->Result<Value,String>{
 if !confirmed{return Err("Сначала сверьте модель, время и файлы задания в истории Fal.".into());}
 processing::cloud_fence(app,||{let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let(owner,current,old,model):(String,String,Option<String>,String)=tx.query_row("SELECT project_id,state,request_id,model_id FROM editor_jobs WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
 if project.is_some_and(|p|p!=owner)||current!="unknownSubmit"||old.is_some(){return Err("Сверка доступна только для неизвестной отправки этого проекта без request ID.".into());}
 let request=request.trim();let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE request_id=?1)",[request],|r|r.get(0)).map_err(|e|e.to_string())?;if duplicate{return Err("Этот request ID уже связан с другим заданием.".into());}
 let root=model.split('/').take(2).collect::<Vec<_>>().join("/");let base=test_root().unwrap_or_else(||"https://queue.fal.run".into());let status=tracking(&format!("{base}/{root}/requests/{request}/status"),&model,request,"status")?;let response=tracking(&format!("{base}/{root}/requests/{request}"),&model,request,"response")?;let cancel=tracking(&format!("{base}/{root}/requests/{request}/cancel"),&model,request,"cancel")?;
 tx.execute("UPDATE editor_jobs SET request_id=?2,status_url=?3,response_url=?4,cancel_url=?5,state='queued',error='Идентификатор сверён пользователем; проверяется прежняя отправка.',updated_at_unix=?6 WHERE id=?1",params![id,request,status.as_str(),response.as_str(),cancel.as_str(),unix_now()]).map_err(|e|e.to_string())?;tx.execute("UPDATE editor_attempts SET status='remotePending',request_id=?2 WHERE job_id=?1 AND status='submitting'",params![id,request]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;emit(app,&owner);list(app,Some(&owner))})
}

// Admission and the first poll share the profile-revocation fence.
async fn send_with_consent(app:&tauri::AppHandle,id:&str,request:reqwest::RequestBuilder)->Result<reqwest::Response,String>{
 use std::future::{poll_fn,Future};let mut future=Box::pin(request.send());let mut submitted=false;
 poll_fn(|context|{if submitted{return future.as_mut().poll(context).map(|r|r.map_err(|_|"Submission response lost; automatic repeat is disabled.".to_string()));}
  match processing::cloud_fence(app,||{profile_preferences::require_cloud(app)?;let db=store::connection(app)?;let changed=db.execute("UPDATE editor_jobs SET state='submitting',updated_at_unix=?2 WHERE id=?1 AND state='preparingInputs'",params![id,unix_now()]).map_err(|e|e.to_string())?;if changed!=1{return Err("Submission cancelled before admission.".into());}db.execute("UPDATE editor_attempts SET status='submitting' WHERE job_id=?1 AND status='reserved'",[id]).map_err(|e|e.to_string())?;phase_now(&db,id,"submitting")?;submitted=true;Ok(future.as_mut().poll(context))}){Ok(value)=>value.map(|r|r.map_err(|_|"Submission response lost; automatic repeat is disabled.".to_string())),Err(error)=>std::task::Poll::Ready(Err(error))}
 }).await
}


#[cfg(test)]
mod project_admission_tests{
 use super::*;
 fn fixture()->Connection{let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE editor_jobs(id TEXT PRIMARY KEY,project_id TEXT,state TEXT);INSERT INTO editor_jobs VALUES('a','one','interrupted'),('b','one','running'),('c','two','failed');").unwrap();db}
 #[test]fn resume_and_new_start_share_active_project_guard(){
  let db=fixture();assert!(project_idle(&db,"one",&HashSet::new()).is_err());
  db.execute("UPDATE editor_jobs SET state='failed' WHERE id='b'",[]).unwrap();
  assert!(project_idle(&db,"one",&HashSet::new()).is_ok());
  assert!(project_idle(&db,"one",&HashSet::from(["a".into()])).is_err());
 }
 #[test]fn another_project_lease_does_not_block_failed_project(){
  let db=fixture();db.execute("UPDATE editor_jobs SET state='failed' WHERE id='b'",[]).unwrap();
  assert!(project_idle(&db,"one",&HashSet::from(["c".into()])).is_ok());
 }
 #[test]fn phase_diagnostics_preserve_first_timestamp_and_frozen_request(){
  let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE editor_jobs(id TEXT PRIMARY KEY,snapshot TEXT);INSERT INTO editor_jobs VALUES('j','{\"draft\":{\"prompt\":\"keep\"},\"receipt\":\"same\"}');").unwrap();
  mark_phase(&db,"j","submitting",1000).unwrap();mark_phase(&db,"j","running",1100).unwrap();mark_phase(&db,"j","submitting",1200).unwrap();mark_phase(&db,"j","downloading",1300).unwrap();
  let raw:String=db.query_row("SELECT snapshot FROM editor_jobs",[],|r|r.get(0)).unwrap();let v:Value=serde_json::from_str(&raw).unwrap();assert_eq!(v["diagnosticTimings"]["submitting"]["firstMs"],1000);assert_eq!(v["diagnosticTimings"]["submitting"]["lastMs"],1200);assert_eq!(v["draft"]["prompt"],"keep");assert_eq!(v["receipt"],"same");assert!(mark_phase(&db,"j","bad.path",1400).is_err());
 }

}
