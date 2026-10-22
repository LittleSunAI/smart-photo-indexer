//! Small persistence helpers for the existing jobs/tasks/attempts coordinator.
use super::*;

pub const SCHEMA:&str=include_str!("processing-queue.sql");

pub(super) fn create_empty(db:&Connection,key:&str)->Result<String,String>{
    if key.trim().is_empty()||key.len()>160{return Err("Invalid queue creation identity.".into());}
    let id=stable_text_id("job",&format!("empty:{key}"));
    let config=json!({"providerMode":"local","model":"","emptyQueue":true}).to_string();
    db.execute("INSERT OR IGNORE INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix,queue_mode) VALUES(?1,?2,?3,'prepared',?4,?4,'manual')",params![id,format!("empty:{key}"),config,unix_now()]).map_err(|e|e.to_string())?;
    let visible:bool=db.query_row("SELECT work_visible FROM processing_jobs WHERE id=?1",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if !visible{return Err("This queue was removed. Create a new queue.".into());}Ok(id)
}

pub(super) fn context(config:&JobConfig,credential:Option<&str>,receipt:Option<&str>)->Value{
    json!({"config":config,"credentialRef":credential,"receiptId":receipt})
}
pub(super) fn begin(db:&mut Connection,key:&str,mode:&str,config:&JobConfig,credential:Option<&str>)->Result<(String,String,Option<Value>),String>{
    begin_target(db,key,mode,config,credential,None)
}
pub(super) fn begin_target(db:&mut Connection,key:&str,mode:&str,config:&JobConfig,credential:Option<&str>,target:Option<&str>)->Result<(String,String,Option<Value>),String>{
    if !["direct","manual"].contains(&mode){return Err("Unknown queue mode.".into());}
    if target.is_some()&&mode!="manual"{return Err("Only manual queues accept an explicit destination.".into());}
    let receipt=stable_text_id("processing-command",key);
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    if let Some((job,status,accepted,skipped))=tx.query_row("SELECT job_id,status,accepted_count,skipped_count FROM processing_commands WHERE id=?1",[&receipt],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u64>(2)?,r.get::<_,u64>(3)?))).optional().map_err(|e|e.to_string())?{
        if target.is_some_and(|id|id!=job){return Err("Command receipt belongs to a different queue.".into());}
        let saved:Value=tx.query_row("SELECT execution_context FROM processing_commands WHERE id=?1",[&receipt],|r|r.get::<_,String>(0)).map_err(|e|e.to_string()).and_then(|s|serde_json::from_str(&s).map_err(|e|e.to_string()))?;
        if saved["targetJobId"].as_str()!=target||saved["queueMode"].as_str().is_some_and(|saved|saved!=mode){return Err("Command receipt has a different queue destination or mode.".into());}
        if status=="preparing"{return Err("This preparation is already running.".into());}
        if ["failed","cancelled"].contains(&status.as_str()){return Err("Preparation was cancelled or interrupted; create a new command.".into());}
        return Ok((job.clone(),receipt,Some(json!({"jobId":job,"duplicate":true,"totalCount":accepted,"skippedCount":skipped,"readyCount":saved["readyCount"].as_u64().unwrap_or(0),"duplicateCount":saved["duplicateCount"].as_u64().unwrap_or(0),"alreadyReady":accepted==0&&saved["duplicateCount"].as_u64().unwrap_or(0)==0}))));
    }
    if let Some(id)=target{
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id=?1 AND queue_mode='manual' AND work_visible=1 AND status<>'preparing')",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !valid{return Err("Selected manual queue is no longer available. Choose a queue or create a new one.".into());}
    }
    let id=if let Some(id)=target{id.to_owned()}else if mode=="direct"{
        tx.query_row("SELECT id FROM processing_jobs WHERE queue_mode='direct'",[],|r|r.get::<_,String>(0)).optional().map_err(|e|e.to_string())?.unwrap_or_else(||stable_text_id("job","persistent-direct-v1"))
    }else{stable_text_id("job",key)};
    // Old pre-receipt commands keep their exact identity and old semantics.
    if mode=="manual"&&target.is_none()&&tx.query_row("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id=?1)",[&id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{
        return Ok((id.clone(),receipt,Some(json!({"jobId":id,"duplicate":true}))));
    }
    tx.execute("INSERT OR IGNORE INTO processing_jobs(id,request_key,config,status,credential_ref,requested_concurrency,created_at_unix,updated_at_unix,queue_mode) VALUES(?1,?2,?3,?4,?5,?6,?7,?7,?8)",params![id,if mode=="direct"{"persistent-direct-v1"}else{key},serde_json::to_string(config).map_err(|e|e.to_string())?,if mode=="direct"{"paused"}else{"preparing"},credential,if mode=="direct"{0}else{config.concurrency},unix_now(),mode]).map_err(|e|e.to_string())?;
    let mut command_context=context(config,credential,Some(&receipt));command_context["targetJobId"]=json!(target);command_context["queueMode"]=json!(mode);
    tx.execute("INSERT INTO processing_commands(id,request_key,job_id,status,execution_context,created_at_unix,updated_at_unix) VALUES(?1,?2,?3,'preparing',?4,?5,?5)",params![receipt,key,id,command_context.to_string(),unix_now()]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())?;Ok((id,receipt,None))
}

pub(super) fn existing_direct(db:&Connection,job:&str,image:&str,path:&str)->Result<Option<(String,String,bool)>,String>{
    // Cloud receipts survive generations: their unknown cost must still be
    // reconciled. Local attempts have no remote receipt to recover. Only their
    // current live attempt blocks reuse; the runtime lease fences cancellation.
    db.query_row("SELECT t.id,t.status,EXISTS(SELECT 1 FROM processing_attempt_context a JOIN processing_jobs j ON j.id=a.job_id WHERE a.task_id=t.id AND a.status IN ('reserved','running','remotePending','unknown','responseSaved') AND (COALESCE(json_extract(a.effective_config,'$.providerMode'),'')<>'local' OR (a.status IN ('reserved','running','responseSaved') AND a.run_revision=t.run_revision AND a.generation=j.generation AND j.status NOT IN ('stopped','completed')))) FROM processing_tasks t WHERE job_id=?1 AND (image_id=?2 OR source_path=?3 COLLATE NOCASE) ORDER BY work_visible DESC,t.rowid LIMIT 1",params![job,image,path],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())
}
pub(super) fn reusable(status:&str,uncertain:bool,leased:bool)->bool{
    !uncertain&&!leased&&["succeeded","failed","cancelled","waitingService"].contains(&status)
}
pub(super) fn claim(db:&Connection,id:&str,job:&str,image:&str,path:&str,sha:&str,snapshot:&Value,plan:Option<&str>,reuse:bool)->Result<(),String>{
    let receipt=snapshot["_execution"]["receiptId"].as_str().ok_or("Missing command receipt.")?;
    let accepted:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_commands WHERE id=?1 AND status='preparing')",[receipt],|r|r.get(0)).map_err(|e|e.to_string())?;
    if !accepted{return Err("Preparation cancelled.".into());}
    let baseline:String=db.query_row("SELECT CASE WHEN status='queued' THEN COALESCE((SELECT json_extract(snapshot,'$._queuePriorStatus') FROM processing_tasks WHERE image_id=?1 AND json_extract(snapshot,'$._queuePriorStatus') IS NOT NULL ORDER BY rowid DESC LIMIT 1),'notProcessed') ELSE status END FROM images WHERE id=?1",[image],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut snapshot=snapshot.clone();snapshot["_queuePriorStatus"]=json!(baseline);
    if reuse{
        // The attempt ledger already contains normalized results; only this
        // run's replaceable continuation checkpoints are reset.
        db.execute("DELETE FROM processing_passes WHERE task_id=?1",[id]).map_err(|e|e.to_string())?;
        db.execute("UPDATE processing_tasks SET image_id=?2,source_path=?3,sha256=?4,snapshot=?5,status='pending',completed_passes=0,result=NULL,error=NULL,retry_at_unix=NULL,metadata_committed=0,rename_plan_id=?6,rename_status=?7,run_revision=run_revision+1,work_visible=1 WHERE id=?1",params![id,image,path,sha,snapshot.to_string(),plan,if plan.is_some(){"pending"}else{"none"}]).map_err(|e|e.to_string())?;
    }else{
        db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status,rename_plan_id,rename_status) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,?8)",params![id,job,image,path,sha,snapshot.to_string(),plan,if plan.is_some(){"pending"}else{"none"}]).map_err(|e|e.to_string())?;
    }Ok(())
}

