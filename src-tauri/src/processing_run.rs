//! Whole-workspace controls for the existing jobs coordinator, without another scheduler.
use super::*;

#[derive(Clone,Copy)]
pub(super) struct RunToken{pub id:u64,pub preflight:bool}

pub(super) fn candidates(db:&Connection)->Result<Vec<String>,String>{
    candidates_scoped(db,None)
}
pub(super) fn validate_scope(db:&Connection,job:Option<&str>,mode:Option<&str>)->Result<(),String>{
    if mode.is_some_and(|m|!matches!(m,"direct"|"manual")){return Err("Invalid queue mode scope".into());}
    if let (Some(id),Some(mode))=(job,mode){let actual:String=db.query_row("SELECT queue_mode FROM processing_jobs WHERE id=?1",[id],|r|r.get(0)).map_err(|_|"Queue job no longer exists")?;if actual!=mode{return Err("Queue job does not belong to the requested mode".into());}}
    Ok(())
}
pub(super) fn candidates_scoped(db:&Connection,mode:Option<&str>)->Result<Vec<String>,String>{
    validate_scope(db,None,mode)?;
    let mut q=db.prepare("SELECT id FROM processing_jobs j WHERE work_visible=1 AND (?1 IS NULL OR queue_mode=?1) AND status IN ('prepared','paused','interrupted','waitingService','queued','running') AND EXISTS(SELECT 1 FROM processing_tasks t WHERE t.job_id=j.id AND t.work_visible=1 AND t.status NOT IN ('succeeded','failed','cancelled')) ORDER BY created_at_unix,rowid").map_err(|e|e.to_string())?;
    let rows=q.query_map([mode],|r|r.get(0)).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())
}
pub(super) fn aggregate(db:&Connection)->Result<Value,String>{
    aggregate_scoped(db,None)
}
pub(super) fn aggregate_scoped(db:&Connection,mode:Option<&str>)->Result<Value,String>{
    validate_scope(db,None,mode)?;
    let (active,ready,stoppable):(u32,u32,u32)=db.query_row("SELECT COALESCE(SUM(status IN ('running','queued')),0),COALESCE(SUM(status IN ('prepared','paused','interrupted','waitingService')),0),COUNT(*) FROM processing_jobs j WHERE work_visible=1 AND (?1 IS NULL OR queue_mode=?1) AND status IN ('prepared','paused','interrupted','waitingService','queued','running') AND EXISTS(SELECT 1 FROM processing_tasks t WHERE t.job_id=j.id AND t.work_visible=1 AND t.status NOT IN ('succeeded','failed','cancelled'))",[mode],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    let (total,done,failed,cancelled,running,pending,retry_waiting,skipped):(u64,u64,u64,u64,u64,u64,u64,u64)=db.query_row("SELECT COUNT(*),COALESCE(SUM(t.status='succeeded'),0),COALESCE(SUM(t.status='failed'),0),COALESCE(SUM(t.status='cancelled'),0),COALESCE(SUM(t.status IN ('preparing','running','committing')),0),COALESCE(SUM(t.status IN ('pending','waitingRetry','waitingService')),0),COALESCE(SUM(t.status='waitingRetry'),0),COALESCE(SUM(t.status='skipped'),0) FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE j.work_visible=1 AND (?1 IS NULL OR j.queue_mode=?1) AND t.work_visible=1 AND j.status<>'preparing'",[mode],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).map_err(|e|e.to_string())?;
    let finished=done+failed+cancelled+skipped;
    Ok(json!({"active":active,"ready":ready,"stoppable":stoppable,"counters":{"total":total,"done":done,"failed":failed,"cancelled":cancelled,"running":running,"pending":pending,"retryWaiting":retry_waiting,"skipped":skipped,"finished":finished,"progressPercent":if total==0{0}else{finished*100/total}}}))
}

#[tauri::command]
pub async fn create_processing_queue(app:tauri::AppHandle,request_key:String)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<ProcessingRuntime>();let _gate=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
        if index_backups::restore_blocked(&app)?{return Err("Finish restoring before creating a queue.".into());}
        let id=queue::create_empty(&connection(&app)?,&request_key)?;notify_change(&app);Ok(json!({"jobId":id}))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn get_processing_queue_contexts(app:tauri::AppHandle,job_id:Option<String>,retry:Option<bool>,queue_mode:Option<String>)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let db=connection(&app)?;validate_scope(&db,job_id.as_deref(),queue_mode.as_deref())?;
        let mut q=db.prepare("SELECT json_extract(t.effective_config,'$.providerMode'),json_extract(t.effective_config,'$.model'),COUNT(*) FROM processing_task_context t JOIN processing_jobs j ON j.id=t.job_id WHERE j.work_visible=1 AND t.work_visible=1 AND (?1 IS NULL OR j.id=?1) AND (?3 IS NULL OR j.queue_mode=?3) AND j.status IN ('prepared','paused','interrupted','waitingService','queued','running','completed') AND (t.status NOT IN ('succeeded','failed','cancelled') OR (?2 AND t.status='failed')) GROUP BY 1,2").map_err(|e|e.to_string())?;
        let rows=q.query_map(params![job_id,retry.unwrap_or(false),queue_mode],|r|Ok(json!({"providerMode":r.get::<_,String>(0)?,"model":r.get::<_,String>(1)?,"total":r.get::<_,u64>(2)?}))).map_err(|e|e.to_string())?;
        Ok(Value::Array(rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?))
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn processing_queue_command(app:tauri::AppHandle,action:String,queue_mode:Option<String>)->Result<Value,String>{
    if queue_mode.as_deref()==Some("direct"){
        let id={let runtime=app.state::<ProcessingRuntime>();let _gate=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;let db=connection(&app)?;validate_scope(&db,None,Some("direct"))?;db.query_row("SELECT id FROM processing_jobs WHERE queue_mode='direct'",[],|r|r.get::<_,String>(0)).optional().map_err(|e|e.to_string())?};
        if !["start","resume","pause","stop","retry"].contains(&action.as_str()){return Err("Invalid direct queue command".into());}
        if let Some(id)=id{match job_command(app.clone(),id.clone(),action,None,None).await{Ok(_)=>return Ok(json!({"accepted":1,"errors":[]})),Err(error)=>return Ok(json!({"accepted":0,"errors":[{"jobId":id,"error":error}]}))}}
        return Ok(json!({"accepted":0,"errors":[]}));
    }
    {let db=connection(&app)?;validate_scope(&db,None,queue_mode.as_deref())?;}
    if !["start","resume","pause","stop"].contains(&action.as_str()){return Err("Invalid workspace queue command.".into());}
    let starting=["start","resume"].contains(&action.as_str());
    let (token,ids)={
        let runtime=app.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
        core.next_run=core.next_run.wrapping_add(1);let id=core.next_run;
        core.run=starting.then_some(RunToken{id,preflight:true});
        let db=connection(&app)?;
        if !starting{db.execute("UPDATE processing_jobs SET status='paused' WHERE work_visible=1 AND status IN ('running','queued') AND (?1 IS NULL OR queue_mode=?1)",[queue_mode.as_deref()]).map_err(|e|e.to_string())?;}
        (id,candidates_scoped(&db,queue_mode.as_deref())?)
    };
    notify_change(&app);
    let mut errors=Vec::new();let mut accepted=0;
    for id in ids{
        if starting&&!is_current(&app,token)?{return Ok(json!({"cancelled":true}));}
        if let Err(error)=job_command(app.clone(),id.clone(),if starting{"start".into()}else{action.clone()},None,Some(token)).await{
            if starting&&!is_current(&app,token)?{return Ok(json!({"cancelled":true}));}
            if starting{
                let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
                if !core.run.is_some_and(|r|r.id==token){return Ok(json!({"cancelled":true}));}
                let db=connection(&app)?;
                db.execute("UPDATE processing_jobs SET status='paused',error=?2 WHERE id=?1 AND status NOT IN ('stopped','completed')",params![id,error]).map_err(|e|e.to_string())?;
            }
            errors.push(json!({"jobId":id,"error":error}));
        }else{accepted+=1;}
    }
    if starting{
        let runtime=app.state::<ProcessingRuntime>();let mut core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;
        if let Some(run)=core.run.as_mut().filter(|r|r.id==token){run.preflight=false;}
    }
    notify_change(&app);Ok(json!({"accepted":accepted,"errors":errors}))
}
fn is_current(app:&tauri::AppHandle,id:u64)->Result<bool,String>{let runtime=app.state::<ProcessingRuntime>();let core=runtime.core.lock().map_err(|_|"Coordinator lock failed")?;Ok(core.run.is_some_and(|r|r.id==id))}
