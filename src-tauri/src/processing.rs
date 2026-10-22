//! Durable coordinator. No provider work belongs to React or a window lifetime.
use crate::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::watch;
use std::sync::{Arc,OnceLock,atomic::{AtomicBool,Ordering}};

fn preparations()->&'static Mutex<HashMap<String,Arc<AtomicBool>>>{
    static MAP:OnceLock<Mutex<HashMap<String,Arc<AtomicBool>>>>=OnceLock::new();MAP.get_or_init(||Mutex::new(HashMap::new()))
}
struct PreparationGuard { key:String, cancel:Arc<AtomicBool> }
impl Drop for PreparationGuard {fn drop(&mut self){if let Ok(mut map)=preparations().lock(){if map.get(&self.key).is_some_and(|token|Arc::ptr_eq(token,&self.cancel)){map.remove(&self.key);}}}}
fn preparation_token(key:&str)->Result<Arc<AtomicBool>,String>{
    if key.is_empty() || key.len()>160{return Err("Неверный идентификатор подготовки.".into());}
    let mut map=preparations().lock().map_err(|_|"Preparation lock failed.")?;
    if map.len()>=128{map.retain(|_,value|Arc::strong_count(value)>1);}
    if map.len()>=128{return Err("Дождитесь окончания текущей подготовки.".into());}
    Ok(Arc::clone(map.entry(key.into()).or_insert_with(||Arc::new(AtomicBool::new(false)))))
}
#[tauri::command]
pub async fn cancel_processing_preparation(app:tauri::AppHandle,request_key:String)->Result<(),String>{
    preparation_token(&request_key)?.store(true,Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
        let mut db=connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let tasks=queue::cancel_receipt(&tx,&request_key)?;
        for(id,cancel) in &core.cancellations{let task:String=tx.query_row("SELECT task_id FROM processing_attempts WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;if tasks.contains(&task){let _=cancel.send(true);}}
        tx.commit().map_err(|e|e.to_string())?;notify_change(&app);Ok(())
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn remove_processing_work(app:tauri::AppHandle,scope:String,job_id:Option<String>,task_id:Option<String>,queue_mode:Option<String>)->Result<Value,String>{
    let owner=app.clone();let(count,attempts)=tauri::async_runtime::spawn_blocking(move||{
        let runtime=owner.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
        let mut db=connection(&owner)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let mut result=queue::remove_scoped(&tx,&scope,job_id.as_deref(),task_id.as_deref(),queue_mode.as_deref())?;
        tx.commit().map_err(|e|e.to_string())?;
        if scope=="allWorking"&&queue_mode.as_deref()!=Some("direct"){core.run=None;}
        result.1.retain(|id|{if let Some(cancel)=core.cancellations.get(id){let _=cancel.send(true);false}else{true}});
        notify_change(&owner);Ok::<_,String>(result)
    }).await.map_err(|e|e.to_string())??;
    // Cancellation addresses only the captured old attempts, never a reused
    // task ID or an independent manual job containing the same source photo.
    for id in attempts{let app=app.clone();tauri::async_runtime::spawn(async move{if let Ok(work)=connection(&app).and_then(|db|queue::attempt_work(&db,&id)){let _=provider_transport::cancel_attempt(&app,&work,&id).await;}});}
    Ok(json!({"removedCount":count}))
}

pub const SCHEMA: &str = r#"
    CREATE TABLE processing_jobs (
        id TEXT PRIMARY KEY, request_key TEXT NOT NULL UNIQUE, config TEXT NOT NULL,
        status TEXT NOT NULL, generation INTEGER NOT NULL DEFAULT 0,
        credential_ref TEXT, requested_concurrency INTEGER NOT NULL DEFAULT 1,
        created_at_unix INTEGER NOT NULL, updated_at_unix INTEGER NOT NULL, error TEXT
    );
    CREATE TABLE processing_tasks (
        id TEXT PRIMARY KEY, job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
        image_id TEXT NOT NULL REFERENCES images(id), source_path TEXT NOT NULL, sha256 TEXT NOT NULL,
        snapshot TEXT NOT NULL, status TEXT NOT NULL, completed_passes INTEGER NOT NULL DEFAULT 0,
        result TEXT, metadata_committed INTEGER NOT NULL DEFAULT 0, rename_plan_id TEXT,
        rename_status TEXT NOT NULL DEFAULT 'none', error TEXT, retry_at_unix INTEGER,
        UNIQUE(job_id, source_path)
    );
    CREATE INDEX idx_processing_tasks_job ON processing_tasks(job_id, status);
    CREATE TABLE processing_passes (
        task_id TEXT NOT NULL REFERENCES processing_tasks(id) ON DELETE CASCADE,
        pass_number INTEGER NOT NULL, result TEXT NOT NULL, attempt_id TEXT NOT NULL UNIQUE,
        PRIMARY KEY(task_id, pass_number)
    );
    CREATE TABLE processing_attempts (
        id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES processing_tasks(id) ON DELETE CASCADE,
        pass_number INTEGER NOT NULL, generation INTEGER NOT NULL, status TEXT NOT NULL,
        started_at_unix INTEGER NOT NULL, finished_at_unix INTEGER,
        reserved_micros INTEGER NOT NULL, cost_micros INTEGER, usage TEXT,
        remote_id TEXT, status_url TEXT, response_url TEXT, cancel_url TEXT,
        response TEXT, error TEXT
    );
    CREATE INDEX idx_processing_attempts_task ON processing_attempts(task_id, pass_number);
    UPDATE tasks SET status='interrupted' WHERE status IN ('pending','running','queued');
"#;

#[derive(Default)]
pub struct ProcessingRuntime { core: Mutex<RuntimeCore>,wake:tokio::sync::Notify }
fn notify_change(app:&tauri::AppHandle){
    static DIRTY:AtomicBool=AtomicBool::new(false);static RUNNING:AtomicBool=AtomicBool::new(false);
    DIRTY.store(true,Ordering::SeqCst);
    if RUNNING.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_ok(){
        let app=app.clone();tauri::async_runtime::spawn_blocking(move||loop{
            DIRTY.store(false,Ordering::SeqCst);
            match connection(&app).and_then(|db|queue::refresh_image_statuses(&db)){
                Ok(changes) if !changes.is_empty()=>{let _=app.emit("processing-statuses",&changes);},
                Err(error)=>{let _=app.emit("processing-error",json!({"message":error}));},_=>{}
            }
            if DIRTY.load(Ordering::SeqCst){continue;}
            RUNNING.store(false,Ordering::SeqCst);
            if !DIRTY.load(Ordering::SeqCst)||RUNNING.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){break;}
        });
    }
    app.state::<ProcessingRuntime>().wake.notify_one();crate::photo_editor::jobs::wake(app);let _=app.emit("processing-changed",json!({}));
}
/// The editor and recognition acquire the same fence for durable cloud admission.
/// The closure must be synchronous; this lock is never held across HTTP.
pub(super) fn cloud_fence<T>(app:&tauri::AppHandle,f:impl FnOnce()->Result<T,String>)->Result<T,String>{let runtime=app.state::<ProcessingRuntime>();let _gate=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;f()}
pub(super) fn file_action_fence<T>(app:&tauri::AppHandle,paths:&[String],f:impl FnOnce()->Result<T,String>)->Result<T,String>{
 let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
 crate::file_actions::metadata::check_recognition_busy(&connection(app)?,paths,&core.leased)?;
 f()
}
fn settle_remote_failure_db(db:&mut Connection,attempt:&str,message:&str)->Result<(),String>{
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let task:String=tx.query_row("SELECT task_id FROM processing_attempts WHERE id=?1 AND remote_id IS NOT NULL",[attempt],|r|r.get(0)).map_err(|e|e.to_string())?;
 let changed=tx.execute("UPDATE processing_attempts SET status='remoteFailed',error=?2,finished_at_unix=?3 WHERE id=?1 AND status IN ('unknown','remotePending','running')",params![attempt,message,unix_now()]).map_err(|e|e.to_string())?;
 if changed>0{tx.execute("UPDATE processing_tasks SET status='failed',error=?2,retry_at_unix=NULL WHERE id=?1 AND work_visible=1 AND status NOT IN ('succeeded','cancelled') AND run_revision=(SELECT run_revision FROM processing_attempts WHERE id=?3)",params![task,message,attempt]).map_err(|e|e.to_string())?;}
 tx.commit().map_err(|e|e.to_string())?;Ok(())
}
pub(super) async fn settle_terminal_remote_failure(app:&tauri::AppHandle,attempt:&str,message:&str)->Result<(),String>{let app=app.clone();let attempt=attempt.to_string();let message=message.chars().filter(|c|!c.is_control()).take(1000).collect::<String>();tauri::async_runtime::spawn_blocking(move||settle_remote_failure_db(&mut connection(&app)?,&attempt,&message)).await.map_err(|e|e.to_string())?}

// Uncertain outcome retains its money and same-photo replay fence, not an
// indefinitely presumed execution slot. Known admitted requests remain counted.
fn occupied_cloud_db(db:&Connection,provider:&str)->Result<u32,String>{db.query_row("SELECT COUNT(*) FROM processing_attempt_context WHERE json_extract(effective_config,'$.providerMode')=?1 AND status IN ('reserved','running','remotePending')",[provider],|r|r.get(0)).map_err(|e|e.to_string())}
pub(super) fn occupied_cloud(app:&tauri::AppHandle,provider:&str)->Result<u32,String>{occupied_cloud_db(&connection(app)?,provider)}

pub(super) fn active_cloud_limit(app:&tauri::AppHandle,provider:&str)->Result<u32,String>{connection(app)?.query_row("SELECT COALESCE(MAX(CASE WHEN j.requested_concurrency=0 THEN COALESCE(json_extract(t.effective_config,'$.concurrency'),1) ELSE j.requested_concurrency END),0) FROM processing_jobs j JOIN processing_task_context t ON t.job_id=j.id WHERE json_extract(t.effective_config,'$.providerMode')=?1 AND t.work_visible=1 AND t.status NOT IN ('succeeded','cancelled','failed') AND j.status IN ('queued','running','paused','interrupted')",[provider],|r|r.get(0)).map_err(|e|e.to_string())}
#[derive(Default)]
struct RuntimeCore {
    leased: HashSet<String>, cancellations: HashMap<String, watch::Sender<bool>>,
    next_request: HashMap<String, Instant>,
    next_run:u64,run:Option<run::RunToken>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct JobConfig {
    #[serde(default)] pub queue_contract_version:u32,
    #[serde(default="yes")] pub only_unprocessed:bool,
    pub provider_mode: String, pub model: String,
    #[serde(default="default_output_mode")] pub output_mode:String,
    #[serde(default="one")] pub passes: u32,
    #[serde(default="one")] pub concurrency: u32,
    #[serde(default)] pub request_delay_ms: u64,
    #[serde(default="default_budget")] pub max_estimated_cost_usd: f64,
    #[serde(default)] pub estimated_cost_per_image_usd: f64,
    #[serde(default="default_retries")] pub max_retries_per_task: u32,
    #[serde(default)] pub stop_on_first_error: bool,
    #[serde(default)] pub rename_stop_on_first_error: bool,
    #[serde(default)] pub smart_rename_enabled: bool,
    #[serde(default)] pub source_roots: Vec<String>,
    #[serde(default)] pub source_query_id: Option<String>,
    #[serde(default)] pub excluded_source_paths: Vec<String>,
    #[serde(default)] pub recursive_processing: bool,
    #[serde(default)] pub analysis_copy: Option<profile_preferences::AnalysisCopy>,
    #[serde(default)] pub endpoint: Option<String>,
    #[serde(default)] pub prompt_version: String,
    #[serde(default)] pub analysis_contract: Option<Value>,
    #[serde(default="default_tokens")] pub max_tokens: u32,
    #[serde(default="default_side")] pub image_max_side: u32,
    #[serde(default)] pub fal_reasoning: bool,
    #[serde(default)] pub local_reasoning: bool,
}
fn one()->u32{1} fn yes()->bool{true} fn default_budget()->f64{0.0} fn default_retries()->u32{2}
fn default_tokens()->u32{3200} fn default_side()->u32{2560}
fn default_output_mode()->String{"both".into()}
fn metadata_fields(mode:&str)->Result<Vec<&'static str>,String>{match mode{"tags"=>Ok(vec!["tags"]),"description"=>Ok(vec!["description"]),"both"=>Ok(vec!["description","tags"]),_=>Err("Unknown frozen analysis output mode; metadata was not changed.".into())}}

#[cfg(test)]
mod output_mode_tests{
    use super::*;
    fn config(mode:Option<&str>,model:&str)->JobConfig{
        let mut value=json!({"providerMode":"local","model":model});
        if let Some(mode)=mode{value["outputMode"]=json!(mode);}
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn older_jobs_default_to_both_and_single_field_updates_preserve_the_other_field(){
        assert_eq!(config(None,"qwen3.5:2b").output_mode,"both");
        assert_eq!(metadata_fields("tags").unwrap(),vec!["tags"]);
        assert_eq!(metadata_fields("description").unwrap(),vec!["description"]);
        assert_eq!(metadata_fields("both").unwrap(),vec!["description","tags"]);
        assert!(metadata_fields("unknown").is_err());
        assert!(validate_config(&mut config(Some("unknown"),"qwen3.5:2b")).is_err());
    }
    #[test]
    fn output_modes_freeze_the_correct_prompt_limits_and_local_reasoning(){
        let mut detailed=config(None,"qwen3.5:2b");validate_config(&mut detailed).unwrap();
        assert_eq!(detailed.prompt_version,"archive-detailed-v3");assert_eq!(detailed.max_tokens,3200);assert!(!detailed.local_reasoning);
        let mut tags=config(Some("tags"),"qwen3.5:2b");validate_config(&mut tags).unwrap();
        assert_eq!(tags.prompt_version,"archive-quick-tags-v1");assert_eq!(tags.max_tokens,640);assert_eq!(tags.image_max_side,768);assert!(!tags.local_reasoning);
        let mut fast=config(Some("description"),"qwen3-vl:2b-instruct-q4_K_M");validate_config(&mut fast).unwrap();
        assert_eq!(fast.prompt_version,"archive-fast-summary-v1");assert_eq!(fast.max_tokens,1024);assert_eq!(fast.image_max_side,1024);assert!(!fast.local_reasoning);
    }
    #[test]
    fn local_reasoning_is_opt_in_for_both_qwen35_models(){
        for model in ["qwen3.5:2b","qwen3.5:4b"]{
            for mode in ["both","description","tags"]{
                let mut fresh=config(Some(mode),model);
                validate_config(&mut fresh).unwrap();
                assert!(!fresh.local_reasoning,"{model}/{mode} must default to a direct answer");
                let mut explicit=config(Some(mode),model);
                explicit.local_reasoning=true;
                validate_config(&mut explicit).unwrap();
                assert_eq!(explicit.local_reasoning,mode!="tags","explicit historical reasoning remains compatible; tags stay direct");
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PrepareJobInput {
    request_key: String, config: JobConfig, credential_ref: Option<String>, tasks: Vec<Value>,
    #[serde(default)] target_job_id:Option<String>,
    #[serde(default="manual_mode")] queue_mode:String,
}
fn manual_mode()->String{"manual".into()}

fn connection(app:&tauri::AppHandle)->Result<Connection,String>{
    let mut db=open_index_connection(app)?; run_index_migrations(&mut db)?; Ok(db)
}
fn test_transport(config:&JobConfig)->bool{
    (config.provider_mode=="controlled-test" || config.provider_mode=="fal" && std::env::var("SMART_PHOTO_EDITOR_TEST_TRANSPORT").as_deref()==Ok("1")) && std::env::var("SMART_PHOTO_TEST_TRANSPORT").as_deref()==Ok("1")
        && std::env::var_os("SMART_PHOTO_DATA_DIR").is_some()
}
fn validate_config(c:&mut JobConfig)->Result<(),String>{
    if !["both","tags","description"].contains(&c.output_mode.as_str()){return Err("Unknown analysis output mode.".into());}
    let mut fast=false;
    let quick=c.output_mode=="tags" || c.provider_mode=="local"&&provider_transport::model_catalog()["local"].as_array().is_some_and(|models|models.iter().any(|model|model["id"]==c.model&&model["fastLocal"]==true));
    let supported_reasoning=c.provider_mode=="local" && provider_transport::model_catalog()["local"].as_array().is_some_and(|models|models.iter().any(|model|model["id"]==c.model && model["localReasoning"]==true));
    if c.local_reasoning && !supported_reasoning && !(c.provider_mode=="local" && std::env::var("SMART_PHOTO_LOCAL_EVAL").as_deref()==Ok("1") && std::env::var_os("SMART_PHOTO_DATA_DIR").is_some()){
        return Err("Experimental local reasoning requires an isolated evaluation profile.".into());
    }
    if !(1..=5).contains(&c.passes) || !(1..=100).contains(&c.concurrency) || c.max_retries_per_task>5
        || c.request_delay_ms>60000 || !c.max_estimated_cost_usd.is_finite()
        || c.max_estimated_cost_usd<0.0 || !c.estimated_cost_per_image_usd.is_finite()
        || c.estimated_cost_per_image_usd<0.0 || !((if quick{128}else{1600})..=4096).contains(&c.max_tokens)
        || !((if quick{256}else{1024})..=4096).contains(&c.image_max_side) { return Err("Invalid processing limits.".into()); }
    if !["local","openrouter","fal"].contains(&c.provider_mode.as_str()) && !test_transport(c) {
        return Err("Unknown processing provider.".into());
    }
    if !test_transport(c){
        let model=provider_transport::selected_model(&c.provider_mode,&c.model)?;
        c.fal_reasoning=c.provider_mode=="fal" && model["requiresReasoning"].as_bool().unwrap_or(false);
        if c.provider_mode=="local"{
            c.concurrency=1;fast=model["fastLocal"]==true;
            // Catalog reasoning is a capability, not a request to enable it.
            // Keep explicit legacy configs readable; new jobs default to false.
            c.local_reasoning=c.local_reasoning && !fast && c.output_mode!="tags";
            provider_transport::local_endpoint(c.endpoint.as_deref())?;c.estimated_cost_per_image_usd=0.0;
        }
        else {
            // Reserve a conservative input/output envelope per pass, including N's final candidates.
            let input=16000.0+9000.0*f64::from(c.passes.saturating_sub(1));
            let tokens=if c.output_mode=="tags"{c.max_tokens.min(640)}else{c.max_tokens};
            let estimate=(input*model["inputUsdPerMillion"].as_f64().ok_or("Missing input pricing.")?+f64::from(tokens)*model["outputUsdPerMillion"].as_f64().ok_or("Missing output pricing.")?)/1_000_000.0;
            c.estimated_cost_per_image_usd=c.estimated_cost_per_image_usd.max(estimate);
        }
    }
    // Stage 1B enables one worker. The stored ceiling already supports the later bounded pool.
    let contract=if c.output_mode=="tags"{c.max_tokens=c.max_tokens.min(640);c.image_max_side=c.image_max_side.min(768);provider_transport::quick_tags_contract()}
        else if fast{c.max_tokens=c.max_tokens.min(1024);c.image_max_side=c.image_max_side.min(1024);provider_transport::fast_summary_contract()}
        else{provider_transport::analysis_contract()};
    c.prompt_version=contract["version"].as_str().ok_or("Invalid analysis contract version.")?.into();
    c.analysis_contract=Some(contract.clone());
    Ok(())
}

#[tauri::command]
pub async fn prepare_processing_job(app:tauri::AppHandle, mut input:PrepareJobInput)->Result<Value,String>{
    crate::provider_availability::require(&input.config.provider_mode)?;
    input.config.queue_contract_version=2;
    validate_config(&mut input.config)?;
    let token=preparation_token(&input.request_key)?;
    let guard=PreparationGuard{key:input.request_key.clone(),cancel:Arc::clone(&token)};
    if !test_transport(&input.config){
        let ready=provider_transport::ensure_ai_provider_ready(app.clone(),input.config.provider_mode.clone(),input.config.model.clone(),input.config.endpoint.clone(),input.credential_ref.clone()).await?;
        if ready["ready"]!=true{return Err(ready["message"].as_str().unwrap_or("Provider is not ready.").into());}
    }
    tauri::async_runtime::spawn_blocking(move||{
        let _guard=guard;
        if token.load(Ordering::SeqCst){return Err("Подготовка отменена.".into());}
        prepare_streamed_job(&app,&input,&token)

    }).await.map_err(|e|e.to_string())?
}

fn prepare_streamed_job(app:&tauri::AppHandle,input:&PrepareJobInput,token:&AtomicBool)->Result<Value,String>{
 if input.request_key.trim().is_empty()||input.request_key.len()>160||input.tasks.is_empty()&&input.config.source_roots.is_empty()&&input.config.source_query_id.is_none(){return Err("Choose processing sources first.".into());}
 if input.config.analysis_copy.as_ref().is_some_and(|copy|copy.enabled)&&!input.config.smart_rename_enabled{return Err("Copies with smart names require smart naming.".into());}
 let(id,receipt,duplicate)={
  let runtime=app.state::<ProcessingRuntime>();let _gate=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
  if index_backups::restore_blocked(app)?{return Err("Finish restoring the index before preparing another job.".into());}
  queue::begin_target(&mut connection(app)?,&input.request_key,&input.queue_mode,&input.config,input.credential_ref.as_deref(),input.target_job_id.as_deref())?
 };
 if let Some(mut duplicate)=duplicate{duplicate["status"]=json!(connection(app)?.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?);return Ok(duplicate);}
 let result=(||{
  let db=connection(app)?;let accepted=std::cell::Cell::new(0u64);let skipped=std::cell::Cell::new(0u64);let ready=std::cell::Cell::new(0u64);let duplicates=std::cell::Cell::new(0u64);
  let issue=|path:&Path,error:&str|->Result<(),String>{skipped.set(skipped.get()+1);db.execute("INSERT INTO action_logs(action_type,message,new_value,created_at_unix) VALUES('processing_source_skipped',?1,?2,?3)",params![error,json!({"jobId":id,"path":path_to_string(path)}).to_string(),unix_now()]).map_err(|e|e.to_string())?;Ok(())};
  let accept=|raw:&Path,original:Option<&Value>|->Result<(),String>{
   if token.load(Ordering::SeqCst){return Err("Preparation cancelled.".into());}
   let prepared=(||{
    source_paths::require_photo_source(raw)?;let canonical=fs::canonicalize(raw).map_err(|e|e.to_string())?;source_paths::require_photo_source(&canonical)?;let path=path_to_string(&canonical);
    let stat=fs::metadata(&canonical).map_err(|e|e.to_string())?;
    let sync=sync_scanned_images_to_index_internal(app.clone(),vec![IndexImageInputDto{path:path.clone(),file_name:file_name(&canonical),extension:extension(&canonical),size_bytes:Some(stat.len()),modified_at_unix:modified_at_unix(&stat),width:None,height:None}])?;
    let image=sync.indexed_images.first().ok_or("Source indexing failed.")?;
    let hash=read_indexed_sha256(&db,&image.image_id)?.ok_or("Missing indexed source hash.")?;
    let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
    if token.load(Ordering::SeqCst){return Err("Preparation cancelled.".into());}
    let existing=queue::existing_direct(&db,&id,&image.image_id,&path)?;
    if let Some((task_id,status,uncertain))=&existing{
     if !queue::reusable(status,*uncertain,core.leased.contains(task_id)){duplicates.set(duplicates.get()+1);return Ok(());}
    }
    let mut task=original.cloned().unwrap_or_else(||json!({"targetPath":path}));
    let Some(effective)=fields::prepare(&db,&image.image_id,&hash,&mut task,&input.config)? else{ready.set(ready.get()+1);return Ok(());};
    queue::freeze(&mut task,&effective,input.credential_ref.as_deref(),&receipt)?;
    if !input.config.smart_rename_enabled{task["renamePlanId"]=Value::Null;}
    if input.config.smart_rename_enabled&&task["renamePlanId"].as_str().is_none(){
     let mut preview=preview_smart_rename_image_internal(&SmartRenameInputDto{image_id:Some(image.image_id.clone()),current_path:path.clone(),file_name:file_name(&canonical),modified_at_unix:modified_at_unix(&stat),pattern:"date_unique8".into(),plan_id:None});
     if preview.status=="ready"||preview.status=="unchanged"&&input.config.analysis_copy.as_ref().is_some_and(|copy|copy.enabled){task["renamePlanId"]=json!(if let Some(copy)=input.config.analysis_copy.as_ref().filter(|copy|copy.enabled){analysis_copy::prepare(&db,&mut preview,copy)?}else{file_operations::prepare_rename_plan(&db,&preview)?});}
     else if preview.status!="unchanged"{return Err(preview.message);}
    }
    let plan=task["renamePlanId"].as_str();let task_id=existing.as_ref().map(|row|row.0.clone()).unwrap_or_else(||stable_text_id("task",&format!("{id}|{path}")));
    let tx=rusqlite::Transaction::new_unchecked(&db,rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    if let Some(plan)=plan{validate_file_plan(&tx,&input.config,plan,&image.image_id,&path,&hash,true)?;}
    queue::claim(&tx,&task_id,&id,&image.image_id,&path,&hash,&task,plan,existing.is_some())?;tx.commit().map_err(|e|e.to_string())?;
    accepted.set(accepted.get()+1);if accepted.get()%128==0{let _=app.emit("processing-preparation-progress",json!({"requestKey":input.request_key,"accepted":accepted.get(),"skipped":skipped.get()}));}Ok(())
   })();
   if let Err(error)=prepared{if token.load(Ordering::SeqCst){return Err(error);}issue(raw,&error)?;}Ok(())
  };
  if let Some(query_id)=&input.config.source_query_id{
   // Persist the frozen list before any slow file work, then release the search snapshot.
   let mut writer=connection(app)?;let mut ordinal=0i64;
   search::materialize_processing_paths(app,query_id,&input.config.excluded_source_paths,token,|paths|{let tx=writer.transaction().map_err(|e|e.to_string())?;for path in paths{tx.execute("INSERT OR IGNORE INTO processing_sources(job_id,command_id,ordinal,path) VALUES(?1,?2,?3,?4)",params![id,receipt,ordinal,path]).map_err(|e|e.to_string())?;ordinal+=1;}tx.commit().map_err(|e|e.to_string())})?;
   let mut after=-1i64;loop{let batch:Vec<(i64,String)>={let mut q=db.prepare("SELECT ordinal,path FROM processing_sources WHERE job_id=?1 AND command_id=?2 AND ordinal>?3 ORDER BY ordinal LIMIT 128").map_err(|e|e.to_string())?;let r=q.query_map(params![id,receipt,after],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;r.collect::<Result<_,_>>().map_err(|e|e.to_string())?};if batch.is_empty(){break;}for(ordinal,path)in batch{accept(Path::new(&path),None)?;after=ordinal;}}
  }
  for task in &input.tasks{let path=task["targetPath"].as_str().ok_or("Task source path is required")?;if is_supported_image(Path::new(path)){accept(Path::new(path),Some(task))?;}}
  processing_sources::visit(&input.config.source_roots,input.config.recursive_processing,token,|path|accept(path,None),&issue)?;
  if token.load(Ordering::SeqCst){return Err("Preparation cancelled.".into());}
  if accepted.get()==0&&ready.get()==0&&duplicates.get()==0{return Err("No available photos were found. See skipped sources in the journal.".into());}
  let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
  if token.load(Ordering::SeqCst){return Err("Preparation cancelled.".into());}
  queue::finish(&db,&id,&receipt,&input.queue_mode,accepted.get(),ready.get(),skipped.get(),duplicates.get())?;
  if input.queue_mode=="manual"&&accepted.get()>0&&core.run.is_some(){db.execute("UPDATE processing_jobs SET status='queued' WHERE id=?1 AND status IN ('prepared','paused','interrupted','completed')",[&id]).map_err(|e|e.to_string())?;}
  Ok(json!({"jobId":id,"duplicate":false,"status":db.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?,"queueMode":input.queue_mode,"totalCount":accepted.get(),"readyCount":ready.get(),"duplicateCount":duplicates.get(),"skippedCount":skipped.get(),"alreadyReady":accepted.get()==0&&duplicates.get()==0,"message":if accepted.get()==0&&duplicates.get()==0{"Все выбранные фотографии уже обработаны для этого результата"}else{""}}))
 })();
 if let Err(error)=&result{
  let runtime=app.state::<ProcessingRuntime>();let _gate=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
  let mut db=connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
  queue::cancel_receipt(&tx,&input.request_key)?;
  tx.execute("UPDATE processing_commands SET status='failed',error=?2 WHERE id=?1",params![receipt,error]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;
 }

 notify_change(app);result
}

#[tauri::command]
pub async fn processing_job_command(app:tauri::AppHandle, job_id:String, action:String,request_key:Option<String>)->Result<Value,String>{
    job_command(app,job_id,action,request_key,None).await
}
async fn job_command(app:tauri::AppHandle,job_id:String,action:String,request_key:Option<String>,run_token:Option<u64>)->Result<Value,String>{
    if ["start","resume","retry"].contains(&action.as_str()) {
        let lookup_app=app.clone();let lookup_job=job_id.clone();
        let receipt=request_key.as_deref().map(|key|stable_text_id("processing-command",key));let retry=action=="retry";
        let contexts=tauri::async_runtime::spawn_blocking(move||{
            let db=connection(&lookup_app)?;
            queue::ready_contexts(&db,&lookup_job,receipt.as_deref(),retry)
        }).await.map_err(|e|e.to_string())??;
        for(config,credential_ref) in contexts{if !test_transport(&config) {
            let ready=provider_transport::ensure_ai_provider_ready(app.clone(),config.provider_mode,config.model,config.endpoint,credential_ref).await?;
            if ready["ready"]!=true{return Err(ready["message"].as_str().unwrap_or("Закреплённый провайдер задания не готов.").into());}
        }}
    }
    let cancel_app=app.clone();let cancel_job=job_id.clone();let should_cancel=action=="stop";
    let result=tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<ProcessingRuntime>(); let core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        if run_token.is_some_and(|token|core.next_run!=token||(["start","resume"].contains(&action.as_str())&&!core.run.is_some_and(|run|run.id==token))){return Err("Общая команда отменена.".into());}
        let mut db=connection(&app)?; let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let (state,raw,generation,mode):(String,String,i64,String)=tx.query_row("SELECT status,config,generation,queue_mode FROM processing_jobs WHERE id=?1",[&job_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
        let _cfg:JobConfig=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let visible:bool=tx.query_row("SELECT work_visible FROM processing_jobs WHERE id=?1",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !visible&&["start","resume","retry"].contains(&action.as_str()){return Err("Это задание убрано из рабочей очереди. Создайте новую команду обработки.".into());}
        if let Some(key)=request_key.as_deref(){
            let accepted:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM processing_commands WHERE job_id=?1 AND request_key=?2 AND status IN ('prepared','started','completed'))",params![job_id,key],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !accepted{return Err("Preparation cancelled or not accepted.".into());}
        }
        let next=match action.as_str(){
            "start"|"resume"=>{
                if state=="completed"&&tx.query_row("SELECT NOT EXISTS(SELECT 1 FROM processing_tasks WHERE job_id=?1)",[&job_id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return Ok(json!({"jobId":job_id,"status":"completed","alreadyReady":true}));}
                if !["prepared","paused","interrupted","waitingService","queued","running"].contains(&state.as_str())&&!(mode=="direct"&&["completed","stopped"].contains(&state.as_str())){return Err("This job cannot be started.".into());}
                // Only a receipt with a known remote ID can be resumed. Other
                // photos remain runnable while an unknown outcome is audited.
                tx.execute("UPDATE processing_tasks SET status='pending' WHERE job_id=?1 AND work_visible=1 AND status='waitingService' AND EXISTS(SELECT 1 FROM processing_attempts a WHERE a.task_id=processing_tasks.id AND a.run_revision=processing_tasks.run_revision AND a.remote_id IS NOT NULL AND a.status IN ('unknown','remotePending'))",[&job_id]).map_err(|e|e.to_string())?;
                let other:i64=tx.query_row("SELECT COUNT(*) FROM processing_jobs WHERE id<>?1 AND status='running'",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
                if other>0{"queued"}else{"running"}
            },
            "pause" if ["prepared","interrupted","running","paused","queued","waitingService"].contains(&state.as_str())=>"paused",
            "retry" if ["paused","completed"].contains(&state.as_str())=>{
                if !core.leased.is_empty(){return Err("Wait for admitted responses to finish before retrying failed tasks.".into());}
                let unknown:i64=tx.query_row("SELECT COUNT(*) FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE t.job_id=?1 AND a.status IN ('unknown','remotePending')",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
                if unknown>0{return Err("Unresolved requests must be reconciled before an explicit retry.".into());}
                tx.execute("UPDATE processing_jobs SET generation=generation+1 WHERE id=?1",[&job_id]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE processing_tasks SET status='pending',error=NULL,retry_at_unix=NULL,rename_status=CASE WHEN rename_status='failed' THEN 'pending' ELSE rename_status END WHERE job_id=?1 AND status='failed'",[&job_id]).map_err(|e|e.to_string())?;
                "paused"
            },
            "stop"=>{
                tx.execute("UPDATE processing_jobs SET generation=generation+1 WHERE id=?1",[&job_id]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE processing_tasks SET status='cancelled' WHERE job_id=?1 AND status NOT IN ('succeeded','failed')",[&job_id]).map_err(|e|e.to_string())?;
                for (id,cancel) in &core.cancellations {
                    let belongs:i64=tx.query_row("SELECT COUNT(*) FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE a.id=?1 AND t.job_id=?2",params![id,job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
                    if belongs>0{let _=cancel.send(true);}
                }
                "stopped"
            },
            _=>return Err("Invalid job command or state.".into()),
        };
        tx.execute("UPDATE processing_jobs SET status=?2,updated_at_unix=?3 WHERE id=?1",params![job_id,next,unix_now()]).map_err(|e|e.to_string())?;
        if ["start","resume"].contains(&action.as_str()){if let Some(key)=request_key.as_deref(){tx.execute("UPDATE processing_commands SET status='started',updated_at_unix=?2 WHERE request_key=?1 AND status='prepared'",params![key,unix_now()]).map_err(|e|e.to_string())?;}}
        tx.commit().map_err(|e|e.to_string())?;notify_change(&app);
        Ok(json!({"jobId":job_id,"status":next,"generation":generation+i64::from(action=="stop" || action=="retry")}))
    }).await.map_err(|e|e.to_string())?;
    if should_cancel && result.is_ok(){tauri::async_runtime::spawn(async move{let _=cancel_known_job_attempts(&cancel_app,&cancel_job).await;});}
    result
}

#[tauri::command]
pub async fn set_processing_credential(app:tauri::AppHandle,job_id:String,credential_ref:String,provider:Option<String>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;
        let raw:String=db.query_row("SELECT config FROM processing_jobs WHERE id=?1",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        let config:JobConfig=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let provider=provider.unwrap_or(config.provider_mode);
        if !["fal","openrouter"].contains(&provider.as_str()){return Err("Локальному заданию ключ не нужен.".into());}
        let _key=credentials::unlock(&app,&provider,&credential_ref)?;drop(db);
        let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        let mut db=connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let state:String=tx.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !["prepared","paused","interrupted","stopped","completed"].contains(&state.as_str()){return Err("Сначала приостановите задание.".into());}
        for task in &core.leased {
            let belongs:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks WHERE id=?1 AND job_id=?2)",params![task,job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if belongs{return Err("Дождитесь завершения допущенных запросов перед заменой ключа.".into());}
        }
        let changed=queue::replace_key(&tx,&job_id,&provider,&credential_ref)?;
        if changed==0{return Err("Нет незавершённых фотографий этого сервиса для замены ключа.".into());}
        tx.execute("UPDATE processing_jobs SET updated_at_unix=?2 WHERE id=?1",params![job_id,unix_now()]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO action_logs(action_type,message,created_at_unix) VALUES('processing_credential_replaced',?1,?2)",params![format!("Заменена ссылка на ключ задания {job_id}; провайдер/модель/бюджет не изменены"),unix_now()]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;notify_change(&app);Ok(json!({"jobId":job_id,"updated":true}))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn get_unresolved_processing_attempts(app:tauri::AppHandle,job_id:String,offset:Option<u32>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;let offset=offset.unwrap_or(0);
        let total:u32=db.query_row("SELECT COUNT(*) FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE t.job_id=?1 AND a.status IN ('unknown','remotePending')",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        let mut statement=db.prepare("SELECT a.id,t.source_path,a.pass_number,a.status,a.remote_id,a.reserved_micros,a.cost_micros,a.error FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE t.job_id=?1 AND a.status IN ('unknown','remotePending') ORDER BY a.rowid LIMIT 50 OFFSET ?2").map_err(|e|e.to_string())?;
        let rows=statement.query_map(params![job_id,offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"path":r.get::<_,String>(1)?,"passNumber":r.get::<_,u32>(2)?,"status":r.get::<_,String>(3)?,"remoteId":r.get::<_,Option<String>>(4)?,"reservedUsd":r.get::<_,i64>(5)? as f64/1_000_000.0,"costUsd":r.get::<_,Option<i64>>(6)?.map(|v|v as f64/1_000_000.0),"error":r.get::<_,Option<String>>(7)?}))).map_err(|e|e.to_string())?;
        let items=rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;let next=offset.saturating_add(items.len() as u32);
        Ok(json!({"items":items,"totalCount":total,"nextOffset":if next<total{Some(next)}else{None}}))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn resolve_unknown_processing_attempt(app:tauri::AppHandle,job_id:String,attempt_id:String,confirmed:bool,known_cost_usd:Option<f64>,reason:String)->Result<Value,String>{
    if !confirmed || reason.trim().chars().count()<10 || reason.len()>2000 || reason.contains("fal_sk_") || reason.contains("sk-or-")
        || known_cost_usd.is_some_and(|cost|!cost.is_finite() || !(0.0..=10000.0).contains(&cost)) {
        return Err("Нужно подтвердить проверку в кабинете сервиса, указать пояснение и корректную стоимость либо оставить её неизвестной.".into());
    }
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        let mut db=connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let (state,provider):(String,String)=tx.query_row("SELECT j.status,json_extract(a.effective_config,'$.providerMode') FROM processing_jobs j JOIN processing_attempt_context a ON a.job_id=j.id WHERE j.id=?1 AND a.id=?2",params![job_id,attempt_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        if !["paused","interrupted","stopped","completed"].contains(&state.as_str()) || !["fal","openrouter"].contains(&provider.as_str()) {return Err("Сначала остановите или приостановите облачное задание.".into());}
        for task in &core.leased {
            if tx.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks WHERE id=?1 AND job_id=?2)",params![task,job_id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return Err("Дождитесь завершения допущенных запросов.".into());}
        }
        let cost=known_cost_usd.map(|cost|(cost*1_000_000.0).ceil() as i64);
        let changed=tx.execute("UPDATE processing_attempts SET status='manualResolved',cost_micros=?3,error=?4,finished_at_unix=?5 WHERE id=?1 AND status='unknown' AND remote_id IS NULL AND task_id IN (SELECT id FROM processing_tasks WHERE job_id=?2)",params![attempt_id,job_id,cost,format!("Ручная сверка: {}. Новый запрос разрешён только по явному продолжению задания.",reason.trim()),unix_now()]).map_err(|e|e.to_string())?;
        if changed!=1{return Err("Исход уже изменился, попытка принадлежит другому заданию или имеет remote ID: используйте сверку сервиса.".into());}
        tx.execute("INSERT INTO action_logs(action_type,message,new_value,created_at_unix) VALUES('provider_attempt_manually_resolved',?1,?2,?3)",params![format!("Задание {job_id}; попытка {attempt_id}"),json!({"reason":reason.trim(),"knownCostUsd":known_cost_usd,"unknownReserveRetained":cost.is_none()}).to_string(),unix_now()]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;notify_change(&app);Ok(json!({"resolved":true,"reserveRetained":cost.is_none(),"automaticSubmit":false}))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn reconcile_processing_attempts(app:tauri::AppHandle,job_id:String)->Result<Value,String>{
    static RECONCILE:std::sync::OnceLock<tokio::sync::Semaphore>=std::sync::OnceLock::new();
    let _permit=RECONCILE.get_or_init(||tokio::sync::Semaphore::new(1)).try_acquire().map_err(|_|"Сверка уже выполняется.".to_string())?;
    let query_app=app.clone();
    let attempts=tauri::async_runtime::spawn_blocking(move||{
        let runtime=query_app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        let db=connection(&query_app)?;
        let status:String=db.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if ["running","queued","prepared"].contains(&status.as_str()){return Err("Сначала приостановите задание и дождитесь ответов.".into());}
        for task in &core.leased{let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks WHERE id=?1 AND job_id=?2)",params![task,job_id],|r|r.get(0)).map_err(|e|e.to_string())?;if belongs{return Err("Дождитесь завершения допущенных запросов.".into());}}
        let mut stmt=db.prepare("SELECT id,pass_number FROM processing_attempt_context WHERE job_id=?1 AND remote_id IS NOT NULL AND status IN ('remotePending','unknown') ORDER BY attempt_ordinal LIMIT 8").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([job_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?))).map_err(|e|e.to_string())?;
        rows.map(|r|{let(id,pass)=r.map_err(|e|e.to_string())?;let work=queue::attempt_work(&db,&id)?;Ok((id,pass,work))}).collect::<Result<Vec<_>,String>>()
    }).await.map_err(|e|e.to_string())??;
    let mut checked=0;let mut resolved=0;let mut errors=Vec::new();
    for(id,pass,work) in attempts{
        checked+=1;
        match provider_transport::reconcile_attempt(&app,&work,&id).await{
            Ok(Some(response))=>{
                let save_app=app.clone();let id=id.clone();
                tauri::async_runtime::spawn_blocking(move||save_attempt(&save_app,&work,&id,pass,response)).await.map_err(|e|e.to_string())??;
                resolved+=1;
            },
            Ok(None)=>{let db=connection(&app)?;let terminal:bool=db.query_row("SELECT status='remoteFailed' FROM processing_attempts WHERE id=?1",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;if terminal{resolved+=1;}},Err(error)=>errors.push(error),
        }
    }
    Ok(json!({"checked":checked,"resolved":resolved,"errors":errors}))
}

#[tauri::command]
pub async fn set_processing_concurrency(app:tauri::AppHandle,job_id:String,requested:u32)->Result<Value,String>{
    if !(1..=100).contains(&requested){return Err("Предел одновременной обработки должен быть от 1 до 100.".into());}
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<ProcessingRuntime>();let _core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        let mut db=connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let (raw,status):(String,String)=tx.query_row("SELECT config,status FROM processing_jobs WHERE id=?1",[&job_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        if ["completed","stopped"].contains(&status.as_str()){return Err("Это задание уже завершено.".into());}
        let config:JobConfig=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        if config.provider_mode=="local" && requested!=1{return Err("Локальные фото обрабатываются по одному.".into());}
        tx.execute("UPDATE processing_jobs SET requested_concurrency=?2,updated_at_unix=?3 WHERE id=?1",params![job_id,requested,unix_now()]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;notify_change(&app);
        // The original model/prompt/pass count remain frozen. Admitted work finishes;
        // lowering the limit prevents new leases until occupancy falls below it.
        Ok(json!({"jobId":job_id,"requestedConcurrency":requested,"effectiveConcurrency":effective_requested_concurrency(&config,requested)}))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn get_processing_jobs(app:tauri::AppHandle,offset:Option<u32>,working_only:Option<bool>,hide_finished:Option<bool>,queue_mode:Option<String>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;
        Ok(json!(queue::summaries_scoped(&db,offset.unwrap_or(0),working_only.unwrap_or(false),hide_finished.unwrap_or(false),queue_mode.as_deref())?))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn get_processing_tasks(app:tauri::AppHandle, job_id:String, offset:Option<u32>, limit:Option<u32>,include_excluded:Option<bool>,hide_finished:Option<bool>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;
        Ok(json!(queue::task_rows(&db,&job_id,offset.unwrap_or(0),limit.unwrap_or(100),include_excluded.unwrap_or(false),hide_finished.unwrap_or(false))?))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn get_processing_cost_statistics(app:tauri::AppHandle)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;
        let mut stmt=db.prepare("SELECT json_extract(a.effective_config,'$.providerMode'),COUNT(*),COALESCE(SUM(a.cost_micros),0),COALESCE(SUM(CASE WHEN a.cost_micros IS NULL THEN a.reserved_micros ELSE 0 END),0),COALESCE(SUM(a.cost_micros IS NULL AND COALESCE(json_extract(a.effective_config,'$.providerMode'),'')<>'local'),0) FROM processing_attempt_context a GROUP BY json_extract(a.effective_config,'$.providerMode')").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([],|r|Ok(json!({"provider":r.get::<_,String>(0)?,"attempts":r.get::<_,u64>(1)?,"spentUsd":r.get::<_,i64>(2)? as f64/1e6,"reservedUsd":r.get::<_,i64>(3)? as f64/1e6,"unknownCosts":r.get::<_,u64>(4)?}))).map_err(|e|e.to_string())?;
        let mut values=rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let editor=crate::photo_editor::store::connection(&app)?;
        let (count,spent,reserved,unknown):(u64,i64,i64,u64)=editor.query_row("SELECT COUNT(*),COALESCE(SUM(cost_micros),0),COALESCE(SUM(CASE WHEN cost_micros IS NULL THEN reserved_micros ELSE 0 END),0),COALESCE(SUM(cost_micros IS NULL),0) FROM editor_attempts",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
        if count>0{if let Some(row)=values.iter_mut().find(|v|v["provider"]=="fal"){row["attempts"]=json!(row["attempts"].as_u64().unwrap_or(0)+count);row["spentUsd"]=json!(row["spentUsd"].as_f64().unwrap_or(0.)+spent as f64/1e6);row["reservedUsd"]=json!(row["reservedUsd"].as_f64().unwrap_or(0.)+reserved as f64/1e6);row["unknownCosts"]=json!(row["unknownCosts"].as_u64().unwrap_or(0)+unknown);row["editorAttempts"]=json!(count);}else{values.push(json!({"provider":"fal","attempts":count,"editorAttempts":count,"spentUsd":spent as f64/1e6,"reservedUsd":reserved as f64/1e6,"unknownCosts":unknown}));}}
        Ok(json!(values))
    }).await.map_err(|e|e.to_string())?
}

pub struct CommitCheckpoint { pub job_id:String,pub task_id:String,pub generation:i64,pub run_revision:i64,pub sha256:String }

#[tauri::command]
pub async fn get_processing_snapshot(app:tauri::AppHandle,job_id:Option<String>,after_revision:Option<i64>,hide_finished:Option<bool>,queue_mode:Option<String>)->Result<Value,crate::ui_error::CommandError>{
    tauri::async_runtime::spawn_blocking(move||->Result<Value,crate::ui_error::CommandError>{
        let runtime=app.state::<ProcessingRuntime>();let run=runtime.core.lock().map_err(|_|"Coordinator lock failed")?.run;
        let mut db=connection(&app)?;let tx=db.transaction().map_err(|e|e.to_string())?;
        run::validate_scope(&tx,job_id.as_deref(),queue_mode.as_deref())?;
        let global=run::aggregate(&tx)?;let mut aggregate=run::aggregate_scoped(&tx,queue_mode.as_deref())?;aggregate["otherActive"]=json!(global["active"].as_u64().unwrap_or(0).saturating_sub(aggregate["active"].as_u64().unwrap_or(0))+if queue_mode.as_deref()==Some("direct")&&run.is_some_and(|r|r.preflight){1}else{0});aggregate["runActive"]=json!(run.is_some()&&queue_mode.as_deref()!=Some("direct"));aggregate["preflight"]=json!(run.is_some_and(|r|r.preflight)&&queue_mode.as_deref()!=Some("direct"));
        let jobs=queue::summaries(&tx,0)?;
        let working_jobs=queue::summaries_scoped(&tx,0,true,hide_finished.unwrap_or(false),queue_mode.as_deref())?;
        let selected=if job_id.is_none()&&queue_mode.as_deref()==Some("direct"){
            tx.query_row("SELECT id FROM processing_jobs WHERE queue_mode='direct'",[],|r|r.get::<_,String>(0)).optional().map_err(|e|e.to_string())?.map(|id|json!({"jobId":id})).map(|row|{let id=row["jobId"].as_str().unwrap();tx.query_row("SELECT id,status,json_remove(config,'$.analysisContract'),error,created_at_unix,task_revision,queue_mode,work_visible FROM processing_jobs WHERE id=?1",[id],|r|Ok(json!({"jobId":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"config":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).unwrap_or(Value::Null),"error":r.get::<_,Option<String>>(3)?,"createdAtUnix":r.get::<_,u64>(4)?,"revision":r.get::<_,i64>(5)?,"queueMode":r.get::<_,String>(6)?,"workVisible":r.get::<_,bool>(7)?}))).map_err(|e|e.to_string())}).transpose()?
        }else if let Some(id)=job_id{
            Some(tx.query_row("SELECT id,status,json_remove(config,'$.analysisContract'),error,created_at_unix,task_revision,queue_mode,work_visible FROM processing_jobs WHERE id=?1",[id],|r|Ok(json!({"jobId":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"config":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).unwrap_or(Value::Null),"error":r.get::<_,Option<String>>(3)?,"createdAtUnix":r.get::<_,u64>(4)?,"revision":r.get::<_,i64>(5)?,"queueMode":r.get::<_,String>(6)?,"workVisible":r.get::<_,bool>(7)?}))).map_err(crate::ui_error::processing_target_error)?)
        }else{let pool=if queue_mode.is_some(){&working_jobs}else{&jobs};pool.iter().find(|job|["running","paused","interrupted","waitingService","queued"].contains(&job["status"].as_str().unwrap_or(""))).cloned().or_else(||pool.first().cloned())};
        let Some(mut job)=selected else{return Ok(json!({"jobs":jobs,"workingJobs":working_jobs,"queueMode":queue_mode,"aggregate":aggregate,"job":null,"tasks":[],"revision":0}));};
        let id=job["jobId"].as_str().ok_or("Invalid job id.")?.to_string();
        let counters=tx.query_row("SELECT COUNT(*),COALESCE(SUM(status IN ('pending','waitingRetry')),0),COALESCE(SUM(status IN ('preparing','running','committing')),0),COALESCE(SUM(status='succeeded'),0),COALESCE(SUM(status='failed'),0),COALESCE(SUM(status='cancelled'),0),COALESCE(SUM(status='waitingRetry'),0) FROM processing_tasks WHERE job_id=?1 AND work_visible=1",[&id],|r|Ok(json!({"total":r.get::<_,u32>(0)?,"pending":r.get::<_,u32>(1)?,"running":r.get::<_,u32>(2)?,"done":r.get::<_,u32>(3)?,"failed":r.get::<_,u32>(4)?,"cancelled":r.get::<_,u32>(5)?,"retryWaiting":r.get::<_,u32>(6)?}))).map_err(|e|e.to_string())?;
        job["counters"]=counters;
        if job["workVisible"]==false{job["notice"]=json!("Задание убрано из очереди. Фотографии и результаты доступны в истории.");}
        let ready:u64=tx.query_row("SELECT COALESCE(SUM(COALESCE(json_extract(execution_context,'$.readyCount'),skipped_count)),0) FROM processing_commands WHERE job_id=?1 AND status='completed'",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if job["counters"]["total"]==0&&ready>0{job["notice"]=json!("Все выбранные фотографии уже обработаны для этого результата");}
        job["estimatedPendingUsd"]=json!(tx.query_row("SELECT COALESCE(SUM(COALESCE(json_extract(effective_config,'$.estimatedCostPerImageUsd'),0)*MAX(0,COALESCE(json_extract(effective_config,'$.passes'),1)-completed_passes)),0) FROM processing_task_context WHERE job_id=?1 AND work_visible=1 AND status IN ('pending','waitingRetry','preparing','running','committing')",[&id],|r|r.get::<_,f64>(0)).map_err(|e|e.to_string())?);
        if job["queueMode"]=="direct"||tx.query_row("SELECT COUNT(*)>1 FROM processing_commands WHERE job_id=?1",[&id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{
            if let Some(raw)=tx.query_row("SELECT json_remove(effective_config,'$.analysisContract') FROM processing_task_context WHERE job_id=?1 AND work_visible=1 ORDER BY task_ordinal DESC LIMIT 1",[&id],|r|r.get::<_,String>(0)).optional().map_err(|e|e.to_string())?{job["config"]=serde_json::from_str(&raw).map_err(|e|e.to_string())?;}
            job["mixedContexts"]=json!(tx.query_row("SELECT COUNT(DISTINCT effective_config)>1 FROM processing_task_context WHERE job_id=?1 AND work_visible=1",[&id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?);
        }
        let config:JobConfig=serde_json::from_value(job["config"].clone()).map_err(|e|e.to_string())?;
        let requested:u32=tx.query_row("SELECT requested_concurrency FROM processing_jobs WHERE id=?1",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;
        job["config"]["requestedConcurrency"]=json!(if requested==0{config.concurrency}else{requested});
        job["config"]["effectiveConcurrency"]=json!(effective_requested_concurrency(&config,requested));
        let occupied:u32=tx.query_row("SELECT COUNT(*) FROM processing_attempt_context a WHERE json_extract(a.effective_config,'$.providerMode')=?1 AND a.status IN ('reserved','running','remotePending','unknown')",[&config.provider_mode],|r|r.get(0)).map_err(|e|e.to_string())?;
        job["config"]["occupiedRequests"]=json!(occupied+if config.provider_mode=="fal"{photo_editor::jobs::occupied(&app)?}else{0});
        job["config"]["concurrencyPolicy"]=json!(if config.provider_mode=="local"{"Локально — одно фото"}else{"Сетевой предел 1–100; один декодер, 96 МиБ подготовленных кадров; незавершённые запросы сохраняют слоты"});
        let(spent,reserved,unknown):(i64,i64,u32)=tx.query_row("SELECT COALESCE(SUM(a.cost_micros),0),COALESCE(SUM(CASE WHEN a.cost_micros IS NULL THEN a.reserved_micros ELSE 0 END),0),COALESCE(SUM(a.cost_micros IS NULL AND COALESCE(json_extract(a.effective_config,'$.providerMode'),'')<>'local'),0) FROM processing_attempt_context a JOIN processing_tasks t ON t.id=a.task_id WHERE t.job_id=?1",[&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
        job["spentUsd"]=json!(spent as f64/1e6);job["reservedUsd"]=json!(reserved as f64/1e6);job["unknownCosts"]=json!(unknown);
        job["unresolvedRequests"]=json!(tx.query_row("SELECT COUNT(*) FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE t.job_id=?1 AND a.status IN ('unknown','remotePending')",[&id],|r|r.get::<_,u32>(0)).map_err(|e|e.to_string())?);
        let after=after_revision.unwrap_or(0).max(0);let mut revision=after;let mut tasks=Vec::new();
        let mut stmt=tx.prepare("SELECT c.revision,t.id,t.image_id,t.source_path,t.status,t.completed_passes,t.error,t.metadata_committed,t.rename_status,t.snapshot,t.task_ordinal,t.work_visible,t.run_revision,json_remove(t.effective_config,'$.analysisContract') FROM processing_task_changes c JOIN processing_task_context t ON t.id=c.task_id WHERE c.job_id=?1 AND c.revision>?2 ORDER BY c.revision LIMIT 200").map_err(|e|e.to_string())?;
        for row in stmt.query_map(params![id,after],|r|Ok((r.get::<_,i64>(0)?,json!({"id":r.get::<_,String>(1)?,"imageId":r.get::<_,String>(2)?,"path":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"completedPasses":r.get::<_,u32>(5)?,"error":r.get::<_,Option<String>>(6)?,"metadataCommitted":r.get::<_,bool>(7)?,"renameStatus":r.get::<_,String>(8)?,"ordinal":r.get::<_,i64>(10)?,"workVisible":r.get::<_,bool>(11)?,"runRevision":r.get::<_,i64>(12)?,"config":serde_json::from_str::<Value>(&r.get::<_,String>(13)?).unwrap_or(Value::Null),"sourceItemId":serde_json::from_str::<Value>(&r.get::<_,String>(9)?).ok().and_then(|v|v["sourceItemId"].as_str().map(str::to_string))})))).map_err(|e|e.to_string())?{let(seq,task)=row.map_err(|e|e.to_string())?;revision=seq;tasks.push(task);}
        for task in &mut tasks{
            if ["preparing","running","committing"].contains(&task["status"].as_str().unwrap_or("")) {
                let started:Option<i64>=tx.query_row("SELECT MAX(started_at_unix) FROM processing_attempts WHERE task_id=?1 AND status IN ('reserved','running','remotePending')",[task["id"].as_str().unwrap_or("")],|r|r.get(0)).map_err(|e|e.to_string())?;
                task["currentStartedAtUnix"]=json!(started);
            }
        }
        Ok(json!({"jobs":jobs,"workingJobs":working_jobs,"queueMode":queue_mode,"aggregate":aggregate,"job":job,"tasks":tasks,"revision":revision,"hasMore":revision<job["revision"].as_i64().unwrap_or(0)}))
    }).await.map_err(|e|e.to_string())?
}
pub fn validate_commit(tx:&rusqlite::Transaction<'_>,c:&CommitCheckpoint)->Result<(),String>{
    let valid:i64=tx.query_row("SELECT COUNT(*) FROM processing_jobs j JOIN processing_tasks t ON t.job_id=j.id WHERE j.id=?1 AND t.id=?2 AND j.generation=?3 AND j.status IN ('running','paused') AND t.status='committing' AND t.sha256=?4 AND t.result IS NOT NULL AND t.run_revision=?5 AND t.work_visible=1",params![c.job_id,c.task_id,c.generation,c.sha256,c.run_revision],|r|r.get(0)).map_err(|e|e.to_string())?;
    if valid!=1{return Err("Stale processing result was rejected.".into());} Ok(())
}
pub fn complete_commit(tx:&rusqlite::Transaction<'_>,c:&CommitCheckpoint)->Result<(),String>{
    tx.execute("UPDATE processing_tasks SET metadata_committed=1,status=CASE WHEN rename_status='pending' THEN 'pending' ELSE 'succeeded' END WHERE id=?1 AND run_revision=?2 AND work_visible=1",params![c.task_id,c.run_revision]).map_err(|e|e.to_string())?; Ok(())
}

pub fn start_coordinator(app:&tauri::AppHandle){
    let app=app.clone();
    tauri::async_runtime::spawn(async move{
        let initial=app.clone();
        let recovered=tauri::async_runtime::spawn_blocking(move||recover(&initial)).await;
        if !matches!(recovered,Ok(Ok(()))){eprintln!("Processing recovery failed; coordinator is disabled.");return;}
        loop {
            let tick_app=app.clone();
            match tauri::async_runtime::spawn_blocking(move||admit(&tick_app)).await {
                Ok(Ok(Admission::Work(work)))=>{let app=app.clone();tauri::async_runtime::spawn(async move{run_copy(app,work).await;});},
                Ok(Ok(Admission::Idle))=>{app.state::<ProcessingRuntime>().wake.notified().await;continue;},
                Ok(Ok(Admission::Waiting))=>{},
                Ok(Err(error))=>{let _=app.emit("processing-error",json!({"message":error}));tokio::time::sleep(Duration::from_secs(5)).await;continue;},
                Err(_)=>{tokio::time::sleep(Duration::from_secs(5)).await;continue;}
            }
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
    });
}
fn recover(app:&tauri::AppHandle)->Result<(),String>{
    let mut db=connection(app)?; let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_jobs SET status='preparationFailed',error='Preparation interrupted; create a new job.' WHERE status='preparing'",[]).map_err(|e|e.to_string())?;
    queue::recover_preparations(&tx)?;
    tx.execute("UPDATE processing_jobs SET status='interrupted' WHERE status='running' OR (queue_mode='manual' AND status='queued')",[]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_attempts SET status=CASE WHEN remote_id IS NULL THEN 'unknown' ELSE 'remotePending' END,error='Process ended before a durable response' WHERE status IN ('running','remotePending')",[]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_attempts SET status='notSent',cost_micros=0,reserved_micros=0,error='Process ended before submit admission' WHERE status='reserved'",[]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_tasks SET status='pending',rename_status=CASE WHEN rename_status='applying' THEN 'pending' ELSE rename_status END WHERE work_visible=1 AND status IN ('running','preparing','committing')",[]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())?;
    // The paid response is already durable; no repeat POST is necessary.
    queue::recover_saved(&mut db)?;queue::refresh_image_statuses(&db)?;Ok(())
}

#[derive(Clone)]
pub(super) struct Work {
    pub(super) job_id:String,pub(super) task_id:String,pub(super) image_id:String,pub(super) path:String,
    pub(super) sha:String,pub(super) generation:i64,pub(super) run_revision:i64,pub(super) config:JobConfig,pub(super) credential_ref:Option<String>,
}
enum Admission{Idle,Waiting,Work(Work)}
fn pause_disabled_job(db:&Connection,job_id:&str,provider:&str)->Result<bool,String>{
    if crate::provider_availability::available(provider){return Ok(false);}
    db.execute("UPDATE processing_jobs SET status='paused',error=?2,updated_at_unix=?3 WHERE id=?1",params![job_id,crate::provider_availability::message(),unix_now()]).map_err(|e|e.to_string())?;
    Ok(true)
}
#[cfg(test)]mod release_gate_tests{
 use super::*;
 #[test]fn release_gate_preserves_saved_work_and_credentials(){
  let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE processing_jobs(id TEXT PRIMARY KEY,status TEXT,error TEXT,updated_at_unix INTEGER,config TEXT,credential_ref TEXT);CREATE TABLE saved_tasks(job_id TEXT,status TEXT,snapshot TEXT,remote_id TEXT);INSERT INTO processing_jobs VALUES('router','running',NULL,0,'router-config','saved-router-key'),('fal','running',NULL,0,'fal-config','saved-fal-key');INSERT INTO saved_tasks VALUES('router','pending','unchanged-snapshot','paid-receipt');").unwrap();
  let blocked=pause_disabled_job(&db,"router","openrouter").unwrap();assert_eq!(blocked,!crate::provider_availability::available("openrouter"));
  let state:(String,String,String)=db.query_row("SELECT status,config,credential_ref FROM processing_jobs WHERE id='router'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!(state,(if blocked{"paused"}else{"running"}.into(),"router-config".into(),"saved-router-key".into()));
  assert!(!pause_disabled_job(&db,"fal","fal").unwrap());assert_eq!(db.query_row("SELECT status FROM processing_jobs WHERE id='fal'",[],|r|r.get::<_,String>(0)).unwrap(),"running");
  let task:(String,String,String)=db.query_row("SELECT status,snapshot,remote_id FROM saved_tasks",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!(task,("pending".into(),"unchanged-snapshot".into(),"paid-receipt".into()));
 }
}
fn admit(app:&tauri::AppHandle)->Result<Admission,String>{
    let runtime=app.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
    let mut db=connection(app)?;
    // Paused/interrupted jobs retain their attempts and money reservations,
    // but do not own scheduling. Only an explicit resume reactivates them.
    let active:Option<(String,String,String,i64,Option<String>,u32)>=db.query_row("SELECT id,status,config,generation,credential_ref,requested_concurrency FROM processing_jobs WHERE status='running' ORDER BY created_at_unix,rowid LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(|e|e.to_string())?;
    let Some((job_id,state,raw,generation,_legacy_credential_ref,requested))=active else{
        let count=db.execute("UPDATE processing_jobs SET status='running' WHERE id=(SELECT id FROM processing_jobs WHERE status='queued' ORDER BY created_at_unix,rowid LIMIT 1)",[]).map_err(|e|e.to_string())?;
        if count==0&&core.run.is_some_and(|run|!run.preflight){core.run=None;}
        return Ok(if count>0{Admission::Waiting}else{Admission::Idle});
    };
    if state!="running"{return Ok(Admission::Idle);}
    let _legacy_config:JobConfig=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
    let source={let mut stmt=db.prepare("SELECT id,image_id,source_path,sha256 FROM processing_tasks WHERE job_id=?1 AND work_visible=1 AND status IN ('pending','waitingRetry') AND (json_extract(snapshot,'$._execution.receiptId') IS NULL OR EXISTS(SELECT 1 FROM processing_commands c WHERE c.id=json_extract(processing_tasks.snapshot,'$._execution.receiptId') AND c.status IN ('prepared','started','completed'))) AND (retry_at_unix IS NULL OR retry_at_unix<=?2) ORDER BY rowid LIMIT 16").map_err(|e|e.to_string())?;
        let rows=stmt.query_map(params![job_id,unix_now()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?;
        let mut found=None;for row in rows{let row=row.map_err(|e|e.to_string())?;if !core.leased.contains(&row.0)&&!crate::file_actions::reserves_path(app,&row.2)?{found=Some(row);break;}}found};
    let Some((task_id,image_id,path,sha))=source else{
        let unfinished:i64=db.query_row("SELECT COUNT(*) FROM processing_tasks WHERE job_id=?1 AND work_visible=1 AND status NOT IN ('succeeded','failed','cancelled')",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if unfinished==0{db.execute("UPDATE processing_jobs SET status='completed',updated_at_unix=?2 WHERE id=?1",params![job_id,unix_now()]).map_err(|e|e.to_string())?;}
        else{let runnable:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks WHERE job_id=?1 AND work_visible=1 AND status NOT IN ('succeeded','failed','cancelled','waitingService'))",[&job_id],|r|r.get(0)).map_err(|e|e.to_string())?;if !runnable{db.execute("UPDATE processing_jobs SET status='waitingService',updated_at_unix=?2 WHERE id=?1",params![job_id,unix_now()]).map_err(|e|e.to_string())?;return Ok(Admission::Waiting);}}
        return Ok(if unfinished==0{Admission::Idle}else{Admission::Waiting});
    };
    let(config,credential_ref,_receipt)=queue::task_context(&db,&task_id)?;
    if pause_disabled_job(&db,&job_id,&config.provider_mode)?{return Ok(Admission::Waiting);}
    let uncertain:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE a.task_id=?1 AND a.run_revision=t.run_revision AND a.status='unknown' AND a.remote_id IS NULL)",[&task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if uncertain{db.execute("UPDATE processing_tasks SET status='waitingService',error='Исход отправленного запроса неизвестен; требуется сверка сервиса' WHERE id=?1",[&task_id]).map_err(|e|e.to_string())?;return Ok(Admission::Waiting);}
    if !fields::refresh(&db,&task_id)?{return Ok(Admission::Waiting);}
    let run_revision:i64=db.query_row("SELECT run_revision FROM processing_tasks WHERE id=?1",[&task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let concurrency=effective_requested_concurrency(&config,requested);
    if core.leased.len()>=concurrency as usize{return Ok(Admission::Waiting);}
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_tasks SET status='preparing',retry_at_unix=NULL WHERE id=?1",[&task_id]).map_err(|e|e.to_string())?; tx.commit().map_err(|e|e.to_string())?;
    core.leased.insert(task_id.clone());
    Ok(Admission::Work(Work{job_id,task_id,image_id,path,sha,generation,run_revision,config,credential_ref}))
}

fn current_state(db:&Connection,w:&Work)->Result<String,String>{
    let (state,generation,revision,visible):(String,i64,i64,bool)=db.query_row("SELECT j.status,j.generation,t.run_revision,t.work_visible FROM processing_jobs j JOIN processing_tasks t ON t.job_id=j.id WHERE j.id=?1 AND t.id=?2",params![w.job_id,w.task_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
    if generation!=w.generation||revision!=w.run_revision||!visible{return Err("Cancelled processing ownership.".into());} Ok(state)
}

async fn run_copy(app:tauri::AppHandle,w:Work){
    let work=w.clone();let worker_app=app.clone();
    let result=run_copy_inner(&app,&w).await;
    let _=tauri::async_runtime::spawn_blocking(move||{
        let runtime=worker_app.state::<ProcessingRuntime>(); let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
        core.leased.remove(&work.task_id);
        let db=connection(&worker_app)?;
        if let Err(error)=result {
            if current_state(&db,&work).is_ok(){
                db.execute("UPDATE processing_tasks SET status='failed',error=?2 WHERE id=?1 AND status<>'succeeded'",params![work.task_id,error]).map_err(|e|e.to_string())?;
                if work.config.stop_on_first_error {db.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1",params![work.job_id,error]).map_err(|e|e.to_string())?;}
            }
        }
        notify_change(&worker_app);
        Ok::<_,String>(())
    }).await;
}

async fn run_copy_inner(app:&tauri::AppHandle,w:&Work)->Result<(),String>{
    let check_app=app.clone(); let check_work=w.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&check_app)?;
        let committed:bool=db.query_row("SELECT metadata_committed FROM processing_tasks WHERE id=?1",[&check_work.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !committed && compute_file_sha256(Path::new(&check_work.path))?!=check_work.sha{return Err("Source contents changed; create a new job.".into());}Ok::<_,String>(())
    }).await.map_err(|e|e.to_string())??;
    let mut frame=None;
    loop {
        let app_prepare=app.clone();let work=w.clone();
        let next=tauri::async_runtime::spawn_blocking(move||begin_pass(&app_prepare,&work)).await.map_err(|e|e.to_string())??;
        match next {
            Next::Wait=>return Ok(()),
            Next::Commit(result)=>{
                let app_commit=app.clone();let work=w.clone();
                return tauri::async_runtime::spawn_blocking(move||{
                    let checkpoint=CommitCheckpoint{job_id:work.job_id,task_id:work.task_id,generation:work.generation,run_revision:work.run_revision,sha256:work.sha};
                    let input:IndexMetadataUpdateInputDto=serde_json::from_value(json!({"imageId":work.image_id,"path":work.path,"source":"ai","model":work.config.model,"fields":metadata_fields(&work.config.output_mode)?,"description":result["description"],"tags":result["tags"]})).map_err(|e|e.to_string())?;
                    let saved=update_image_metadata_guarded(app_commit.clone(),input,Some(&checkpoint))?;
                    let _=viewer_sessions::update_metadata(&app_commit,&saved);
                    let _=app_commit.emit("processing-metadata",serde_json::to_value(saved).map_err(|e|e.to_string())?);
                    Ok(())
                }).await.map_err(|e|e.to_string())?;
            },
            Next::Rename=>{
                let rename_app=app.clone();let work=w.clone();
                return tauri::async_runtime::spawn_blocking(move||run_rename(&rename_app,&work)).await.map_err(|e|e.to_string())?;
            },
            Next::Request{attempt,pass,mut cancel,candidates}=>{
                // A resumed remote receipt retains its original key/model even
                // when the task's key for future attempts has been replaced.
                let request_work=queue::attempt_work(&connection(app)?,&attempt)?;
                let w=&request_work;
                let response=tokio::select!{
                    r=async {
                        if test_transport(&w.config){return controlled_request(app,w,&attempt,pass,candidates).await.map(|raw|provider_transport::reply_from_raw("controlled-test",raw)).map_err(|message|provider_transport::ProviderFailure{message,retryable:true,uncertain:false,retry_after:1,group:None});}
                        if frame.is_none() && load_remote(app,&attempt).await.map_err(provider_transport::ProviderFailure::definitive)?.is_none(){frame=Some(provider_transport::prepare_frame(w).await.map_err(provider_transport::ProviderFailure::definitive)?);}
                        let reply=provider_transport::infer(app,w,&attempt,&candidates,frame.as_ref()).await?;
                        persist_reply(app,&attempt,&reply).await.map_err(provider_transport::ProviderFailure::unknown)?;
                        Ok(reply)
                    }=>r,
                    _=cancel.changed()=>{
                        let cancel_app=app.clone();let cancel_work=w.clone();let cancel_attempt=attempt.clone();
                        tauri::async_runtime::spawn(async move{let _=provider_transport::cancel_attempt(&cancel_app,&cancel_work,&cancel_attempt).await;});
                        Err(provider_transport::ProviderFailure::unknown("User cancelled this attempt."))
                    },
                };
                let app_save=app.clone();let work=w.clone();
                let saved=tauri::async_runtime::spawn_blocking(move||save_attempt(&app_save,&work,&attempt,pass,response)).await.map_err(|e|e.to_string())??;
                if !saved{return Ok(());}
            }
        }
    }
}

enum Next {Wait,Commit(Value),Rename,Request{attempt:String,pass:u32,cancel:watch::Receiver<bool>,candidates:Vec<Value>}}
fn begin_pass(app:&tauri::AppHandle,w:&Work)->Result<Next,String>{
    let runtime=app.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
    let mut db=connection(app)?;
    if current_state(&db,w)?!="running"{
        db.execute("UPDATE processing_tasks SET status='pending' WHERE id=?1 AND run_revision=?2 AND work_visible=1 AND status IN ('preparing','running')",params![w.task_id,w.run_revision]).map_err(|e|e.to_string())?;
        return Ok(Next::Wait);
    }
    let (completed,committed,rename):(u32,bool,String)=db.query_row("SELECT completed_passes,metadata_committed,rename_status FROM processing_tasks WHERE id=?1",[&w.task_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    if committed{return if rename=="pending"{
        db.execute("UPDATE processing_tasks SET rename_status='applying' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;
        Ok(Next::Rename)
    }else{Ok(Next::Wait)};}
    if completed==w.config.passes {
        let raw:String=db.query_row("SELECT result FROM processing_tasks WHERE id=?1",[&w.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        db.execute("UPDATE processing_tasks SET status='committing' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;
        return Ok(Next::Commit(serde_json::from_str(&raw).map_err(|e|e.to_string())?));
    }
    let pass=completed+1;
    let existing:Option<String>=db.query_row("SELECT id FROM processing_attempts WHERE task_id=?1 AND pass_number=?2 AND run_revision=?3 AND remote_id IS NOT NULL AND status IN ('unknown','remotePending') ORDER BY rowid DESC LIMIT 1",params![w.task_id,pass,w.run_revision],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if let Some(id)=existing{
        let candidates=read_candidates(&db,w,pass)?;
        let (sender,cancel)=watch::channel(false);core.cancellations.insert(id.clone(),sender);
        db.execute("UPDATE processing_attempts SET status='remotePending' WHERE id=?1",[&id]).map_err(|e|e.to_string())?;
        db.execute("UPDATE processing_tasks SET status='running' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;
        return Ok(Next::Request{attempt:id,pass,cancel,candidates});
    }
    let requested:u32=db.query_row("SELECT requested_concurrency FROM processing_jobs WHERE id=?1",[&w.job_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let effective=effective_requested_concurrency(&w.config,requested);
    if core.leased.len()>effective as usize{
        db.execute("UPDATE processing_tasks SET status='pending' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;
        return Ok(Next::Wait);
    }
    let attempts:i64=db.query_row("SELECT COUNT(*) FROM processing_attempts WHERE task_id=?1 AND pass_number=?2 AND generation=?3 AND run_revision=?4 AND status<>'notSent'",params![w.task_id,pass,w.generation,w.run_revision],|r|r.get(0)).map_err(|e|e.to_string())?;
    if attempts>i64::from(w.config.max_retries_per_task){return Err("Retry limit reached for this pass.".into());}
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    if matches!(w.config.provider_mode.as_str(),"fal"|"openrouter"){
        // Share confirmed/admitted capacity with editing, using the same predicate.
        let occupied=occupied_cloud_db(&tx,&w.config.provider_mode)?;
        let occupied=occupied+if w.config.provider_mode=="fal"{photo_editor::jobs::occupied(app)?}else{0};
        if occupied>=effective{
            tx.execute("UPDATE processing_tasks SET status='waitingRetry',retry_at_unix=?2,error='Ожидание свободного места сервиса: незавершённые запросы сохраняют свой лимит' WHERE id=?1",params![w.task_id,unix_now().saturating_add(2)]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;return Ok(Next::Wait);
        }
    }
    let(_,_,receipt)=queue::task_context(&tx,&w.task_id)?;
    let spent:i64=tx.query_row("SELECT COALESCE(SUM(COALESCE(a.cost_micros,a.reserved_micros)),0) FROM processing_attempt_context a WHERE a.job_id=?1 AND (?2 IS NULL OR a.receipt_id=?2)",params![w.job_id,if w.config.queue_contract_version>=2{receipt.as_deref()}else{None}],|r|r.get(0)).map_err(|e|e.to_string())?;
    let reserve=(w.config.estimated_cost_per_image_usd*1_000_000.0).ceil() as i64;
    if fields::budget_micros(&w.config).is_some_and(|limit|spent+reserve>limit){return Err("Budget exhausted; unresolved attempts retain their reservations.".into());}
    let service=w.config.provider_mode.clone();let now=Instant::now();
    let cooldown:Option<u64>=tx.query_row("SELECT not_before_unix FROM processing_service_cooldowns WHERE provider=?1",[&service],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if cooldown.is_some_and(|deadline|deadline>unix_now()){
        tx.execute("UPDATE processing_tasks SET status='waitingRetry',retry_at_unix=?2 WHERE id=?1",params![w.task_id,cooldown]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;return Ok(Next::Wait);
    }
    if core.next_request.get(&service).is_some_and(|next|*next>now){
        tx.execute("UPDATE processing_tasks SET status='pending' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;return Ok(Next::Wait);
    }
    let ordinal:i64=tx.query_row("SELECT COUNT(*) FROM processing_attempts WHERE task_id=?1",[&w.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let id=stable_text_id("attempt",&format!("{}|{pass}|{ordinal}",w.task_id));
    tx.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,run_revision,execution_context,status,started_at_unix,reserved_micros) VALUES(?1,?2,?3,?4,?5,?6,'reserved',?7,?8)",params![id,w.task_id,pass,w.generation,w.run_revision,queue::context(&w.config,w.credential_ref.as_deref(),receipt.as_deref()).to_string(),unix_now(),reserve]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE processing_tasks SET status='running' WHERE id=?1",[&w.task_id]).map_err(|e|e.to_string())?;
    let mut candidates=Vec::new();
    if w.config.passes>1 && pass==w.config.passes{
        let mut stmt=tx.prepare("SELECT result FROM processing_passes WHERE task_id=?1 ORDER BY pass_number").map_err(|e|e.to_string())?;
        for raw in stmt.query_map([&w.task_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?{candidates.push(serde_json::from_str(&raw.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?);}
    }
    tx.commit().map_err(|e|e.to_string())?;
    let (sender,cancel)=watch::channel(false);core.cancellations.insert(id.clone(),sender);
    Ok(Next::Request{attempt:id,pass,cancel,candidates})
}
fn effective_requested_concurrency(config:&JobConfig,requested:u32)->u32{
    if config.provider_mode=="local"{return 1;}
    // Decoding has its own single-worker semaphore. Keep uploads/response buffers bounded.
    (if requested==0{config.concurrency}else{requested}).clamp(1,100)
}
fn read_candidates(db:&Connection,w:&Work,pass:u32)->Result<Vec<Value>,String>{
    if w.config.passes==1 || pass!=w.config.passes{return Ok(Vec::new());}
    let mut stmt=db.prepare("SELECT result FROM processing_passes WHERE task_id=?1 ORDER BY pass_number").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([&w.task_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
    let result=rows.map(|r|serde_json::from_str(&r.map_err(|e|e.to_string())?).map_err(|e|e.to_string())).collect();result
}

async fn controlled_request(app:&tauri::AppHandle,w:&Work,attempt:&str,pass:u32,candidates:Vec<Value>)->Result<Value,String>{
    if !test_transport(&w.config){return Err("Provider transport has not been enabled.".into());}
    let endpoint=w.config.endpoint.as_deref().ok_or("Controlled endpoint is required.")?;
    let url=reqwest::Url::parse(endpoint).map_err(|e|e.to_string())?;
    if url.scheme()!="http" || url.host_str()!=Some("127.0.0.1") {return Err("Controlled transport only accepts loopback HTTP.".into());}
    let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(30)).build().map_err(|e|e.to_string())?;
    let request=client.post(url).json(&json!({"taskId":w.task_id,"sourceRevision":w.sha,"passNumber":pass,"totalPasses":w.config.passes,"candidates":candidates,"model":w.config.model}));
    let mut response=send_with_admission(app,w,attempt,request).await.map_err(|e|e.to_string())?;
    let code=response.status(); let mut bytes=Vec::new();
    while let Some(chunk)=response.chunk().await.map_err(|_|"Response read failed.")?{
        if bytes.len()+chunk.len()>1024*1024{return Err("Provider response exceeds 1 MiB.".into());}bytes.extend_from_slice(&chunk);
    }
    if !code.is_success(){return Err(format!("Controlled provider returned HTTP {}.",code.as_u16()));}
    let value:Value=serde_json::from_slice(&bytes).map_err(|_|"Provider returned malformed JSON.".to_string())?;
    Ok(value)
}

pub(super) async fn send_with_admission(app:&tauri::AppHandle,w:&Work,attempt:&str,request:reqwest::RequestBuilder)->Result<reqwest::Response,String>{
    use std::future::{poll_fn,Future};
    use std::task::Poll;
    let mut future=Box::pin(request.send());let mut submitted=false;
    let mut wait:Option<std::pin::Pin<Box<tokio::time::Sleep>>>=None;
    poll_fn(|context|{
        if !submitted {
            let runtime=app.state::<ProcessingRuntime>();let Ok(mut core)=runtime.core.lock() else{return Poll::Ready(Err("Coordinator lock failed.".into()));};
            let check=(||{
                let db=connection(app)?;
                if w.config.provider_mode!="local" {if let Err(error)=profile_preferences::require_cloud(app){db.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1",params![w.job_id,error]).map_err(|e|e.to_string())?;db.execute("UPDATE processing_attempts SET status='notSent',cost_micros=0,reserved_micros=0,error=?2 WHERE id=?1",params![attempt,error]).map_err(|e|e.to_string())?;return Err(error);}}

                if current_state(&db,w).as_deref()!=Ok("running") {
                    db.execute("UPDATE processing_attempts SET status='notSent',cost_micros=0,reserved_micros=0,error='Paused before submit' WHERE id=?1",[attempt]).map_err(|e|e.to_string())?;
                    db.execute("UPDATE processing_tasks SET status='pending' WHERE id=?1 AND status<>'cancelled' AND run_revision=?2 AND work_visible=1",params![w.task_id,w.run_revision]).map_err(|e|e.to_string())?;
                    return Err("Not submitted: pause/stop acknowledged before admission.".into());
                }
                Ok::<_,String>(())
            })();
            if let Err(error)=check{return Poll::Ready(Err(error));}
            if let Some(next)=core.next_request.get(&w.config.provider_mode).copied(){
                if next>Instant::now(){
                    let sleep=wait.get_or_insert_with(||Box::pin(tokio::time::sleep(next.duration_since(Instant::now()))));
                    sleep.as_mut().reset(tokio::time::Instant::from_std(next));
                    if sleep.as_mut().poll(context).is_pending(){return Poll::Pending;}
                    wait=None;
                }
            }
            let submitted_record=connection(app).and_then(|db|db.execute("UPDATE processing_attempts SET status='running' WHERE id=?1 AND status='reserved'",[attempt]).map_err(|e|e.to_string()));
            match submitted_record{Ok(1)=>{},Ok(_)=>return Poll::Ready(Err("Not submitted: attempt admission changed.".into())),Err(error)=>return Poll::Ready(Err(error))}
            core.next_request.insert(w.config.provider_mode.clone(),Instant::now()+Duration::from_millis(w.config.request_delay_ms));
            submitted=true;
            // The first poll starts submit under the same fence as command acknowledgement.
            return future.as_mut().poll(context).map(|r|r.map_err(|error|provider_transport::inference_network_message(&w.config.provider_mode,&error)));
        }
        future.as_mut().poll(context).map(|r|r.map_err(|error|provider_transport::inference_network_message(&w.config.provider_mode,&error)))
    }).await
}

fn validate_file_plan(db:&Connection,config:&JobConfig,id:&str,image:&str,path:&str,sha:&str,preparing:bool)->Result<(String,String,String),String>{
 let(kind,owner,source,expected,status,target):(String,String,String,String,String,String)=db.query_row("SELECT kind,image_id,source_path,sha256,status,target_path FROM file_operations WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|e|e.to_string())?;
 if !config.smart_rename_enabled{return Err("Умное имя выключено; файловая операция не разрешена заданием.".into());}
 let copy=config.analysis_copy.as_ref().filter(|copy|copy.enabled);
 if kind!=if copy.is_some(){"analysisCopy"}else{"rename"}||owner!=image||source!=path||expected!=sha{return Err("План файловой операции не соответствует фотографии и настройкам задания.".into());}
 if preparing&&(status!="planned"||db.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks WHERE rename_plan_id=?1)",[id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?){return Err("План уже использован другим заданием. Подготовьте новый предпросмотр.".into());}
 let parent=Path::new(path).parent().ok_or("Нет папки исходника")?;
 let expected_parent=if let Some(copy)=copy{match copy.destination.as_str(){"beside"=>parent,"folder"=>Path::new(&copy.folder),_=>return Err("Неизвестная папка копии.".into())}}else{parent};
 let actual_parent=Path::new(&target).parent().ok_or("Нет папки назначения")?;
 if fs::canonicalize(expected_parent).map_err(|e|e.to_string())?!=fs::canonicalize(actual_parent).map_err(|e|e.to_string())?{return Err("Папка плана отличается от сохранённой папки задания.".into());}
 Ok((kind,status,target))
}
#[cfg(all(test,windows))]mod file_phase_tests{
 use super::*;
 #[test]fn file_plan_matches_owner_content_kind_destination_and_single_claim(){
  let root=crate::test_support::temp_dir("smart-photo-plan");fs::create_dir_all(root.join("copies")).unwrap();fs::create_dir(root.join("elsewhere")).unwrap();let source=path_to_string(&root.join("original.png"));let target=path_to_string(&root.join("copies/copy.png"));
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,status,created_at_unix,updated_at_unix) VALUES('i','i','original.png','processed',1,1)",[]).unwrap();
  db.execute("INSERT INTO file_operations(id,kind,image_id,source_path,target_path,sha256,size_bytes,status,created_at_unix,updated_at_unix) VALUES('plan','analysisCopy','i',?1,?2,'expected',1,'planned',1,1)",params![source,target]).unwrap();
  let config:JobConfig=serde_json::from_value(json!({"providerMode":"local","model":"qwen3.5:2b","smartRenameEnabled":true,"analysisCopy":{"enabled":true,"destination":"folder","folder":path_to_string(&root.join("copies"))}})).unwrap();
  assert!(validate_file_plan(&db,&config,"plan","i",&source,"expected",true).is_ok());
  for(image,path,sha)in[("other",source.as_str(),"expected"),("i","other.png","expected"),("i",source.as_str(),"changed")]{assert!(validate_file_plan(&db,&config,"plan",image,path,sha,true).is_err());}
  let mut changed=config.clone();changed.smart_rename_enabled=false;assert!(validate_file_plan(&db,&changed,"plan","i",&source,"expected",false).is_err());changed=config.clone();changed.analysis_copy.as_mut().unwrap().enabled=false;assert!(validate_file_plan(&db,&changed,"plan","i",&source,"expected",true).is_err());changed=config.clone();changed.analysis_copy.as_mut().unwrap().folder=path_to_string(&root.join("elsewhere"));assert!(validate_file_plan(&db,&changed,"plan","i",&source,"expected",true).is_err());
  db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','key','{}','prepared',1,1)",[]).unwrap();db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status,rename_plan_id) VALUES('t','j','i',?1,'expected','{}','pending','plan')",[&source]).unwrap();assert!(validate_file_plan(&db,&config,"plan","i",&source,"expected",true).is_err());assert!(validate_file_plan(&db,&config,"plan","i",&source,"expected",false).is_ok());fs::remove_dir_all(root).unwrap();
 }
}
fn run_rename(app:&tauri::AppHandle,w:&Work)->Result<(),String>{
    let runtime=app.state::<ProcessingRuntime>();let _admission=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;
    let mut db=connection(app)?;file_operations::recover(&mut db)?;
    if current_state(&db,w).as_deref()!=Ok("running"){
        db.execute("UPDATE processing_tasks SET rename_status='pending',status=CASE WHEN status='cancelled' THEN status ELSE 'pending' END WHERE id=?1 AND run_revision=?2 AND work_visible=1",params![w.task_id,w.run_revision]).map_err(|e|e.to_string())?;return Ok(());
    }
    let plan:String=db.query_row("SELECT rename_plan_id FROM processing_tasks WHERE id=?1",[&w.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let outcome=validate_file_plan(&db,&w.config,&plan,&w.image_id,&w.path,&w.sha,false).and_then(|(kind,status,target)|{
      let result=if status=="committed"{Ok(None)}else if kind=="analysisCopy"{analysis_copy::apply(&mut db,&plan).map(|_|None)}else{
        file_operations::rename(app.clone(),SmartRenameInputDto{image_id:Some(w.image_id.clone()),current_path:w.path.clone(),file_name:file_name(Path::new(&w.path)),modified_at_unix:None,pattern:"date_unique8".into(),plan_id:Some(plan)}).map(Some)
      };result.map(|result|(result,target))
    });
    match outcome {
        Ok((result,target))=>{
            db.execute("UPDATE processing_tasks SET rename_status='succeeded',status=CASE WHEN (SELECT generation FROM processing_jobs WHERE id=?2)=?3 THEN 'succeeded' ELSE status END,error=NULL WHERE id=?1",params![w.task_id,w.job_id,w.generation]).map_err(|e|e.to_string())?;
            if let Some(result)=result{let _=app.emit("processing-renamed",serde_json::to_value(result).map_err(|e|e.to_string())?);}
            let _=app.emit("processing-changed",json!({"jobId":w.job_id,"taskId":w.task_id,"renamedPath":target}));Ok(())
        },
        Err(error)=>{
            db.execute("UPDATE processing_tasks SET rename_status='failed',status=CASE WHEN status='cancelled' THEN status ELSE 'failed' END,error=?2 WHERE id=?1",params![w.task_id,error]).map_err(|e|e.to_string())?;
            if w.config.rename_stop_on_first_error{db.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1 AND generation=?3 AND status='running'",params![w.job_id,error,w.generation]).map_err(|e|e.to_string())?;}
            // Analysis has already committed. This is a separate file phase, never an inference retry.
            Ok(())
        }
    }
}

pub(super) fn validate_result(mut value:Value)->Result<Value,String>{
    let fields=value.as_object().ok_or("Analysis must be a JSON object.")?;
    if fields.len()!=5 || fields.keys().any(|field|!["description","tags","objects","scene","confidence"].contains(&field.as_str())){return Err("Analysis contains fields outside its schema.".into());}
    let description=value.get("description").and_then(Value::as_str).ok_or("Description is missing.")?.trim();
    if description.is_empty() || description.len()>256*1024{return Err("Description is empty or too large.".into());}
    let tags=value.get("tags").and_then(Value::as_array).ok_or("Tags must be an array.")?;
    if tags.len()>30 || tags.iter().any(|v|v.as_str().is_none_or(|s|s.trim().is_empty() || s.len()>160)){return Err("Invalid tags.".into());}
    let confidence=value.get("confidence").and_then(Value::as_f64).ok_or("Confidence is missing.")?;
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence){return Err("Invalid confidence.".into());}
    let objects=value.get("objects").and_then(Value::as_array).ok_or("Objects must be an array.")?;
    if objects.len()>100 || objects.iter().any(|item|item.as_str().is_none_or(|text|text.trim().is_empty() || text.len()>256)) {return Err("Invalid objects.".into());}
    let scene=value.get("scene").and_then(Value::as_str).ok_or("Scene must be a string.")?;
    if scene.len()>4096 {return Err("Scene exceeds its size limit.".into());}
    value["description"]=json!(description);
    let mut normalized=Vec::<String>::new();
    for tag in value["tags"].as_array().ok_or("Tags must be an array.")?.iter().filter_map(Value::as_str){
        let tag=tag.trim();if !normalized.iter().any(|old|old.to_lowercase()==tag.to_lowercase()){normalized.push(tag.into());}
    }
    value["tags"]=json!(normalized);Ok(value)
}
fn save_attempt(app:&tauri::AppHandle,w:&Work,id:&str,pass:u32,response:Result<provider_transport::ProviderReply,provider_transport::ProviderFailure>)->Result<bool,String>{
    let response=response.map_err(|error|error.for_provider(&w.config.provider_mode));
    let runtime=app.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed.")?;core.cancellations.remove(id);
    let mut db=connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let status:String=tx.query_row("SELECT status FROM processing_attempts WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if matches!(status.as_str(),"succeeded"|"failed"|"lateResponse"|"manualResolved"){return Ok(false);}
    if status=="notSent" || status=="reserved" {
        tx.execute("UPDATE processing_attempts SET status='notSent',cost_micros=0,reserved_micros=0,finished_at_unix=?2 WHERE id=?1",params![id,unix_now()]).map_err(|e|e.to_string())?;
        if current_state(&tx,w).is_err(){tx.commit().map_err(|e|e.to_string())?;return Ok(false);}
        let prep_error=if status=="reserved" && current_state(&tx,w).as_deref()==Ok("running"){response.err().filter(|e|!e.retryable).map(|e|e.message)}else{None};
        if let Some(error)=prep_error {
            tx.execute("UPDATE processing_tasks SET status='failed',error=?2 WHERE id=?1 AND status<>'cancelled'",params![w.task_id,error]).map_err(|e|e.to_string())?;
            if w.config.stop_on_first_error{tx.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1 AND generation=?3 AND status='running'",params![w.job_id,error,w.generation]).map_err(|e|e.to_string())?;}
        }else{tx.execute("UPDATE processing_tasks SET status='pending' WHERE id=?1 AND status<>'cancelled'",[&w.task_id]).map_err(|e|e.to_string())?;}
        tx.commit().map_err(|e|e.to_string())?;return Ok(false);
    }
    let state=current_state(&tx,w);
    if state.is_err() || state.as_deref()==Ok("stopped"){
        // Durable cost/response, if available, is retained. Generation prevents metadata application.
        match response {
            Ok(reply)=>{let normalized=provider_transport::decode_reply(&w.config.provider_mode,&reply.raw,w.config.analysis_contract.as_ref()).ok().map(|v|v.to_string());tx.execute("UPDATE processing_attempts SET status='lateResponse',response=?2,cost_micros=?3,usage=?4,error='Cancelled; late response was not applied',finished_at_unix=?5,normalized_result=?6 WHERE id=?1",params![id,reply.raw.to_string(),reply.cost_micros,reply.usage.to_string(),unix_now(),normalized]).map_err(|e|e.to_string())?;},
            Err(error)=>{tx.execute("UPDATE processing_attempts SET status=CASE WHEN response IS NOT NULL THEN 'lateResponse' WHEN ?3=1 THEN 'failed' WHEN remote_id IS NOT NULL THEN 'remotePending' ELSE 'unknown' END,error=?4,finished_at_unix=?2,cost_micros=CASE WHEN ?5 THEN 0 ELSE cost_micros END WHERE id=?1",params![id,unix_now(),!error.uncertain,error.message,w.config.provider_mode=="local"]).map_err(|e|e.to_string())?;}
        }
        tx.commit().map_err(|e|e.to_string())?;return Ok(false);
    }
    let mut successful=false;
    match response {
        Ok(reply)=>{
            tx.execute("UPDATE processing_attempts SET response=?2,cost_micros=?3,usage=?4,finished_at_unix=?5 WHERE id=?1",params![id,reply.raw.to_string(),reply.cost_micros,reply.usage.to_string(),unix_now()]).map_err(|e|e.to_string())?;
            match provider_transport::decode_reply(&w.config.provider_mode,&reply.raw,w.config.analysis_contract.as_ref()){
                Ok(result)=>{
                    let raw=result.to_string();
                    tx.execute("UPDATE processing_attempts SET status='succeeded',error=NULL,normalized_result=?2 WHERE id=?1",params![id,raw]).map_err(|e|e.to_string())?;
                    tx.execute("INSERT INTO processing_passes(task_id,pass_number,result,attempt_id) VALUES(?1,?2,?3,?4)",params![w.task_id,pass,raw,id]).map_err(|e|e.to_string())?;
                    tx.execute("UPDATE processing_tasks SET completed_passes=?2,result=?3,status='pending',error=NULL WHERE id=?1",params![w.task_id,pass,raw]).map_err(|e|e.to_string())?;successful=true;
                },
                Err(error)=>{
                    tx.execute("UPDATE processing_attempts SET status='failed',error=?2 WHERE id=?1",params![id,error]).map_err(|e|e.to_string())?;
                    schedule_failure(&tx,w,pass,&error,true,1)?;
                }
            }
        },
        Err(error)=>{
            if error.retryable && error.retry_after>1{tx.execute("INSERT INTO processing_service_cooldowns(provider,not_before_unix) VALUES(?1,?2) ON CONFLICT(provider) DO UPDATE SET not_before_unix=MAX(not_before_unix,excluded.not_before_unix)",params![w.config.provider_mode,unix_now().saturating_add(error.retry_after)]).map_err(|e|e.to_string())?;}
            let remote:bool=tx.query_row("SELECT remote_id IS NOT NULL FROM processing_attempts WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
            let status=if error.uncertain{if remote{"remotePending"}else{"unknown"}}else{"failed"};
            let cost=if test_transport(&w.config) || w.config.provider_mode=="local" || !error.uncertain && !remote{Some(0)}else{None};
            tx.execute("UPDATE processing_attempts SET status=?2,cost_micros=COALESCE(?3,cost_micros),error=?4,finished_at_unix=?5 WHERE id=?1",params![id,status,cost,error.message,unix_now()]).map_err(|e|e.to_string())?;
            schedule_failure(&tx,w,pass,&error.message,error.retryable && !error.uncertain,error.retry_after)?;
            if let Some(group)=error.group{queue::fail_unsent_group(&tx,w,group,&error.message)?;}
            if error.uncertain{
                tx.execute("UPDATE processing_tasks SET status='waitingService',error=?2,retry_at_unix=NULL WHERE id=?1",params![w.task_id,error.message]).map_err(|e|e.to_string())?;}
        }
    }
    tx.commit().map_err(|e|e.to_string())?;
    let _=app.emit("processing-changed",json!({"jobId":w.job_id,"taskId":w.task_id,"completedPass":pass}));
    Ok(successful)
}
fn schedule_failure(tx:&rusqlite::Transaction<'_>,w:&Work,pass:u32,error:&str,retryable:bool,delay:u64)->Result<(),String>{
    let attempts:u32=tx.query_row("SELECT COUNT(*) FROM processing_attempts WHERE task_id=?1 AND pass_number=?2 AND generation=?3 AND run_revision=?4 AND status<>'notSent'",params![w.task_id,pass,w.generation,w.run_revision],|r|r.get(0)).map_err(|e|e.to_string())?;
    let retry=retryable && attempts<=w.config.max_retries_per_task;
    let next=unix_now().saturating_add(delay.max(1));
    tx.execute("UPDATE processing_tasks SET status=?2,retry_at_unix=?3,error=?4 WHERE id=?1",params![w.task_id,if retry{"waitingRetry"}else{"failed"},next,error]).map_err(|e|e.to_string())?;
    if !retry && w.config.stop_on_first_error{tx.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1 AND generation=?3 AND status='running'",params![w.job_id,error,w.generation]).map_err(|e|e.to_string())?;}
    Ok(())
}

#[derive(Clone)]
pub(super) struct RemoteRequest {pub id:String,pub status_url:String,pub response_url:String,pub cancel_url:String}
impl RemoteRequest {
    pub fn from_submit(value:&Value)->Result<Self,String>{
        let field=|name:&str|value[name].as_str().map(str::to_string).ok_or_else(||format!("Fal did not return {name}."));
        let remote=Self{id:field("request_id")?,status_url:field("status_url")?,response_url:field("response_url")?,cancel_url:field("cancel_url")?};
        for(url,kind) in [(&remote.status_url,"status"),(&remote.response_url,"response"),(&remote.cancel_url,"cancel")]{provider_transport::fal_remote_url(url,&remote.id,kind)?;}
        Ok(remote)
    }
}
pub(super) async fn persist_remote(app:&tauri::AppHandle,id:&str,remote:&RemoteRequest)->Result<(),String>{
    let app=app.clone();let id=id.to_string();let remote=remote.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;
        // Include cancelled/unknown attempts: submit may finish just after a Stop acknowledgement.
        let changed=db.execute("UPDATE processing_attempts SET remote_id=?2,status_url=?3,response_url=?4,cancel_url=?5,status=CASE WHEN status='running' THEN 'remotePending' ELSE status END WHERE id=?1 AND remote_id IS NULL",params![id,remote.id,remote.status_url,remote.response_url,remote.cancel_url]).map_err(|e|e.to_string())?;
        if changed!=1{return Err("Fal request id could not be checkpointed.".into());}Ok(())
    }).await.map_err(|e|e.to_string())?
}
pub(super) async fn load_remote(app:&tauri::AppHandle,id:&str)->Result<Option<RemoteRequest>,String>{
    let app=app.clone();let id=id.to_string();
    tauri::async_runtime::spawn_blocking(move||connection(&app)?.query_row("SELECT remote_id,status_url,response_url,cancel_url FROM processing_attempts WHERE id=?1 AND remote_id IS NOT NULL",[id],|r|Ok(RemoteRequest{id:r.get(0)?,status_url:r.get(1)?,response_url:r.get(2)?,cancel_url:r.get(3)?})).optional().map_err(|e|e.to_string())).await.map_err(|e|e.to_string())?
}
pub(super) async fn is_cancelled(app:&tauri::AppHandle,w:&Work)->Result<bool,String>{
    let app=app.clone();let w=w.clone();tauri::async_runtime::spawn_blocking(move||{let db=connection(&app)?;Ok(current_state(&db,&w).is_err() || current_state(&db,&w).as_deref()==Ok("stopped"))}).await.map_err(|e|e.to_string())?
}
pub(super) async fn record_cancel(app:&tauri::AppHandle,id:&str,code:u16)->Result<(),String>{
    let app=app.clone();let id=id.to_string();tauri::async_runtime::spawn_blocking(move||{
        connection(&app)?.execute("UPDATE processing_attempts SET error=COALESCE(error,'') || ?2 WHERE id=?1",params![id,format!("; Fal cancellation HTTP {code}; final outcome still requires reconciliation")]).map_err(|e|e.to_string())?;Ok(())
    }).await.map_err(|e|e.to_string())?
}
async fn persist_reply(app:&tauri::AppHandle,id:&str,reply:&provider_transport::ProviderReply)->Result<(),String>{
    let app=app.clone();let id=id.to_string();let reply=reply.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;let work=queue::attempt_work(&db,&id)?;
        let normalized=provider_transport::decode_reply(&work.config.provider_mode,&reply.raw,work.config.analysis_contract.as_ref()).ok().map(|v|v.to_string());
        db.execute("UPDATE processing_attempts SET status=CASE WHEN status IN ('running','remotePending') THEN 'responseSaved' ELSE status END,response=?2,cost_micros=?3,usage=?4,finished_at_unix=?5,normalized_result=COALESCE(?6,normalized_result) WHERE id=?1",params![id,reply.raw.to_string(),reply.cost_micros,reply.usage.to_string(),unix_now(),normalized]).map_err(|e|e.to_string())?;Ok(())
    }).await.map_err(|e|e.to_string())?
}

async fn cancel_known_job_attempts(app:&tauri::AppHandle,job_id:&str)->Result<(),String>{
    let query_app=app.clone();let job_id=job_id.to_string();
    let attempts=tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&query_app)?;
        let mut stmt=db.prepare("SELECT id FROM processing_attempt_context WHERE job_id=?1 AND remote_id IS NOT NULL AND status IN ('running','remotePending','unknown')").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([job_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
        rows.map(|r|{let id=r.map_err(|e|e.to_string())?;let work=queue::attempt_work(&db,&id)?;Ok((id,work))}).collect::<Result<Vec<_>,String>>()
    }).await.map_err(|e|e.to_string())??;
    for(id,work) in attempts{let _=provider_transport::cancel_attempt(app,&work,&id).await;}
    Ok(())
}

#[cfg(test)]mod remote_terminal_tests{
 use super::*;
 #[test]fn terminal_failure_releases_slot_but_keeps_unknown_money(){let mut db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE images(id TEXT PRIMARY KEY);CREATE TABLE tasks(id TEXT PRIMARY KEY,status TEXT);INSERT INTO images(id) VALUES('i');").unwrap();db.execute_batch(SCHEMA).unwrap();db.execute_batch(queue::SCHEMA).unwrap();db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','key','{}','stopped',0,0)",[]).unwrap();db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned','sha','{}','cancelled')",[]).unwrap();db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,remote_id) VALUES('a','t',1,0,'unknown',0,12000,'known-id')",[]).unwrap();settle_remote_failure_db(&mut db,"a","confirmed terminal error").unwrap();let(state,reserve,cost):(String,u64,Option<u64>)=db.query_row("SELECT status,reserved_micros,cost_micros FROM processing_attempts WHERE id='a'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!(state,"remoteFailed");assert_eq!(reserve,12000);assert_eq!(cost,None);let task:String=db.query_row("SELECT status FROM processing_tasks WHERE id='t'",[],|r|r.get(0)).unwrap();assert_eq!(task,"cancelled");assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts WHERE status IN ('unknown','remotePending','running','reserved')",[],|r|r.get::<_,u32>(0)).unwrap(),0);}
 #[test]fn unknown_ledger_is_not_an_infinite_global_execution_slot(){
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png',1,1)",[]).unwrap();db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','owned','{\"providerMode\":\"fal\"}','stopped',1,1)",[]).unwrap();db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned','sha','{}','cancelled')",[]).unwrap();
  for(id,state,remote)in [("lost","unknown",None),("legacy","unknown",Some("legacy-id")),("known","remotePending",Some("known-id")),("live","running",None),("queued","reserved",None)]{db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,remote_id) VALUES(?1,'t',1,0,?2,1,37,?3)",params![id,state,remote]).unwrap();}
  assert_eq!(occupied_cloud_db(&db,"fal").unwrap(),3);assert_eq!(occupied_cloud_db(&db,"openrouter").unwrap(),0);assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts WHERE status='unknown'",[],|r|r.get::<_,i64>(0)).unwrap(),2);assert_eq!(db.query_row("SELECT SUM(reserved_micros) FROM processing_attempts",[],|r|r.get::<_,i64>(0)).unwrap(),185);
 }

}
#[path="processing_queue.rs"]
pub(super) mod queue;
#[path="processing_run.rs"]
pub(super) mod run;
#[path="processing_fields.rs"]
pub(super) mod fields;