pub(super) fn refresh_image_statuses(db:&Connection)->Result<Vec<Value>,String>{
    let sql="WITH states AS (SELECT t.image_id,MAX(t.work_visible=1 AND j.work_visible=1 AND j.status NOT IN ('stopped','completed','preparationFailed') AND t.status NOT IN ('succeeded','failed','cancelled','skipped')) AS live,MAX(t.work_visible=1 AND j.work_visible=1 AND t.status='failed') AS failed FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id GROUP BY t.image_id), baselines AS (SELECT image_id,json_extract(snapshot,'$._queuePriorStatus') AS prior,ROW_NUMBER() OVER (PARTITION BY image_id ORDER BY rowid DESC) AS ordinal FROM processing_tasks WHERE json_extract(snapshot,'$._queuePriorStatus') IS NOT NULL), desired AS (SELECT i.id,CASE WHEN i.status IN ('processed','missing','relocated') THEN i.status WHEN s.live THEN 'queued' WHEN s.failed THEN 'error' ELSE COALESCE(b.prior,CASE WHEN i.status='queued' THEN 'notProcessed' ELSE i.status END) END AS status FROM images i JOIN states s ON s.image_id=i.id LEFT JOIN baselines b ON b.image_id=i.id AND b.ordinal=1) UPDATE images SET status=(SELECT status FROM desired WHERE id=images.id) WHERE id IN (SELECT id FROM desired WHERE status<>images.status) RETURNING id,status";
    let mut q=db.prepare(sql).map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok(json!({"imageId":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?}))).map_err(|e|e.to_string())?;let mut changes:Vec<Value>=rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?;drop(q);
    for change in &mut changes{let mut q=db.prepare("SELECT path FROM image_paths WHERE image_id=?1 AND is_current=1").map_err(|e|e.to_string())?;let rows=q.query_map([change["imageId"].as_str().unwrap_or("")],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;change["paths"]=json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?);}Ok(changes)
}

