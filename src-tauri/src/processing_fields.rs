//! Field readiness is about this file's bytes and the requested result, not
//! queue status. The same predicate is used at preparation and admission.
use super::*;
pub const SCHEMA:&str=include_str!("processing-fields.sql");
fn mode(fields:&[String])->&'static str{if fields.len()==2{"both"}else if fields.first().is_some_and(|f|f=="tags"){"tags"}else{"description"}}
pub(super) fn missing(db:&Connection,image:&str,sha:&str,requested:&[String])->Result<Vec<String>,String>{
    let (description,description_sha,tags_sha,empty_done):(Option<String>,Option<String>,Option<String>,bool)=db.query_row("SELECT description,description_content_sha,tags_content_sha,tags_analysis_completed FROM images WHERE id=?1",[image],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
    let tags=read_tags_for_image_from_connection(db,image)?;
    Ok(requested.iter().filter(|f|match f.as_str(){"description"=>description_sha.as_deref()!=Some(sha)||description.as_deref().is_none_or(|v|v.trim().is_empty()),"tags"=>tags_sha.as_deref()!=Some(sha)||tags.is_empty()&&!empty_done,_=>true}).cloned().collect())
}
pub(super) fn prepare(db:&Connection,image:&str,sha:&str,snapshot:&mut Value,config:&JobConfig)->Result<Option<JobConfig>,String>{
    let requested=metadata_fields(&config.output_mode)?.into_iter().map(str::to_string).collect::<Vec<_>>();
    let pending=if config.queue_contract_version>=2&&config.only_unprocessed{missing(db,image,sha,&requested)?}else{requested.clone()};
    if pending.is_empty(){return Ok(None);}
    let(description,tags)=read_metadata_versions(db,image)?;
    snapshot["_fieldPlan"]=json!({"requested":requested,"missing":pending,"descriptionVersion":description,"tagsVersion":tags});
    let mut effective=config.clone();effective.output_mode=mode(&pending).into();if effective.output_mode!=config.output_mode&&!test_transport(config){effective.estimated_cost_per_image_usd=0.0;}validate_config(&mut effective)?;Ok(Some(effective))
}
pub(super) fn refresh(db:&Connection,task:&str)->Result<bool,String>{
    let(config,key,receipt)=queue::task_context(db,task)?;
    if config.queue_contract_version<2||!config.only_unprocessed{return Ok(true);}
    let(image,sha,raw,passes):(String,String,String,u32)=db.query_row("SELECT image_id,sha256,snapshot,completed_passes FROM processing_tasks WHERE id=?1",[task],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
    // An accepted pass/unknown request is immutable. Do not reset it on resume.
    let attempts:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id WHERE a.task_id=?1 AND a.run_revision=t.run_revision AND a.status<>'notSent')",[task],|r|r.get(0)).map_err(|e|e.to_string())?;
    if passes>0||attempts{return Ok(true);}
    let mut snapshot:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
    let requested=snapshot["_fieldPlan"]["requested"].as_array().map(|v|v.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>()).unwrap_or(metadata_fields(&config.output_mode)?.into_iter().map(str::to_string).collect());
    let pending=missing(db,&image,&sha,&requested)?;
    if pending.is_empty(){db.execute("UPDATE processing_tasks SET status='succeeded',error=NULL WHERE id=?1",[task]).map_err(|e|e.to_string())?;return Ok(false);}
    // The command already owns the original limits. Recover them if a field
    // that was ready at preparation was removed before the first send.
    let original:Option<String>=if let Some(receipt)=receipt.as_deref(){db.query_row("SELECT json_extract(execution_context,'$.config') FROM processing_commands WHERE id=?1",[receipt],|r|r.get(0)).optional().map_err(|e|e.to_string())?}else{None};
    let mut next=original.map(|v|serde_json::from_str::<JobConfig>(&v)).transpose().map_err(|e|e.to_string())?.unwrap_or_else(||config.clone());next.output_mode=mode(&pending).into();if next.output_mode!=config.output_mode&&!test_transport(&config){next.estimated_cost_per_image_usd=0.0;}validate_config(&mut next)?;
    snapshot["_fieldPlan"]["missing"]=json!(pending);let(d,t)=read_metadata_versions(db,&image)?;snapshot["_fieldPlan"]["descriptionVersion"]=json!(d);snapshot["_fieldPlan"]["tagsVersion"]=json!(t);
    snapshot["_execution"]=queue::context(&next,key.as_deref(),receipt.as_deref());
    if snapshot.to_string()!=raw{db.execute("UPDATE processing_tasks SET snapshot=?2 WHERE id=?1",params![task,snapshot.to_string()]).map_err(|e|e.to_string())?;}
    Ok(true)
}
pub(crate) fn commit_fields(db:&Connection,c:&CommitCheckpoint,image:&str,fields:&[String])->Result<Vec<String>,String>{
    let(config,_,_)=queue::task_context(db,&c.task_id)?;
    if config.queue_contract_version<2{return Ok(fields.to_vec());}
    let raw:String=db.query_row("SELECT snapshot FROM processing_tasks WHERE id=?1",[&c.task_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let snapshot:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;let(d,t)=read_metadata_versions(db,image)?;
    let pending=if config.only_unprocessed{missing(db,image,&c.sha256,fields)?}else{fields.to_vec()};
    Ok(pending.into_iter().filter(|f|snapshot["_fieldPlan"][if f=="description"{"descriptionVersion"}else{"tagsVersion"}].as_i64()==Some(if f=="description"{d}else{t})).collect())
}
pub(super) fn budget_micros(c:&JobConfig)->Option<i64>{if c.queue_contract_version>=2&&c.max_estimated_cost_usd==0.0{None}else{Some((c.max_estimated_cost_usd*1e6).floor() as i64)}}

#[cfg(test)]mod tests{
 use super::*;
 fn db()->Connection{let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png','bytes',1,1)",[]).unwrap();db}
 fn config(mode:&str)->JobConfig{serde_json::from_value(json!({"providerMode":"local","model":"qwen3.5:2b","outputMode":mode,"queueContractVersion":2})).unwrap()}
 fn requested()->Vec<String>{vec!["description".into(),"tags".into()]}
 fn task(db:&Connection,config:&JobConfig)->CommitCheckpoint{
  let mut snapshot=json!({});let effective=prepare(db,"i","bytes",&mut snapshot,config).unwrap().unwrap();queue::freeze(&mut snapshot,&effective,None,"receipt").unwrap();
  db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','request',?1,'running',1,1)",[serde_json::to_string(config).unwrap()]).unwrap();
  db.execute("INSERT INTO processing_commands(id,request_key,job_id,status,execution_context,created_at_unix,updated_at_unix) VALUES('receipt','request','j','prepared',?1,1,1)",[queue::context(config,None,Some("receipt")).to_string()]).unwrap();
  db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned.png','bytes',?1,'pending')",[snapshot.to_string()]).unwrap();
  CommitCheckpoint{job_id:"j".into(),task_id:"t".into(),generation:0,run_revision:0,sha256:"bytes".into()}
 }
 #[test]fn requested_fields_and_content_bound_empty_success_are_distinct_from_manual_clear(){
  let db=db();assert_eq!(missing(&db,"i","bytes",&requested()).unwrap(),requested());
  db.execute("UPDATE images SET tags_content_sha='bytes',tags_analysis_completed=1",[]).unwrap();
  assert_eq!(missing(&db,"i","bytes",&requested()).unwrap(),vec!["description"]);assert!(missing(&db,"i","bytes",&["tags".into()]).unwrap().is_empty());
  assert_eq!(missing(&db,"i","changed-bytes",&requested()).unwrap(),requested());
  db.execute("UPDATE images SET tags_analysis_completed=0,tags_version=1",[]).unwrap();assert_eq!(missing(&db,"i","bytes",&requested()).unwrap(),requested());
  db.execute("UPDATE images SET description='Manual text',description_content_sha='bytes'",[]).unwrap();assert_eq!(missing(&db,"i","bytes",&requested()).unwrap(),vec!["tags"]);
 }
 #[test]fn tags_to_both_freezes_only_missing_description_and_full_ready_creates_no_task_plan(){
  let db=db();db.execute("UPDATE images SET tags_content_sha='bytes',tags_analysis_completed=1",[]).unwrap();
  let mut snapshot=json!({});let effective=prepare(&db,"i","bytes",&mut snapshot,&config("both")).unwrap().unwrap();assert_eq!(effective.output_mode,"description");assert_eq!(snapshot["_fieldPlan"]["requested"],json!(["description","tags"]));assert_eq!(snapshot["_fieldPlan"]["missing"],json!(["description"]));
  assert!(prepare(&db,"i","bytes",&mut json!({}),&config("tags")).unwrap().is_none());
  let mut repeat=config("tags");repeat.only_unprocessed=false;assert_eq!(prepare(&db,"i","bytes",&mut json!({}),&repeat).unwrap().unwrap().output_mode,"tags");
 }
 #[test]fn admission_rechecks_ready_fields_and_commit_preserves_parallel_manual_versions(){
  let db=db();let c=task(&db,&config("both"));
  db.execute("UPDATE images SET description='Manual concurrent',description_content_sha='bytes',description_version=1",[]).unwrap();
  assert_eq!(commit_fields(&db,&c,"i",&requested()).unwrap(),vec!["tags"]);
  assert!(refresh(&db,"t").unwrap());assert_eq!(queue::task_context(&db,"t").unwrap().0.output_mode,"tags");
  db.execute("UPDATE images SET tags_content_sha='bytes',tags_analysis_completed=1",[]).unwrap();assert!(!refresh(&db,"t").unwrap());assert_eq!(db.query_row("SELECT status FROM processing_tasks",[],|r|r.get::<_,String>(0)).unwrap(),"succeeded");
 }
 #[test]fn explicit_reprocess_keeps_version_guard_and_legacy_zero_is_not_unlimited(){
  let db=db();let mut repeat=config("both");repeat.only_unprocessed=false;let c=task(&db,&repeat);
  db.execute("UPDATE images SET tags_version=1",[]).unwrap();assert_eq!(commit_fields(&db,&c,"i",&requested()).unwrap(),vec!["description"]);
  let mut c=config("both");assert_eq!(budget_micros(&c),None);c.queue_contract_version=0;assert_eq!(budget_micros(&c),Some(0));c.max_estimated_cost_usd=0.25;assert_eq!(budget_micros(&c),Some(250000));c.queue_contract_version=2;assert_eq!(budget_micros(&c),Some(250000));
 }
 #[test]fn field_removed_while_waiting_reuses_original_command_limits_before_first_send(){
  let db=db();db.execute("UPDATE images SET description='Already ready',description_content_sha='bytes'",[]).unwrap();task(&db,&config("both"));assert_eq!(queue::task_context(&db,"t").unwrap().0.output_mode,"tags");
  db.execute("UPDATE images SET description=NULL,description_version=1",[]).unwrap();assert!(refresh(&db,"t").unwrap());let next=queue::task_context(&db,"t").unwrap().0;assert_eq!(next.output_mode,"both");assert_eq!(next.max_tokens,3200);
 }
}