pub(super) fn finish(db:&Connection,job:&str,receipt:&str,mode:&str,accepted:u64,ready:u64,skipped:u64,duplicates:u64)->Result<(),String>{
    let tx=rusqlite::Transaction::new_unchecked(db,rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let changed=tx.execute("UPDATE processing_commands SET status=?2,accepted_count=?3,skipped_count=?4,execution_context=json_set(execution_context,'$.readyCount',?5,'$.duplicateCount',?6),updated_at_unix=?7 WHERE id=?1 AND status='preparing'",params![receipt,if accepted==0{"completed"}else{"prepared"},accepted,skipped,ready,duplicates,unix_now()]).map_err(|e|e.to_string())?;
    if changed!=1{return Err("Preparation cancelled.".into());}
    if mode=="manual"&&accepted>0{
        tx.execute("UPDATE processing_jobs SET config=(SELECT effective_config FROM processing_task_context WHERE job_id=?1 AND work_visible=1 ORDER BY task_ordinal LIMIT 1),requested_concurrency=COALESCE((SELECT json_extract(effective_config,'$.concurrency') FROM processing_task_context WHERE job_id=?1 AND work_visible=1 ORDER BY task_ordinal LIMIT 1),1) WHERE id=?1 AND json_extract(config,'$.emptyQueue')=1",[job]).map_err(|e|e.to_string())?;
    }
    if mode=="manual"{tx.execute("UPDATE processing_jobs SET status=?2,error=NULL,updated_at_unix=?3 WHERE id=?1 AND (status='preparing' OR (?4>0 AND status IN ('completed','stopped','preparationFailed')))",params![job,if accepted==0{"completed"}else{"prepared"},unix_now(),accepted]).map_err(|e|e.to_string())?;}
    else{tx.execute("UPDATE processing_jobs SET status=CASE WHEN status IN ('completed','stopped') AND ?2>0 THEN 'paused' ELSE status END,updated_at_unix=?3 WHERE id=?1",params![job,accepted,unix_now()]).map_err(|e|e.to_string())?;}
    tx.commit().map_err(|e|e.to_string())?;Ok(())
}
pub(super) fn cancel_receipt(db:&Connection,key:&str)->Result<Vec<String>,String>{
    let receipt=stable_text_id("processing-command",key);
    let ids={let mut q=db.prepare("SELECT id FROM processing_task_context WHERE receipt_id=?1 AND work_visible=1").map_err(|e|e.to_string())?;let r=q.query_map([&receipt],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;r.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    db.execute("UPDATE processing_tasks SET work_visible=0,run_revision=run_revision+1,status='cancelled' WHERE id IN (SELECT id FROM processing_task_context WHERE receipt_id=?1 AND work_visible=1)",[&receipt]).map_err(|e|e.to_string())?;
    db.execute("UPDATE processing_commands SET status='cancelled',updated_at_unix=?2 WHERE id=?1",params![receipt,unix_now()]).map_err(|e|e.to_string())?;
    db.execute("UPDATE processing_jobs SET status='preparationFailed',error='Preparation cancelled' WHERE status='preparing' AND id IN (SELECT job_id FROM processing_commands WHERE id=?1)",[&receipt]).map_err(|e|e.to_string())?;Ok(ids)
}
pub(super) fn summaries(db:&Connection,offset:u32)->Result<Vec<Value>,String>{
    summaries_filtered(db,offset,false,false)
}
pub(super) fn summaries_filtered(db:&Connection,offset:u32,working_only:bool,hide_finished:bool)->Result<Vec<Value>,String>{
    summaries_scoped(db,offset,working_only,hide_finished,None)
}
pub(super) fn summaries_scoped(db:&Connection,offset:u32,working_only:bool,hide_finished:bool,mode:Option<&str>)->Result<Vec<Value>,String>{
    run::validate_scope(db,None,mode)?;
    let mut q=db.prepare("WITH page AS (SELECT rowid AS job_ordinal,* FROM processing_jobs j WHERE status<>'preparing' AND (?4 IS NULL OR queue_mode=?4) AND (NOT ?2 OR (work_visible=1 AND (json_extract(config,'$.emptyQueue')=1 OR EXISTS(SELECT 1 FROM processing_tasks t WHERE t.job_id=j.id AND t.work_visible=1)))) AND (NOT ?3 OR json_extract(config,'$.emptyQueue')=1 OR EXISTS(SELECT 1 FROM processing_tasks t WHERE t.job_id=j.id AND t.work_visible=1 AND t.status<>'succeeded')) ORDER BY created_at_unix DESC,rowid DESC LIMIT 100 OFFSET ?1) SELECT j.id,j.status,j.generation,json_remove(j.config,'$.analysisContract'),j.error,j.created_at_unix,j.task_revision,j.queue_mode,COUNT(t.id),COALESCE(SUM(t.status='succeeded'),0),COALESCE(SUM(t.status='failed'),0),COALESCE(SUM(t.status IN ('preparing','running','committing')),0),j.work_visible FROM page j LEFT JOIN processing_tasks t ON t.job_id=j.id AND (t.work_visible=1 OR j.work_visible=0) GROUP BY j.id ORDER BY j.created_at_unix DESC,j.job_ordinal DESC").map_err(|e|e.to_string())?;
    let rows=q.query_map(params![offset.min(100000),working_only,hide_finished,mode],|r|Ok(json!({"jobId":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"generation":r.get::<_,i64>(2)?,"config":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).unwrap_or(Value::Null),"error":r.get::<_,Option<String>>(4)?,"createdAtUnix":r.get::<_,u64>(5)?,"revision":r.get::<_,i64>(6)?,"queueMode":r.get::<_,String>(7)?,"total":r.get::<_,u64>(8)?,"done":r.get::<_,u64>(9)?,"failed":r.get::<_,u64>(10)?,"active":r.get::<_,u64>(11)?,"workVisible":r.get::<_,bool>(12)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
pub(super) fn task_rows(db:&Connection,job:&str,offset:u32,limit:u32,include_excluded:bool,hide_finished:bool)->Result<Vec<Value>,String>{
    let mut stmt=db.prepare("SELECT id,source_path,status,completed_passes,COALESCE(result,(SELECT normalized_result FROM processing_attempts a WHERE a.task_id=t.id AND a.normalized_result IS NOT NULL ORDER BY a.rowid DESC LIMIT 1)),error,metadata_committed,rename_status,image_id FROM processing_tasks t WHERE job_id=?1 AND (?4 OR work_visible=1) AND (NOT ?5 OR status<>'succeeded') ORDER BY t.rowid LIMIT ?2 OFFSET ?3").map_err(|e|e.to_string())?;
    let rows=stmt.query_map(params![job,limit.clamp(1,500),offset,include_excluded,hide_finished],|r|Ok(json!({"id":r.get::<_,String>(0)?,"path":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"completedPasses":r.get::<_,u32>(3)?,"result":r.get::<_,Option<String>>(4)?.and_then(|s|serde_json::from_str::<Value>(&s).ok()),"error":r.get::<_,Option<String>>(5)?,"metadataCommitted":r.get::<_,bool>(6)?,"renameStatus":r.get::<_,String>(7)?,"imageId":r.get::<_,String>(8)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
pub(super) fn recover_preparations(db:&Connection)->Result<(),String>{
    db.execute("UPDATE processing_tasks SET work_visible=0,run_revision=run_revision+1,status='cancelled' WHERE id IN (SELECT t.id FROM processing_task_context t JOIN processing_commands c ON c.id=t.receipt_id WHERE c.status='preparing' AND t.work_visible=1)",[]).map_err(|e|e.to_string())?;
    db.execute("UPDATE processing_commands SET status='failed',error='Preparation interrupted',updated_at_unix=?1 WHERE status='preparing'",[unix_now()]).map_err(|e|e.to_string())?;Ok(())
}
pub(super) fn remove(db:&Connection,scope:&str,job:Option<&str>,task:Option<&str>)->Result<(usize,Vec<String>),String>{
    remove_scoped(db,scope,job,task,None)
}
pub(super) fn remove_scoped(db:&Connection,scope:&str,job:Option<&str>,task:Option<&str>,mode:Option<&str>)->Result<(usize,Vec<String>),String>{
    run::validate_scope(db,job,mode)?;
    if !["photo","job","allWorking"].contains(&scope)||scope!="allWorking"&&job.is_none()||scope=="photo"&&task.is_none(){return Err("Invalid queue removal scope or identity.".into());}
    if let Some(job)=job{if !db.query_row("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id=?1)",[job],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return Err("Queue job no longer exists.".into());}}
    let predicate="t.work_visible=1 AND (?4 IS NULL OR t.job_id IN (SELECT id FROM processing_jobs WHERE queue_mode=?4)) AND (?1='allWorking' OR (?1='job' AND t.job_id=?2) OR (?1='photo' AND t.job_id=?2 AND t.id=?3))";
    let attempts={let mut q=db.prepare(&format!("SELECT a.id FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE {predicate} AND a.run_revision=t.run_revision AND a.status IN ('reserved','running','remotePending','unknown')")).map_err(|e|e.to_string())?;let rows=q.query_map(params![scope,job,task,mode],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    // Freeze the attempt IDs before a later append may reuse the task row.
    for id in &attempts{db.execute("UPDATE processing_attempts SET status='notSent',reserved_micros=0,cost_micros=0,error='Removed before submit' WHERE id=?1 AND status='reserved'",[id]).map_err(|e|e.to_string())?;}
    let count=db.execute(&format!("UPDATE processing_tasks AS t SET work_visible=0,run_revision=run_revision+1,status=CASE WHEN status IN ('succeeded','failed') THEN status ELSE 'cancelled' END,retry_at_unix=NULL WHERE {predicate}"),params![scope,job,task,mode]).map_err(|e|e.to_string())?;
    if scope!="photo"{
        db.execute("UPDATE processing_commands SET status='cancelled',updated_at_unix=?3 WHERE status='preparing' AND (?4 IS NULL OR job_id IN (SELECT id FROM processing_jobs WHERE queue_mode=?4)) AND (?1='allWorking' OR job_id=?2)",params![scope,job,unix_now(),mode]).map_err(|e|e.to_string())?;
        db.execute("UPDATE processing_jobs SET status='stopped',generation=generation+1,work_visible=CASE WHEN queue_mode='direct' THEN 1 ELSE 0 END,updated_at_unix=?3 WHERE work_visible=1 AND (?4 IS NULL OR queue_mode=?4) AND (?1='allWorking' OR id=?2)",params![scope,job,unix_now(),mode]).map_err(|e|e.to_string())?;
    }
    Ok((count,attempts))
}
pub(super) fn freeze(snapshot:&mut Value,config:&JobConfig,credential:Option<&str>,receipt:&str)->Result<(),String>{
    let object=snapshot.as_object_mut().ok_or("Invalid processing source snapshot.")?;
    object.insert("_execution".into(),context(config,credential,Some(receipt)));Ok(())
}
pub(super) fn task_context(db:&Connection,task:&str)->Result<(JobConfig,Option<String>,Option<String>),String>{
    let(raw,key,receipt):(String,Option<String>,Option<String>)=db.query_row("SELECT effective_config,effective_credential_ref,receipt_id FROM processing_task_context WHERE id=?1",[task],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    Ok((serde_json::from_str(&raw).map_err(|e|e.to_string())?,key,receipt))
}
pub(super) fn attempt_work(db:&Connection,attempt:&str)->Result<Work,String>{
    db.query_row("SELECT job_id,task_id,image_id,source_path,sha256,generation,run_revision,effective_config,effective_credential_ref FROM processing_attempt_context WHERE id=?1",[attempt],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,Option<String>>(8)?))).map_err(|e|e.to_string()).and_then(|(job_id,task_id,image_id,path,sha,generation,run_revision,raw,credential_ref)|Ok(Work{job_id,task_id,image_id,path,sha,generation,run_revision,config:serde_json::from_str(&raw).map_err(|e|e.to_string())?,credential_ref}))
}
pub(super) fn ready_contexts(db:&Connection,job:&str,receipt:Option<&str>,retry:bool)->Result<Vec<(JobConfig,Option<String>)>,String>{
    let mut q=db.prepare("SELECT DISTINCT effective_config,effective_credential_ref FROM processing_task_context WHERE job_id=?1 AND work_visible=1 AND (?2 IS NULL OR receipt_id=?2) AND (status IN ('pending','waitingRetry','preparing','running','committing') OR (?3 AND status='failed'))").map_err(|e|e.to_string())?;
    let rows=q.query_map(params![job,receipt,retry],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?))).map_err(|e|e.to_string())?;
    rows.map(|r|{let(raw,key)=r.map_err(|e|e.to_string())?;Ok((serde_json::from_str(&raw).map_err(|e|e.to_string())?,key))}).collect()
}

pub(super) fn replace_key(db:&Connection,job:&str,provider:&str,key:&str)->Result<usize,String>{
    let rows={let mut q=db.prepare("SELECT id,snapshot,receipt_id,effective_config FROM processing_task_context WHERE job_id=?1 AND work_visible=1 AND status NOT IN ('succeeded','cancelled') AND json_extract(effective_config,'$.providerMode')=?2").map_err(|e|e.to_string())?;
        let mapped=q.query_map(params![job,provider],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?;
        mapped.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    for(id,snapshot,receipt,config) in &rows{
        let mut snapshot:Value=serde_json::from_str(snapshot).map_err(|e|e.to_string())?;
        let config:JobConfig=serde_json::from_str(config).map_err(|e|e.to_string())?;
        snapshot["_execution"]=context(&config,Some(key),receipt.as_deref());
        db.execute("UPDATE processing_tasks SET snapshot=?2 WHERE id=?1",params![id,snapshot.to_string()]).map_err(|e|e.to_string())?;
    }Ok(rows.len())
}

/// Rebuild only the current run's checkpoint; every paid response still gets
/// a durable normalized history row, including an obsolete/removed task run.
pub(super) fn recover_saved(db:&mut Connection)->Result<(),String>{
    let saved={let mut q=db.prepare("SELECT id,task_id,pass_number,response,effective_config,run_revision FROM processing_attempt_context WHERE status='responseSaved'").map_err(|e|e.to_string())?;
        let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,u32>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?))).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    for(id,task,pass,raw,config,revision) in saved{
        let config:JobConfig=serde_json::from_str(&config).map_err(|e|e.to_string())?;
        let raw:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let current:bool=tx.query_row("SELECT t.run_revision=?2 AND t.work_visible=1 AND t.status NOT IN ('cancelled','succeeded') AND j.status NOT IN ('stopped','preparationFailed') AND j.generation=(SELECT generation FROM processing_attempts WHERE id=?3) FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.id=?1",params![task,revision,id],|r|r.get(0)).map_err(|e|e.to_string())?;
        match provider_transport::decode_reply(&config.provider_mode,&raw,config.analysis_contract.as_ref()){
            Ok(result)=>{
                let result=result.to_string();
                tx.execute("UPDATE processing_attempts SET normalized_result=?2,status=?3 WHERE id=?1",params![id,result,if current{"succeeded"}else{"lateResponse"}]).map_err(|e|e.to_string())?;
                if current{
                    tx.execute("INSERT OR IGNORE INTO processing_passes(task_id,pass_number,result,attempt_id) VALUES(?1,?2,?3,?4)",params![task,pass,result,id]).map_err(|e|e.to_string())?;
                    tx.execute("UPDATE processing_tasks SET completed_passes=?2,result=?3,status='pending' WHERE id=?1 AND completed_passes<?2",params![task,pass,result]).map_err(|e|e.to_string())?;
                }
            },
            Err(error)=>{
                tx.execute("UPDATE processing_attempts SET status='failed',error=?2 WHERE id=?1",params![id,error]).map_err(|e|e.to_string())?;
                if current{tx.execute("UPDATE processing_tasks SET status='failed',error=?2 WHERE id=?1",params![task,error]).map_err(|e|e.to_string())?;}
            }
        }
        tx.commit().map_err(|e|e.to_string())?;
    }Ok(())
}

// A confirmed service-wide rejection must not spend one request per remaining photo.
// Only unsent work is changed; active/uncertain receipts continue reconciliation.
pub(super) fn fail_unsent_group(db:&Connection,w:&Work,group:crate::provider_transport::FailureGroup,message:&str)->Result<usize,String>{
    db.execute("UPDATE processing_tasks SET status='failed',retry_at_unix=NULL,error=?1 WHERE id IN (SELECT t.id FROM processing_task_context t JOIN processing_jobs j ON j.id=t.job_id WHERE t.work_visible=1 AND j.work_visible=1 AND t.status IN ('pending','waitingRetry') AND json_extract(t.effective_config,'$.providerMode')=?2 AND t.effective_credential_ref IS ?3 AND (?4=0 OR json_extract(t.effective_config,'$.model')=?5) AND NOT EXISTS(SELECT 1 FROM processing_attempts a WHERE a.task_id=t.id AND a.run_revision=t.run_revision AND a.status IN ('reserved','running','remotePending','unknown','responseSaved')))",params![format!("Сервис недоступен для этой группы: {message}. После изменения ключа/модели можно повторить."),w.config.provider_mode,w.credential_ref,group==crate::provider_transport::FailureGroup::Model,w.config.model]).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn mode_scope_precedes_pagination_commands_and_removal_and_preserves_other_mode(){
        let mut db=working();let c=config("local","scope-fixture");let mut s=json!({});
        let(d,r,_)=begin(&mut db,"old-direct","direct",&c,None).unwrap();freeze(&mut s,&c,None,&r).unwrap();claim(&db,"direct-photo",&d,"i","direct.png","sha",&s,None,false).unwrap();finish(&db,&d,&r,"direct",1,0,0,0).unwrap();
        db.execute("UPDATE processing_jobs SET created_at_unix=1 WHERE id=?1",[&d]).unwrap();
        for n in 0..105{let key=format!("scope-{n}");let(id,r,_)=begin(&mut db,&key,"manual",&c,None).unwrap();freeze(&mut s,&c,None,&r).unwrap();claim(&db,&format!("scope-photo-{n}"),&id,"i",&format!("scope-{n}.png"),"sha",&s,None,false).unwrap();finish(&db,&id,&r,"manual",1,0,0,0).unwrap();}
        assert!(!summaries_filtered(&db,0,true,false).unwrap().iter().any(|j|j["jobId"]==d));
        let direct=summaries_scoped(&db,0,true,false,Some("direct")).unwrap();assert_eq!(direct.len(),1);assert_eq!(direct[0]["jobId"],d);
        assert_eq!(run::candidates_scoped(&db,Some("direct")).unwrap(),vec![d.clone()]);
        assert_eq!(run::aggregate_scoped(&db,Some("direct")).unwrap()["counters"]["total"],1);
        assert!(run::validate_scope(&db,Some(&d),Some("manual")).is_err());assert!(summaries_scoped(&db,0,true,false,Some("bad")).is_err());
        assert!(remove_scoped(&db,"job",Some(&d),None,Some("manual")).is_err());
        let before=run::aggregate_scoped(&db,Some("manual")).unwrap();let attempts:i64=db.query_row("SELECT COUNT(*) FROM processing_attempts",[],|r|r.get(0)).unwrap();
        assert_eq!(remove_scoped(&db,"allWorking",None,None,Some("direct")).unwrap().0,1);
        assert_eq!(run::aggregate_scoped(&db,Some("manual")).unwrap(),before);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts",[],|r|r.get::<_,i64>(0)).unwrap(),attempts);
        assert_eq!(db.query_row("SELECT queue_mode FROM processing_jobs WHERE id=?1",[&d],|r|r.get::<_,String>(0)).unwrap(),"direct");
        assert!(run::candidates_scoped(&db,Some("direct")).unwrap().is_empty());
    }
    #[test]
    fn image_status_considers_other_live_jobs_and_cancel_restores_original_without_marking_done(){
        let mut db=working();let c=config("local","status");let mut snapshot=json!({});
        let(a,ar,_)=begin(&mut db,"status-a","manual",&c,None).unwrap();freeze(&mut snapshot,&c,None,&ar).unwrap();claim(&db,"status-a",&a,"i","a.png","sha",&snapshot,None,false).unwrap();finish(&db,&a,&ar,"manual",1,0,0,0).unwrap();refresh_image_statuses(&db).unwrap();
        let status=|db:&Connection|db.query_row("SELECT status FROM images WHERE id='i'",[],|r|r.get::<_,String>(0)).unwrap();assert_eq!(status(&db),"queued");
        let(b,br,_)=begin(&mut db,"status-b","manual",&c,None).unwrap();freeze(&mut snapshot,&c,None,&br).unwrap();claim(&db,"status-b",&b,"i","b.png","sha",&snapshot,None,false).unwrap();finish(&db,&b,&br,"manual",1,0,0,0).unwrap();remove(&db,"job",Some(&a),None).unwrap();refresh_image_statuses(&db).unwrap();assert_eq!(status(&db),"queued");
        db.execute("UPDATE processing_tasks SET status='failed' WHERE id='status-b'",[]).unwrap();refresh_image_statuses(&db).unwrap();assert_eq!(status(&db),"queued"); // working() still has another live task for i.
        db.execute("UPDATE processing_tasks SET work_visible=0 WHERE id='t'",[]).unwrap();refresh_image_statuses(&db).unwrap();assert_eq!(status(&db),"error");remove(&db,"job",Some(&b),None).unwrap();refresh_image_statuses(&db).unwrap();assert_eq!(status(&db),"notProcessed");
        db.execute("UPDATE images SET status='processed' WHERE id='i'",[]).unwrap();refresh_image_statuses(&db).unwrap();assert_eq!(status(&db),"processed");
    }
    #[test]
    fn empty_queue_is_persistent_visible_idempotent_and_first_append_freezes_config(){
        let mut db=working();let id=create_empty(&db,"empty-owned").unwrap();assert_eq!(id,create_empty(&db,"empty-owned").unwrap());
        assert!(summaries_filtered(&db,0,true,true).unwrap().iter().any(|j|j["jobId"]==id&&j["total"]==0&&j["status"]=="prepared"));
        assert!(!run::candidates(&db).unwrap().contains(&id));
        let mut c=config("local","first-model");c.concurrency=3;
        let(same,r,_)=begin_target(&mut db,"append-empty","manual",&c,None,Some(&id)).unwrap();assert_eq!(same,id);
        finish(&db,&id,&r,"manual",0,1,0,0).unwrap();assert_eq!(db.query_row("SELECT json_extract(config,'$.emptyQueue') FROM processing_jobs WHERE id=?1",[&id],|r|r.get::<_,u32>(0)).unwrap(),1);
        let(_,r,_)=begin_target(&mut db,"append-real","manual",&c,None,Some(&id)).unwrap();let mut s=json!({});freeze(&mut s,&c,None,&r).unwrap();claim(&db,"empty-photo",&id,"i","empty.png","sha",&s,None,false).unwrap();finish(&db,&id,&r,"manual",1,0,0,0).unwrap();
        assert_eq!(db.query_row("SELECT json_extract(config,'$.model'),requested_concurrency FROM processing_jobs WHERE id=?1",[&id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?))).unwrap(),("first-model".into(),3));
        c.model="second-model".into();c.concurrency=9;let(_,r,_)=begin_target(&mut db,"append-again","manual",&c,None,Some(&id)).unwrap();freeze(&mut s,&c,None,&r).unwrap();claim(&db,"second-photo",&id,"i","second.png","sha",&s,None,false).unwrap();finish(&db,&id,&r,"manual",1,0,0,0).unwrap();
        assert_eq!(task_context(&db,"second-photo").unwrap().0.model,"second-model");assert_eq!(db.query_row("SELECT requested_concurrency FROM processing_jobs WHERE id=?1",[&id],|r|r.get::<_,u32>(0)).unwrap(),3);
        let other=create_empty(&db,"empty-other").unwrap();remove(&db,"job",Some(&other),None).unwrap();assert!(create_empty(&db,"empty-other").is_err());remove(&db,"allWorking",None,None).unwrap();assert!(summaries_filtered(&db,0,true,true).unwrap().is_empty());
    }
    #[test]
    fn whole_queue_candidates_and_aggregate_ignore_pagination_and_terminal_or_empty_groups(){
        let mut db=working();let c=config("local","owned");let mut s=json!({});
        for n in 0..105{let key=format!("all-{n}");let(id,r,_)=begin(&mut db,&key,"manual",&c,None).unwrap();freeze(&mut s,&c,None,&r).unwrap();claim(&db,&format!("all-photo-{n}"),&id,"i",&format!("all-{n}.png"),"sha",&s,None,false).unwrap();finish(&db,&id,&r,"manual",1,0,0,0).unwrap();}
        create_empty(&db,"empty-skipped").unwrap();assert_eq!(summaries_filtered(&db,0,true,false).unwrap().len(),100);
        let ids=run::candidates(&db).unwrap();assert_eq!(ids.len(),106); // working() includes one existing pending job.
        let aggregate=run::aggregate(&db).unwrap();assert_eq!(aggregate["ready"],105);assert_eq!(aggregate["active"],1);
        db.execute("UPDATE processing_tasks SET status='cancelled'",[]).unwrap();assert!(run::candidates(&db).unwrap().is_empty());assert_eq!(run::aggregate(&db).unwrap()["stoppable"],0);
    }
    #[test]
    fn local_cancel_timeout_history_does_not_block_explicit_retry_or_erase_cloud_receipts(){
        let db=working();
        db.execute("UPDATE processing_tasks SET status='waitingService' WHERE id='t'",[]).unwrap();
        attempt(&db,"local-timeout","local","unknown",0);
        let row=existing_direct(&db,"j","i","owned.png").unwrap().unwrap();
        assert!(reusable(&row.1,row.2,false));assert!(!reusable(&row.1,row.2,true));
        attempt(&db,"local-old-running","local","running",0);
        assert!(existing_direct(&db,"j","i","owned.png").unwrap().unwrap().2);
        db.execute("UPDATE processing_jobs SET generation=1,status='stopped' WHERE id='j'",[]).unwrap();
        assert!(!existing_direct(&db,"j","i","owned.png").unwrap().unwrap().2);
        attempt(&db,"paid-old","fal","unknown",0);
        db.execute("UPDATE processing_attempts SET remote_id='paid-receipt' WHERE id='paid-old'",[]).unwrap();
        assert!(existing_direct(&db,"j","i","owned.png").unwrap().unwrap().2);
        db.execute("UPDATE processing_attempts SET status='remoteFailed' WHERE id='paid-old'",[]).unwrap();
        assert!(!existing_direct(&db,"j","i","owned.png").unwrap().unwrap().2);
        assert_eq!(db.query_row("SELECT COUNT(*),SUM(reserved_micros),SUM(cost_micros) FROM processing_attempts",[],|r|Ok((r.get::<_,u32>(0)?,r.get::<_,u32>(1)?,r.get::<_,u32>(2)?))).unwrap(),(3,111,57));
    }
    #[test]
    fn cancelled_local_reuse_keeps_single_row_and_late_attempt_ownership(){
        let mut db=working();attempt(&db,"old-local","local","unknown",0);
        db.execute("UPDATE processing_tasks SET status='cancelled',work_visible=0,run_revision=1 WHERE id='t'",[]).unwrap();
        let c=config("local","new-model");let(_,receipt,_)=begin_target(&mut db,"retry-local","manual",&c,None,Some("j")).unwrap();
        let row=existing_direct(&db,"j","i","renamed.png").unwrap().unwrap();assert!(reusable(&row.1,row.2,false));
        let mut s=json!({});freeze(&mut s,&c,None,&receipt).unwrap();claim(&db,"t","j","i","renamed.png","sha",&s,None,true).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*),MAX(run_revision) FROM processing_tasks",[],|r|Ok((r.get::<_,u32>(0)?,r.get::<_,u32>(1)?))).unwrap(),(1,2));
        assert_eq!(attempt_work(&db,"old-local").unwrap().config.model,"attempt-model");
        assert_eq!(attempt_work(&db,"old-local").unwrap().run_revision,0);
        assert_eq!(task_context(&db,"t").unwrap().0.model,"new-model");
    }
    #[test]
    fn manual_target_append_keeps_context_state_and_receipt_ownership(){
        let mut db=working();let c=config("local","old");
        let(a,first,_)=begin(&mut db,"manual-first","manual",&c,Some("old-key")).unwrap();
        let mut snapshot=json!({});freeze(&mut snapshot,&c,Some("old-key"),&first).unwrap();claim(&db,"old-photo",&a,"i","old.png","sha",&snapshot,None,false).unwrap();finish(&db,&a,&first,"manual",1,0,0,0).unwrap();
        let(b,_,_)=begin(&mut db,"manual-other","manual",&c,None).unwrap();assert_ne!(a,b);
        for (index,status) in ["prepared","paused","running","completed","stopped","preparationFailed"].iter().enumerate(){
            db.execute("UPDATE processing_jobs SET status=?2 WHERE id=?1",params![a,status]).unwrap();let key=format!("append-{index}");let mut next=c.clone();next.model="new".into();
            let(same,receipt,_)=begin_target(&mut db,&key,"manual",&next,Some("new-key"),Some(&a)).unwrap();assert_eq!(same,a);
            let mut s=json!({});freeze(&mut s,&next,Some("new-key"),&receipt).unwrap();let photo=format!("photo-{index}");claim(&db,&photo,&a,"i",&format!("new-{index}.png"),"sha",&s,None,false).unwrap();
            finish(&db,&a,&receipt,"manual",1,2,3,4).unwrap();
            let expected=if ["completed","stopped","preparationFailed"].contains(status){"prepared"}else{status};assert_eq!(db.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&a],|r|r.get::<_,String>(0)).unwrap(),expected);
            let replay=begin_target(&mut db,&key,"manual",&next,None,Some(&a)).unwrap().2.unwrap();assert_eq!(replay["readyCount"],2);assert_eq!(replay["duplicateCount"],4);
            assert!(begin_target(&mut db,&key,"manual",&next,None,Some(&b)).is_err());assert!(begin_target(&mut db,&key,"manual",&next,None,None).is_err());
            cancel_receipt(&db,&key).unwrap();assert!(db.query_row("SELECT work_visible FROM processing_tasks WHERE id='old-photo'",[],|r|r.get::<_,bool>(0)).unwrap());
            assert_eq!(task_context(&db,"old-photo").unwrap().0.model,"old");assert_eq!(db.query_row("SELECT json_extract(config,'$.model') FROM processing_jobs WHERE id=?1",[&a],|r|r.get::<_,String>(0)).unwrap(),"old");
        }
        assert!(begin_target(&mut db,"wrong-direct","direct",&c,None,Some(&a)).is_err());
        db.execute("UPDATE processing_jobs SET work_visible=0 WHERE id=?1",[&a]).unwrap();assert!(begin_target(&mut db,"deleted-target","manual",&c,None,Some(&a)).is_err());
    }
    #[test]
    fn interrupted_manual_append_only_excludes_unaccepted_receipt_rows(){
        let mut db=working();let c=config("local","old");let(a,first,_)=begin(&mut db,"accepted","manual",&c,None).unwrap();
        let mut s=json!({});freeze(&mut s,&c,None,&first).unwrap();claim(&db,"kept",&a,"i","old.png","sha",&s,None,false).unwrap();finish(&db,&a,&first,"manual",1,0,0,0).unwrap();db.execute("UPDATE processing_jobs SET status='paused' WHERE id=?1",[&a]).unwrap();
        let(_,pending,_)=begin_target(&mut db,"interrupted","manual",&c,None,Some(&a)).unwrap();freeze(&mut s,&c,None,&pending).unwrap();claim(&db,"excluded",&a,"i","new.png","sha",&s,None,false).unwrap();
        recover_preparations(&db).unwrap();recover_preparations(&db).unwrap();
        assert_eq!(db.query_row("SELECT work_visible FROM processing_tasks WHERE id='kept'",[],|r|r.get::<_,bool>(0)).unwrap(),true);assert_eq!(db.query_row("SELECT work_visible,run_revision FROM processing_tasks WHERE id='excluded'",[],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,i64>(1)?))).unwrap(),(false,1));assert_eq!(db.query_row("SELECT status FROM processing_jobs WHERE id=?1",[&a],|r|r.get::<_,String>(0)).unwrap(),"paused");
        assert!(begin_target(&mut db,"after-recovery","manual",&c,None,Some(&a)).is_ok());
    }
    #[test]
    fn selected_manual_row_lookup_tracks_image_identity_and_uncertain_retry_guards(){
        let mut db=working();let c=config("local","new");
        let row=existing_direct(&db,"j","i","renamed.png").unwrap().unwrap();assert_eq!(row.0,"t");assert!(!reusable(&row.1,row.2,false));
        db.execute("UPDATE processing_tasks SET status='failed' WHERE id='t'",[]).unwrap();attempt(&db,"uncertain","fal","unknown",0);
        let row=existing_direct(&db,"j","i","renamed.png").unwrap().unwrap();assert!(!reusable(&row.1,row.2,false));
        db.execute("UPDATE processing_attempts SET status='failed' WHERE id='uncertain'",[]).unwrap();let row=existing_direct(&db,"j","i","renamed.png").unwrap().unwrap();assert!(!reusable(&row.1,row.2,true));assert!(reusable(&row.1,row.2,false));
        let(_,receipt,_)=begin_target(&mut db,"terminal-retry","manual",&c,None,Some("j")).unwrap();let mut s=json!({});freeze(&mut s,&c,None,&receipt).unwrap();claim(&db,&row.0,"j","i","renamed.png","sha",&s,None,true).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_tasks WHERE job_id='j'",[],|r|r.get::<_,u64>(0)).unwrap(),1);assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts WHERE task_id='t'",[],|r|r.get::<_,u64>(0)).unwrap(),1);assert_eq!(db.query_row("SELECT run_revision FROM processing_tasks WHERE id='t'",[],|r|r.get::<_,u64>(0)).unwrap(),1);
    }
    #[test]
    fn working_and_succeeded_filters_precede_pagination_and_keep_history_defaults(){
        let mut db=working();let c=config("local","old");
        let(a,receipt,_)=begin(&mut db,"visible-old","manual",&c,None).unwrap();let mut s=json!({});freeze(&mut s,&c,None,&receipt).unwrap();
        for i in 0..105{let id=format!("visible-{i}");claim(&db,&id,&a,"i",&format!("{i}.png"),"sha",&s,None,false).unwrap();db.execute("UPDATE processing_tasks SET status=?2 WHERE id=?1",params![id,if i<100{"succeeded"}else{["failed","cancelled","pending","waitingRetry","skipped"][i-100]}]).unwrap();}finish(&db,&a,&receipt,"manual",105,0,0,0).unwrap();
        for i in 0..105{let(j,r,_)=begin(&mut db,&format!("hidden-{i}"),"manual",&c,None).unwrap();freeze(&mut s,&c,None,&r).unwrap();claim(&db,&format!("hidden-{i}"),&j,"i","hidden.png","sha",&s,None,false).unwrap();finish(&db,&j,&r,"manual",1,0,0,0).unwrap();db.execute("UPDATE processing_jobs SET work_visible=0 WHERE id=?1",[&j]).unwrap();}
        let jobs=summaries_filtered(&db,0,true,true).unwrap();assert!(jobs.iter().any(|j|j["jobId"]==a));assert!(summaries(&db,0).unwrap().len()==100);
        let rows=task_rows(&db,&a,0,3,false,true).unwrap();assert_eq!(rows.iter().map(|r|r["status"].as_str().unwrap()).collect::<Vec<_>>(),vec!["failed","cancelled","pending"]);assert_eq!(task_rows(&db,&a,3,3,false,true).unwrap().len(),2);assert_eq!(task_rows(&db,&a,0,3,false,false).unwrap()[0]["status"],"succeeded");
    }
    fn working()->Connection{
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png',1,1)",[]).unwrap();
        db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','legacy',?1,'running',1,1)",[serde_json::to_string(&config("local","legacy-model")).unwrap()]).unwrap();
        db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned.png','sha','{}','pending')",[]).unwrap();db
    }
    fn config(provider:&str,model:&str)->JobConfig{serde_json::from_value(json!({"providerMode":provider,"model":model})).unwrap()}
    fn result()->Value{json!({"description":"Saved description","tags":["cat"],"objects":["cat"],"scene":"room","confidence":0.9})}
    fn legacy()->Connection{
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE images(id TEXT PRIMARY KEY,status TEXT NOT NULL DEFAULT 'notProcessed');CREATE TABLE tasks(id TEXT PRIMARY KEY,status TEXT);INSERT INTO images(id) VALUES('i');").unwrap();
        db.execute_batch(super::super::SCHEMA).unwrap();
        db.execute("INSERT INTO processing_jobs(id,request_key,config,status,credential_ref,created_at_unix,updated_at_unix) VALUES('j','legacy',?1,'running','legacy-key',1,1)",[serde_json::to_string(&config("local","legacy-model")).unwrap()]).unwrap();
        db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned.png','sha','{}','pending')",[]).unwrap();db
    }
    fn attempt(db:&Connection,id:&str,provider:&str,status:&str,revision:i64){
        db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,run_revision,execution_context,response,cost_micros) VALUES(?1,'t',1,0,?2,2,37,?3,?4,?5,19)",params![id,status,revision,context(&config(provider,"attempt-model"),Some("sent-key"),None).to_string(),if provider=="controlled-test"{result()}else{json!({"output":result().to_string()})}.to_string()]).unwrap();
    }
    #[test]
    fn legacy_migration_preserves_money_passes_and_freezes_attempt_before_key_changes(){
        let db=legacy();let raw=result().to_string();
        db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,cost_micros,response) VALUES('old','t',1,0,'succeeded',1,37,19,?1)",[&raw]).unwrap();
        db.execute("INSERT INTO processing_passes(task_id,pass_number,result,attempt_id) VALUES('t',1,?1,'old')",[&raw]).unwrap();
        db.execute_batch(SCHEMA).unwrap();
        assert_eq!(db.query_row("SELECT queue_mode FROM processing_jobs WHERE id='j'",[],|r|r.get::<_,String>(0)).unwrap(),"manual");
        let before=attempt_work(&db,"old").unwrap();assert_eq!(before.config.model,"legacy-model");assert_eq!(before.credential_ref.as_deref(),Some("legacy-key"));
        assert_eq!(replace_key(&db,"j","local","future-key").unwrap(),1);
        assert_eq!(task_context(&db,"t").unwrap().1.as_deref(),Some("future-key"));
        assert_eq!(attempt_work(&db,"old").unwrap().credential_ref.as_deref(),Some("legacy-key"));
        let saved:(i64,i64,String,String)=db.query_row("SELECT reserved_micros,cost_micros,response,normalized_result FROM processing_attempts WHERE id='old'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(saved,(37,19,raw.clone(),raw));assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_passes",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
    #[test]
    fn mixed_task_context_explicit_null_and_attempt_context_are_independent(){
        let db=legacy();db.execute_batch(SCHEMA).unwrap();
        let mut snapshot=json!({"sourceItemId":"original"});freeze(&mut snapshot,&config("fal","new-model"),Some("task-key"),"receipt").unwrap();
        db.execute("UPDATE processing_tasks SET snapshot=?1",[snapshot.to_string()]).unwrap();attempt(&db,"fal","fal","unknown",0);
        assert_eq!(replace_key(&db,"j","fal","replacement").unwrap(),1);
        assert_eq!(task_context(&db,"t").unwrap().1.as_deref(),Some("replacement"));
        assert_eq!(attempt_work(&db,"fal").unwrap().credential_ref.as_deref(),Some("sent-key"));
        freeze(&mut snapshot,&config("openrouter","other-model"),None,"second").unwrap();db.execute("UPDATE processing_tasks SET snapshot=?1",[snapshot.to_string()]).unwrap();
        let(c,key,receipt)=task_context(&db,"t").unwrap();assert_eq!(c.provider_mode,"openrouter");assert_eq!(key,None);assert_eq!(receipt.as_deref(),Some("second"));
        let historical=attempt_work(&db,"fal").unwrap();assert_eq!(historical.config.provider_mode,"fal");assert_eq!(historical.config.model,"attempt-model");
        // The real restore gate sees a Fal attempt even when its job says local.
        assert!(crate::index_backups::require_settled_current_cloud(&db).is_err());
        db.execute("UPDATE processing_attempts SET status='remoteFailed'",[]).unwrap();assert!(crate::index_backups::require_settled_current_cloud(&db).is_ok());
    }
    #[test]
    fn run_revision_and_work_visibility_guard_both_worker_and_metadata_commit(){
        let mut db=legacy();db.execute_batch(SCHEMA).unwrap();attempt(&db,"a","controlled-test","running",0);let w=attempt_work(&db,"a").unwrap();
        db.execute("UPDATE processing_tasks SET status='committing',result=?1",[result().to_string()]).unwrap();
        let c=CommitCheckpoint{job_id:"j".into(),task_id:"t".into(),generation:0,run_revision:0,sha256:"sha".into()};
        {let tx=db.transaction().unwrap();assert!(super::super::validate_commit(&tx,&c).is_ok());}
        db.execute("UPDATE processing_tasks SET run_revision=1",[]).unwrap();assert!(super::super::current_state(&db,&w).is_err());
        {let tx=db.transaction().unwrap();assert!(super::super::validate_commit(&tx,&c).is_err());}
        db.execute("UPDATE processing_tasks SET run_revision=0,work_visible=0",[]).unwrap();assert!(super::super::current_state(&db,&w).is_err());
        {let tx=db.transaction().unwrap();assert!(super::super::validate_commit(&tx,&c).is_err());}
        assert_eq!(db.query_row("SELECT cost_micros FROM processing_attempts WHERE id='a'",[],|r|r.get::<_,i64>(0)).unwrap(),19);
    }
    #[test]
    fn saved_response_recovery_is_idempotent_and_stale_run_only_keeps_history(){
        for stale in [false,true]{let mut db=legacy();db.execute_batch(SCHEMA).unwrap();attempt(&db,"a","controlled-test","responseSaved",0);
            if stale{db.execute("UPDATE processing_tasks SET run_revision=1",[]).unwrap();}
            recover_saved(&mut db).unwrap();recover_saved(&mut db).unwrap();
            let(status,normalized):(String,String)=db.query_row("SELECT status,normalized_result FROM processing_attempts WHERE id='a'",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!(status,if stale{"lateResponse"}else{"succeeded"});assert_eq!(serde_json::from_str::<Value>(&normalized).unwrap(),result());
            assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_passes",[],|r|r.get::<_,i64>(0)).unwrap(),if stale{0}else{1});
            assert_eq!(db.query_row("SELECT completed_passes FROM processing_tasks",[],|r|r.get::<_,i64>(0)).unwrap(),if stale{0}else{1});
        }
    }
    #[test]
    fn direct_container_and_command_receipt_identity_are_database_constraints(){
        let db=legacy();db.execute_batch(SCHEMA).unwrap();db.execute("UPDATE processing_jobs SET queue_mode='direct'",[]).unwrap();
        assert!(db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix,queue_mode) VALUES('other','other','{}','prepared',1,1,'direct')",[]).is_err());
        db.execute("INSERT INTO processing_commands(id,request_key,job_id,status,execution_context,created_at_unix,updated_at_unix) VALUES('c','intent','j','prepared','{}',1,1)",[]).unwrap();
        assert!(db.execute("INSERT INTO processing_commands(id,request_key,job_id,status,execution_context,created_at_unix,updated_at_unix) VALUES('other','intent','j','prepared','{}',1,1)",[]).is_err());
        db.execute("UPDATE processing_tasks SET work_visible=0",[]).unwrap();assert_eq!(db.query_row("SELECT queue_mode FROM processing_jobs",[],|r|r.get::<_,String>(0)).unwrap(),"direct");
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_commands",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    }
    #[test]
    fn direct_commands_share_container_manual_commands_do_not_and_cancel_owns_only_its_receipt(){
        let mut db=legacy();db.execute_batch(SCHEMA).unwrap();let c=config("local","first");
        let(job,a,_)=begin(&mut db,"direct-a","direct",&c,None).unwrap();let mut snapshot=json!({});freeze(&mut snapshot,&c,None,&a).unwrap();claim(&db,"a",&job,"i","first.png","sha",&snapshot,None,false).unwrap();finish(&db,&job,&a,"direct",1,0,0,0).unwrap();
        let mut other=c.clone();other.model="second".into();let(same,b,_)=begin(&mut db,"direct-b","direct",&other,None).unwrap();assert_eq!(same,job);let mut second=json!({});freeze(&mut second,&other,None,&b).unwrap();claim(&db,"b",&job,"i","second.png","sha2",&second,None,false).unwrap();finish(&db,&job,&b,"direct",1,0,0,0).unwrap();
        assert_eq!(task_context(&db,"a").unwrap().0.model,"first");assert_eq!(task_context(&db,"b").unwrap().0.model,"second");
        assert!(begin(&mut db,"direct-b","direct",&other,None).unwrap().2.is_some());
        let(ma,_,_)=begin(&mut db,"manual-a","manual",&c,None).unwrap();let(mb,_,_)=begin(&mut db,"manual-b","manual",&c,None).unwrap();assert_ne!(ma,mb);assert_ne!(ma,job);
        cancel_receipt(&db,"direct-b").unwrap();assert_eq!(db.query_row("SELECT work_visible FROM processing_tasks WHERE id='a'",[],|r|r.get::<_,bool>(0)).unwrap(),true);assert_eq!(db.query_row("SELECT work_visible FROM processing_tasks WHERE id='b'",[],|r|r.get::<_,bool>(0)).unwrap(),false);
        assert_eq!(db.query_row("SELECT queue_mode FROM processing_jobs WHERE id=?1",[job],|r|r.get::<_,String>(0)).unwrap(),"direct");
        assert!(!reusable("running",false,false));assert!(!reusable("failed",true,false));assert!(!reusable("failed",false,true));assert!(reusable("failed",false,false));
    }
    #[test]
    fn explicit_terminal_retry_replaces_only_working_checkpoints_and_old_cancel_cannot_touch_new_run(){
        let mut db=legacy();db.execute_batch(SCHEMA).unwrap();let c=config("local","first");let(job,a,_)=begin(&mut db,"retry-a","direct",&c,None).unwrap();
        let mut snapshot=json!({});freeze(&mut snapshot,&c,None,&a).unwrap();claim(&db,"retry",&job,"i","retry.png","sha",&snapshot,None,false).unwrap();
        db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,normalized_result) VALUES('paid','retry',1,0,'succeeded',1,37,?1)",[result().to_string()]).unwrap();db.execute("INSERT INTO processing_passes(task_id,pass_number,result,attempt_id) VALUES('retry',1,?1,'paid')",[result().to_string()]).unwrap();db.execute("UPDATE processing_tasks SET status='failed',completed_passes=1,result=?1 WHERE id='retry'",[result().to_string()]).unwrap();
        let(_,b,_)=begin(&mut db,"retry-b","direct",&c,None).unwrap();freeze(&mut snapshot,&c,None,&b).unwrap();claim(&db,"retry",&job,"i","retry.png","sha",&snapshot,None,true).unwrap();
        cancel_receipt(&db,"retry-a").unwrap();let(revision,visible,passes):(i64,bool,u32)=db.query_row("SELECT run_revision,work_visible,completed_passes FROM processing_tasks WHERE id='retry'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!((revision,visible,passes),(1,true,0));
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_passes WHERE task_id='retry'",[],|r|r.get::<_,i64>(0)).unwrap(),0);assert_eq!(db.query_row("SELECT reserved_micros FROM processing_attempts WHERE id='paid'",[],|r|r.get::<_,i64>(0)).unwrap(),37);
    }
    #[test]
    fn repeated_search_materializations_share_job_but_keep_their_command_sources(){
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();let c=config("local","first");let(job,a,_)=begin(&mut db,"search-a","direct",&c,None).unwrap();let(same,b,_)=begin(&mut db,"search-b","direct",&c,None).unwrap();assert_eq!(job,same);
        db.execute("INSERT INTO processing_sources(job_id,command_id,ordinal,path) VALUES(?1,?2,0,'a.png')",params![job,a]).unwrap();db.execute("INSERT INTO processing_sources(job_id,command_id,ordinal,path) VALUES(?1,?2,0,'b.png')",params![job,b]).unwrap();
        assert_eq!(db.query_row("SELECT path FROM processing_sources WHERE job_id=?1 AND command_id=?2",params![job,b],|r|r.get::<_,String>(0)).unwrap(),"b.png");
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_sources WHERE job_id=?1",[job],|r|r.get::<_,i64>(0)).unwrap(),2);
    }
    #[test]
    fn summary_counts_visible_work_for_manual_cards_and_keeps_direct_identity(){
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();let c=config("local","first");
        db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png',1,1)",[]).unwrap();
        let(job,receipt,_)=begin(&mut db,"card-a","manual",&c,None).unwrap();let mut snapshot=json!({});freeze(&mut snapshot,&c,None,&receipt).unwrap();claim(&db,"a",&job,"i","a.png","sha",&snapshot,None,false).unwrap();claim(&db,"b",&job,"i","b.png","sha",&snapshot,None,false).unwrap();finish(&db,&job,&receipt,"manual",2,0,0,0).unwrap();
        assert_eq!(summaries(&db,0).unwrap()[0]["total"],2);db.execute("UPDATE processing_tasks SET work_visible=0 WHERE id='b'",[]).unwrap();assert_eq!(summaries(&db,0).unwrap()[0]["total"],1);
    }
    #[test]
    fn three_removal_scopes_keep_other_jobs_history_money_and_direct_identity(){
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();let c=config("local","owned");
        db.execute("INSERT INTO images(id,identity_key,file_name,description,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png','Keep metadata',1,1)",[]).unwrap();
        let mut jobs=Vec::new();for(key,mode,task_id) in [("direct","direct","d"),("manual-a","manual","a"),("manual-b","manual","b")]{let(job,receipt,_)=begin(&mut db,key,mode,&c,None).unwrap();let mut snapshot=json!({});freeze(&mut snapshot,&c,None,&receipt).unwrap();claim(&db,task_id,&job,"i","owned.png","sha",&snapshot,None,false).unwrap();finish(&db,&job,&receipt,mode,1,0,0,0).unwrap();jobs.push(job);}
        for(id,task,status) in [("reserved-a","a","reserved"),("sent-b","b","running")]{db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,execution_context) SELECT ?1,id,1,0,?3,1,37,json_extract(snapshot,'$._execution') FROM processing_tasks WHERE id=?2",params![id,task,status]).unwrap();}
        let before=attempt_work(&db,"sent-b").unwrap();let removed={let tx=db.transaction().unwrap();let value=remove(&tx,"photo",Some(&jobs[1]),Some("a")).unwrap();tx.commit().unwrap();value};assert_eq!(removed.0,1);assert_eq!(removed.1,vec!["reserved-a"]);
        assert_eq!(db.query_row("SELECT status FROM processing_attempts WHERE id='reserved-a'",[],|r|r.get::<_,String>(0)).unwrap(),"notSent");assert_eq!(db.query_row("SELECT reserved_micros FROM processing_attempts WHERE id='reserved-a'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert!(super::super::current_state(&db,&before).is_ok());assert_eq!(db.query_row("SELECT work_visible FROM processing_tasks WHERE id='b'",[],|r|r.get::<_,bool>(0)).unwrap(),true);
        {let tx=db.transaction().unwrap();remove(&tx,"job",Some(&jobs[2]),None).unwrap();tx.commit().unwrap();}assert!(super::super::current_state(&db,&before).is_err());assert_eq!(db.query_row("SELECT work_visible FROM processing_jobs WHERE id=?1",[&jobs[2]],|r|r.get::<_,bool>(0)).unwrap(),false);
        assert_eq!(db.query_row("SELECT reserved_micros FROM processing_attempts WHERE id='sent-b'",[],|r|r.get::<_,i64>(0)).unwrap(),37);
        {let tx=db.transaction().unwrap();remove(&tx,"allWorking",None,None).unwrap();tx.commit().unwrap();}
        assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_tasks WHERE work_visible=1",[],|r|r.get::<_,i64>(0)).unwrap(),0);assert_eq!(db.query_row("SELECT description FROM images WHERE id='i'",[],|r|r.get::<_,String>(0)).unwrap(),"Keep metadata");
        assert_eq!(begin(&mut db,"after-clear","direct",&c,None).unwrap().0,jobs[0]);assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts",[],|r|r.get::<_,i64>(0)).unwrap(),2);assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_commands",[],|r|r.get::<_,i64>(0)).unwrap(),4);
    }
    #[test]fn service_failure_stops_only_unsent_matching_group_without_erasing_attempts(){
        use crate::provider_transport::FailureGroup;
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png',1,1)",[]).unwrap();
        for(id,provider,model,key) in [("origin","fal","bad","key-a"),("same","fal","bad","key-a"),("other-model","fal","good","key-a"),("other-key","fal","bad","key-b"),("other-provider","openrouter","bad","key-a"),("active","fal","bad","key-a")]{let c=config(provider,model);let(job,receipt,_)=begin(&mut db,id,"direct",&c,Some(key)).unwrap();let mut snapshot=json!({});freeze(&mut snapshot,&c,Some(key),&receipt).unwrap();claim(&db,id,&job,"i",&format!("{id}.png"),"sha",&snapshot,None,false).unwrap();finish(&db,&job,&receipt,"direct",1,0,0,0).unwrap();}
        for(task,status) in [("origin","failed"),("active","unknown")]{db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,execution_context) SELECT ?1,id,1,0,?2,1,37,json_extract(snapshot,'$._execution') FROM processing_tasks WHERE id=?1",params![task,status]).unwrap();}
        let work=attempt_work(&db,"origin").unwrap();db.execute("UPDATE processing_tasks SET status='failed' WHERE id='origin'",[]).unwrap();
        assert_eq!(fail_unsent_group(&db,&work,FailureGroup::Model,"ZDR violation").unwrap(),1);
        let state=|id:&str|db.query_row("SELECT status FROM processing_tasks WHERE id=?1",[id],|r|r.get::<_,String>(0)).unwrap();assert_eq!(state("same"),"failed");assert_eq!(state("other-model"),"pending");assert_eq!(state("other-key"),"pending");assert_eq!(state("other-provider"),"pending");assert_eq!(state("active"),"pending");
        assert_eq!(fail_unsent_group(&db,&work,FailureGroup::Credential,"Balance exhausted").unwrap(),1);assert_eq!(state("other-model"),"failed");assert_eq!(state("other-key"),"pending");assert_eq!(db.query_row("SELECT COUNT(*) FROM processing_attempts",[],|r|r.get::<_,i64>(0)).unwrap(),2);assert_eq!(db.query_row("SELECT SUM(reserved_micros) FROM processing_attempts",[],|r|r.get::<_,i64>(0)).unwrap(),74);
    }

}
