use crate::*;
use image::GenericImageView;
use super::{contracts::Draft,geometry,local,store};
use serde_json::{json,Value};

/// Leaving a project keeps its task/results, but withdraws automatic selection.
/// The caller serializes this write with session changes and new job admission.
pub(super) fn revoke_result_activation(app:&tauri::AppHandle,project:&str)->Result<(),String>{
 store::connection(app)?.execute("UPDATE editor_job_activation SET state='revoked' WHERE project_id=?1 AND state='eligible'",[project]).map_err(|e|e.to_string())?;
 Ok(())
}

fn select_in_transaction(tx:&Connection,project:&str,revision:u64,step:&str)->Result<(),String>{
 store::require_mutable(tx,project)?;
 let(current,raw):(u64,String)=tx.query_row("SELECT revision,draft FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
 if current!=revision{return Err("Черновик изменился до выбора кадра.".into());}
 super::lifecycle::require_step(tx,project,step)?;
 let asset:String=tx.query_row("SELECT asset_id FROM editor_steps WHERE project_id=?1 AND id=?2",params![project,step],|r|r.get(0)).map_err(|_|"Готовый кадр не найден в проекте")?;
 let old:Draft=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
 if old.form_values.get("__maskStrokes").and_then(Value::as_array).is_some_and(|s|!s.is_empty()){return Err("Сначала примените или отмените незавершённую маску.".into());}
 store::stash_step_draft(tx,project,&old)?;
 let draft=store::step_draft(tx,step)?.unwrap_or_else(||old.new_frame(&asset));
 if draft.asset_id!=asset{return Err("База черновика отличается от выбранного кадра.".into());}
 store::validate_draft_resources(tx,project,&draft)?;
 // An explicit selection must be visible in the strip. Restore visibility in
 // the same transaction, after revision/resource validation, so a rejected
 // selection cannot expose a hidden frame or leave an invisible active step.
 tx.execute("UPDATE editor_step_visibility SET hidden=0 WHERE step_id=?1 AND hidden<>0",[step]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_job_activation SET state='revoked' WHERE project_id=?1 AND state='eligible'",[project]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_projects SET active_step_id=?2,draft=?3,revision=revision+1,updated_at_unix=?4 WHERE id=?1",params![project,step,serde_json::to_string(&draft).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
 Ok(())
}
pub fn select(app:&tauri::AppHandle,project:&str,revision:u64,step:&str)->Result<Value,String>{
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 select_in_transaction(&tx,project,revision,step)?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
pub fn import_file(app:&tauri::AppHandle,project:&str,path:&Path)->Result<Value,String>{
 store::require_mutable(&store::connection(app)?,project)?;let canonical=validate_existing_image_file(&path_to_string(path))?.canonicalize().map_err(|e|e.to_string())?;
 let image=file_safety::decode_bounded(&canonical)?;let asset=local::save_image(app,&image,"image")?;let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
  tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'reference')",params![project,asset["id"].as_str()]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;let mut snapshot=store::project_snapshot(app,project)?;snapshot["importedAssetId"]=asset["id"].clone();Ok(snapshot)
}
pub fn select_asset(app:&tauri::AppHandle,project:&str,revision:u64,asset:&str)->Result<Value,String>{
 local::project_asset(app,project,asset)?;let allowed:bool=store::connection(app)?.query_row("SELECT EXISTS(SELECT 1 FROM editor_resource_owners WHERE project_id=?1 AND asset_id=?2 AND boundary='working') OR EXISTS(SELECT 1 FROM editor_project_assets WHERE project_id=?1 AND asset_id=?2 AND role='reference')",params![project,asset],|r|r.get(0)).map_err(|e|e.to_string())?;if !allowed{return Err("Этот ресурс не входит в рабочую версию проекта.".into());}let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 store::require_mutable(&tx,project)?;
 let current:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if current!=revision{return Err("Черновик изменился до выбора кадра.".into());}
 let existing:Option<String>=tx.query_row("SELECT id FROM editor_steps WHERE project_id=?1 AND asset_id=?2 AND id IN(SELECT step_id FROM editor_working_steps WHERE project_id=?1) AND id<>(SELECT root_step_id FROM editor_projects WHERE id=?1) ORDER BY rowid DESC LIMIT 1",params![project,asset],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 let step=existing.unwrap_or_else(||stable_text_id("editor-import-source",&format!("{project}|{asset}")));
 tx.execute("INSERT OR IGNORE INTO editor_steps(id,project_id,asset_id,operation,title,created_at_unix) VALUES(?1,?2,?3,'import','Добавленный кадр',?4)",params![step,project,asset,unix_now()]).map_err(|e|e.to_string())?;
 select_in_transaction(&tx,project,revision,&step)?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
pub fn clear(app:&tauri::AppHandle,project:&str,revision:u64)->Result<Value,String>{
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
 let (current,raw):(u64,String)=tx.query_row("SELECT revision,draft FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;if current!=revision{return Err("Черновик изменился до очистки истории.".into());}
 let unsettled:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?1 AND state NOT IN ('completed','failed','cancelled'))",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if unsettled{return Err("Сначала завершите задачи проекта.".into());}
 let (first,asset,annotations):(String,String,String)=tx.query_row("SELECT id,asset_id,annotations FROM editor_steps WHERE project_id=?1 AND id=(SELECT root_step_id FROM editor_projects WHERE id=?1)",[project],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
 let old:Draft=serde_json::from_str(&raw).map_err(|e|e.to_string())?;if old.form_values.get("__maskStrokes").and_then(Value::as_array).is_some_and(|s|!s.is_empty()){return Err("Сначала примените или отмените маску.".into());}
 store::stash_step_draft(&tx,project,&old)?;
 let draft=store::step_draft(&tx,&first)?.unwrap_or_else(||old.new_frame(&asset));store::validate_draft_resources(&tx,project,&draft)?;
 let _=annotations;

 tx.execute("UPDATE editor_projects SET active_step_id=?2,draft=?3,revision=revision+1,updated_at_unix=?4 WHERE id=?1",params![project,first,serde_json::to_string(&draft).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
 store::stash_step_draft(&tx,project,&draft)?;
 tx.execute("INSERT INTO editor_step_visibility(step_id,hidden) SELECT id,1 FROM editor_steps WHERE project_id=?1 AND id<>?2 ON CONFLICT(step_id) DO UPDATE SET hidden=1",params![project,first]).map_err(|e|e.to_string())?;tx.execute("DELETE FROM editor_step_visibility WHERE step_id=?1",[&first]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
pub fn restore_step(app:&tauri::AppHandle,project:&str,step:&str)->Result<Value,String>{let mut db=store::connection(app)?;super::lifecycle::require_step(&db,project,step)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;tx.execute("DELETE FROM editor_step_visibility WHERE step_id=?1 AND step_id IN(SELECT id FROM editor_steps WHERE project_id=?2)",params![step,project]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)}
pub fn delete_project(app:&tauri::AppHandle,project:&str)->Result<Value,String>{
 processing::cloud_fence(app,||delete_project_locked(app,project))
}
pub(super) fn delete_project_locked(app:&tauri::AppHandle,project:&str)->Result<Value,String>{
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 store::require_mutable(&tx,project)?;
 let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?1)",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if !exists{return Err("Проект не найден.".into());}
 let unsettled:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?1 AND state NOT IN ('completed','failed','cancelled'))",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if unsettled{return Err("Сначала завершите и сверьте задачи этого проекта. Удаление не отменяет оплату в сервисе.".into());}
 tx.execute("INSERT OR IGNORE INTO editor_project_tombstones(project_id,deleted_at_unix) VALUES(?1,?2)",params![project,unix_now()]).map_err(|e|e.to_string())?;tx.execute("DELETE FROM editor_resource_owners WHERE project_id=?1",[project]).map_err(|e|e.to_string())?;tx.execute("DELETE FROM editor_working_steps WHERE project_id=?1",[project]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(json!({"deleted":true,"originalsPreserved":true}))
}
pub fn gc(app:&tauri::AppHandle)->Result<Value,String>{processing::cloud_fence(app,||gc_locked(app,None))}
/// Caller owns the cloud mutation fence; scoped GC never collects unrelated orphans.
pub(super) fn gc_locked(app:&tauri::AppHandle,allowed:Option<&std::collections::HashSet<String>>)->Result<Value,String>{
 let db=store::connection(app)?;if app.state::<super::jobs::EditorJobs>().has_leases()?{if allowed.is_some(){return Ok(json!({"removedFiles":0,"reclaimedBytes":0,"deferred":true,"errors":[]}));}return Err("Очистка доступна после завершения текущих файловых операций редактора.".into());}
 let mut q=db.prepare("SELECT a.id,a.relative_path,a.preview_path,a.sha256 FROM editor_assets a WHERE NOT EXISTS(SELECT 1 FROM editor_resource_owners p WHERE p.asset_id=a.id AND p.project_id NOT IN (SELECT project_id FROM editor_project_tombstones)) AND NOT EXISTS(SELECT 1 FROM editor_jobs j,json_tree(j.snapshot) t WHERE t.value=a.id AND j.state NOT IN ('completed','failed','cancelled')) AND NOT EXISTS(SELECT 1 FROM editor_publications p WHERE p.asset_id=a.id AND p.state NOT IN ('completed','failed','cancelled'))").map_err(|e|e.to_string())?;
 let candidates=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;drop(q);let mut removed=0;let mut bytes=0u64;let mut errors=Vec::new();
 for(id,relative,preview,sha)in candidates{if allowed.is_some_and(|ids|!ids.contains(&id)){continue;}
  let result=(||->Result<(),String>{
   let path=store::owned_path(app,&relative)?;if path.exists(){if compute_file_sha256(&path)?!=sha{return Err("Owned-файл изменился; его содержимое сохранено для проверки.".into());}
    db.execute("INSERT INTO editor_gc_targets(kind,owner_id,relative_path,expected_sha256,state,updated_at_unix) VALUES('asset',?1,?2,?3,'planned',?4) ON CONFLICT(kind,owner_id,relative_path) DO UPDATE SET state='planned',updated_at_unix=excluded.updated_at_unix",params![id,relative,sha,unix_now()]).map_err(|e|e.to_string())?;
    let length=fs::metadata(&path).map_err(|e|e.to_string())?.len();fs::remove_file(path).map_err(|e|e.to_string())?;bytes+=length;removed+=1;
    db.execute("UPDATE editor_gc_targets SET state='collected',updated_at_unix=?2 WHERE kind='asset' AND owner_id=?1",params![id,unix_now()]).map_err(|e|e.to_string())?;
   }
   let other_paths={let mut q=db.prepare("SELECT relative_path FROM editor_assets WHERE preview_path=?1 AND id<>?2").map_err(|e|e.to_string())?;let rows=q.query_map(params![preview,id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;rows};let mut shared=false;for other in other_paths{if store::owned_path(app,&other)?.is_file(){shared=true;break;}}
   if !shared{let path=store::owned_path(app,&preview)?;if path.is_file(){let length=fs::metadata(&path).map_err(|e|e.to_string())?.len();fs::remove_file(path).map_err(|e|e.to_string())?;bytes+=length;removed+=1;}}Ok(())
  })();if let Err(message)=result{if allowed.is_none(){return Err(message);}errors.push(json!({"assetId":id,"message":message}));}
 }
 if allowed.is_none(){
 let publications={let mut q=db.prepare("SELECT id,target_path,metadata FROM editor_publications WHERE state IN ('completed','failed','cancelled')").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 for(id,target,raw)in publications{let meta:Value=serde_json::from_str(&raw).map_err(|_|"Повреждён журнал публикации")?;
  // Explicit replacement keeps its previous original and prepared result for
  // project recovery. Later replacements legitimately change the external SHA.
  // Scoped project cleanup owns these files; generic copy-staging GC does not.
  if meta["mode"]=="replace"{continue;}
  let Some(relative)=meta["stagingRelative"].as_str()else{continue;};let Some(hash)=meta["sha256"].as_str()else{continue;};
  if Path::new(&target).is_file()&&compute_file_sha256(Path::new(&target))?!=hash{return Err("Опубликованная копия изменена; staging сохранён для проверки.".into());}
  let owned=store::owned_path(app,relative)?;if !owned.exists(){continue;}
  if !relative.starts_with("assets/publication-")||compute_file_sha256(&owned)?!=hash{return Err("Промежуточный файл изменён; очистка остановлена.".into());}
  db.execute("INSERT INTO editor_gc_targets(kind,owner_id,relative_path,expected_sha256,state,updated_at_unix) VALUES('staging',?1,?2,?3,'planned',?4) ON CONFLICT(kind,owner_id,relative_path) DO UPDATE SET state='planned'",params![id,relative,hash,unix_now()]).map_err(|e|e.to_string())?;
  let length=fs::metadata(&owned).map_err(|e|e.to_string())?.len();fs::remove_file(&owned).map_err(|e|e.to_string())?;removed+=1;bytes+=length;
  db.execute("UPDATE editor_gc_targets SET state='collected',updated_at_unix=?2 WHERE kind='staging' AND owner_id=?1",params![id,unix_now()]).map_err(|e|e.to_string())?;
 }

 }
 Ok(json!({"removedFiles":removed,"reclaimedBytes":bytes,"deferred":false,"errors":errors}))
}
pub fn delete_step(app:&tauri::AppHandle,project:&str,step:&str)->Result<Value,String>{
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
 let _operation:String=tx.query_row("SELECT operation FROM editor_steps WHERE project_id=?1 AND id=?2",params![project,step],|r|r.get(0)).map_err(|e|e.to_string())?;
 let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE active_step_id=?1) OR EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?2 AND state NOT IN ('completed','failed','cancelled'))",params![step,project],|r|r.get(0)).map_err(|e|e.to_string())?;
 if step==super::migration::root(&tx,project)?||used{return Err("Исходник или активный кадр удалять нельзя. Выберите другой кадр и завершите задачи.".into());}
 tx.execute("INSERT INTO editor_step_visibility(step_id,hidden) VALUES(?1,1) ON CONFLICT(step_id) DO UPDATE SET hidden=1",[step]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
/// Remove frames from the visible filmstrip atomically. Paid receipts, branches
/// and immutable pixels stay recoverable through the history dialog.
pub fn delete_steps(app:&tauri::AppHandle,project:&str,revision:u64,ids:&[String])->Result<Value,String>{
 if ids.is_empty()||ids.len()>1000{return Err("Выберите от 1 до 1000 кадров.".into());}
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
 let(current,active,raw):(u64,String,String)=tx.query_row("SELECT revision,active_step_id,draft FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
 if current!=revision{return Err("Проект изменился после выбора кадров. Повторите удаление.".into());}
 for id in ids{let _operation:String=tx.query_row("SELECT operation FROM editor_steps WHERE project_id=?1 AND id=?2",params![project,id],|r|r.get(0)).map_err(|_|"Кадр отсутствует в этом проекте".to_string())?;if *id==super::migration::root(&tx,project)?{return Err("Исходник защищён от удаления.".into());}}
 if ids.contains(&active){
  let old:Draft=serde_json::from_str(&raw).map_err(|e|e.to_string())?;store::stash_step_draft(&tx,project,&old)?;
  let(source,asset):(String,String)=tx.query_row("SELECT id,asset_id FROM editor_steps WHERE project_id=?1 AND id=(SELECT root_step_id FROM editor_projects WHERE id=?1)",[project],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
  let draft=store::step_draft(&tx,&source)?.unwrap_or_else(||old.new_frame(&asset));store::validate_draft_resources(&tx,project,&draft)?;
  tx.execute("UPDATE editor_projects SET active_step_id=?2,draft=?3,revision=revision+1,updated_at_unix=?4 WHERE id=?1",params![project,source,serde_json::to_string(&draft).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
 }
 for id in ids{tx.execute("INSERT INTO editor_step_visibility(step_id,hidden) VALUES(?1,1) ON CONFLICT(step_id) DO UPDATE SET hidden=1",[id]).map_err(|e|e.to_string())?;}
 tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
pub fn rename(app:&tauri::AppHandle,project:&str,title:&str)->Result<Value,String>{let title=title.trim();if title.is_empty()||title.chars().count()>100{return Err("Название должно содержать от 1 до 100 символов.".into());}let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;tx.execute("UPDATE editor_projects SET title=?2,updated_at_unix=?3 WHERE id=?1",params![project,title,unix_now()]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)}
pub fn list(app:&tauri::AppHandle)->Result<Value,String>{let db=store::connection(app)?;let mut q=db.prepare("SELECT id,title,source_name,updated_at_unix FROM editor_projects WHERE id NOT IN (SELECT project_id FROM editor_project_tombstones) ORDER BY updated_at_unix DESC,rowid DESC").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"sourceName":r.get::<_,String>(2)?,"updatedAtUnix":r.get::<_,u64>(3)?}))).map_err(|e|e.to_string())?;Ok(json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?))}

/// Quick access uses the same read-only preview, limited to the latest twelve frames.
pub fn preview_latest(app:&tauri::AppHandle,project:&str)->Result<Value,String>{
 let db=store::connection(app)?;
 let total:i64=db.query_row("SELECT COUNT(*) FROM editor_steps s JOIN editor_working_steps w ON w.step_id=s.id AND w.project_id=s.project_id LEFT JOIN editor_step_visibility v ON v.step_id=s.id WHERE s.project_id=?1 AND COALESCE(v.hidden,0)=0",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 preview_project(app,project,total.saturating_sub(12).max(0) as usize)
}
/// Read a bounded strip without selecting a project, editing its draft or loading full pixels.
pub fn preview_project(app:&tauri::AppHandle,project:&str,offset:usize)->Result<Value,String>{
 if offset>100_000{return Err("Слишком большое смещение истории.".into());}
 let db=store::connection(app)?;
 let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?1 AND id NOT IN(SELECT project_id FROM editor_project_tombstones))",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 if !exists{return Err("Проект недоступен.".into());}
 let total:i64=db.query_row("SELECT COUNT(*) FROM editor_steps s JOIN editor_working_steps w ON w.step_id=s.id AND w.project_id=s.project_id LEFT JOIN editor_step_visibility v ON v.step_id=s.id WHERE s.project_id=?1 AND COALESCE(v.hidden,0)=0",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 let mut query=db.prepare("SELECT s.id,s.asset_id,s.title FROM editor_steps s JOIN editor_working_steps w ON w.step_id=s.id AND w.project_id=s.project_id LEFT JOIN editor_step_visibility v ON v.step_id=s.id WHERE s.project_id=?1 AND COALESCE(v.hidden,0)=0 ORDER BY s.created_at_unix,s.rowid LIMIT 30 OFFSET ?2").map_err(|e|e.to_string())?;
 let rows=query.query_map(params![project,offset as i64],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
 let mut frames=Vec::new();
 for(id,asset_id,title)in rows{
  let draft=store::step_draft(&db,&id)?;
  let mut asset=store::asset_value(&db,&asset_id)?;
  let preview=asset["previewPath"].as_str().ok_or("Нет пути миниатюры")?;
  asset["previewPath"]=json!(path_to_string(&store::owned_path(app,preview)?));
  frames.push(json!({"id":id,"title":title,"asset":asset,"orientation":{"rotation":draft.as_ref().map(|d|d.working_turn).unwrap_or(0),"mirrored":draft.as_ref().is_some_and(|d|d.mirrored)}}));
 }
 Ok(json!({"projectId":project,"offset":offset,"total":total,"frames":frames}))
}
pub fn start_again(app:&tauri::AppHandle,project:&str,revision:u64)->Result<String,String>{let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let next=start_again_in_transaction(&tx,project,revision)?;tx.commit().map_err(|e|e.to_string())?;Ok(next)}
pub(super) fn start_again_in_transaction(tx:&Connection,project:&str,revision:u64)->Result<String,String>{
 let next=stable_text_id("editor-project",&format!("new|{project}|{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
 store::require_mutable(tx,project)?;
 let(current,raw):(u64,String)=tx.query_row("SELECT revision,draft FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;if current!=revision{return Err("Сохраните актуальный проект перед новой работой".into());}
 let source:String=tx.query_row("SELECT asset_id FROM editor_steps WHERE project_id=?1 AND id=(SELECT root_step_id FROM editor_projects WHERE id=?1)",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 let previous:Draft=serde_json::from_str(&raw).map_err(|e|e.to_string())?;let mut draft=Draft::initial(&source);draft.layout=previous.layout;draft.layout.right_tab="edit".into();draft.profile=previous.profile;
 if let Some(id)=previous.model_id{if super::catalog::route(&id).ok().is_some_and(|r|r["operations"].as_array().is_some_and(|ops|ops.iter().any(|op|op=="universal-edit"))){draft.model_id=Some(id);}}
 for key in ["__budgetUsd","__concurrency","__brushDiameter"]{if let Some(v)=previous.form_values.get(key){draft.form_values.insert(key.into(),v.clone());}}
 let step=format!("{next}-source");let now=unix_now();tx.execute("INSERT INTO editor_projects(id,title,source_name,source_path,source_sha256,active_step_id,draft,created_at_unix,updated_at_unix) SELECT ?2,title,source_name,source_path,source_sha256,?3,?4,?5,?5 FROM editor_projects WHERE id=?1",params![project,next,step,serde_json::to_string(&draft).map_err(|e|e.to_string())?,now]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'source')",params![next,source]).map_err(|e|e.to_string())?;tx.execute("INSERT INTO editor_steps(id,project_id,asset_id,operation,title,created_at_unix) VALUES(?1,?2,?3,'source','Исходник',?4)",params![step,next,source,now]).map_err(|e|e.to_string())?;store::stash_step_draft(&tx,&next,&draft)?;super::migration::root(&tx,&next)?;Ok(next)
}
pub fn duplicate(app:&tauri::AppHandle,project:&str)->Result<String,String>{let history=super::portable_history::export(app,project)?;let snapshot=store::project_snapshot(app,project)?;let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;let next=duplicate_in_transaction(&tx,project,&snapshot,&history)?;tx.commit().map_err(|e|e.to_string())?;Ok(next)}
pub(super) fn duplicate_in_transaction(tx:&Connection,project:&str,snapshot:&Value,history:&[Value])->Result<String,String>{
 let next=stable_text_id("editor-project",&format!("{project}|{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
 store::require_mutable(tx,project)?;
 let map=|id:&str|format!("{next}-{id}");let remap=|d:&mut Draft|{for f in d.edit_history.undo.iter_mut().chain(&mut d.edit_history.redo){if let Some(id)=&mut f.step_id{*id=map(id);}}d.form_values.remove("__consentScope");if let Some(contexts)=d.form_values.get_mut("__contexts").and_then(Value::as_object_mut){for value in contexts.values_mut(){if let Some(form)=value.get_mut("formValues").and_then(Value::as_object_mut){form.remove("__consentScope");}}}};let mut copied:Draft=serde_json::from_value(snapshot["project"]["draft"].clone()).map_err(|e|e.to_string())?;remap(&mut copied);let active=map(snapshot["project"]["activeStepId"].as_str().ok_or("Неверный active step")?);
 tx.execute("INSERT INTO editor_projects(id,title,source_name,source_path,source_sha256,active_step_id,draft,created_at_unix,updated_at_unix) SELECT ?2,title||' · копия',source_name,source_path,source_sha256,?3,draft,?4,?4 FROM editor_projects WHERE id=?1",params![project,next,active,unix_now()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_projects SET draft=?2 WHERE id=?1",params![next,serde_json::to_string(&copied).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO editor_project_assets(project_id,asset_id,role) SELECT ?2,asset_id,role FROM editor_project_assets WHERE project_id=?1",params![project,next]).map_err(|e|e.to_string())?;
 for step in snapshot["steps"].as_array().ok_or("Неверная история")?{let id=step["id"].as_str().ok_or("Неверный step")?;tx.execute("INSERT INTO editor_steps(id,project_id,asset_id,operation,title,parameters,annotations,job_id,created_at_unix) SELECT ?2,?3,asset_id,operation,title,parameters,annotations,job_id,created_at_unix FROM editor_steps WHERE id=?1",params![id,map(id),next]).map_err(|e|e.to_string())?;if let Some(mut working)=store::step_draft(&tx,id)?{remap(&mut working);tx.execute("INSERT INTO editor_step_drafts(step_id,draft) VALUES(?1,?2)",params![map(id),serde_json::to_string(&working).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;}}
 for step in snapshot["steps"].as_array().unwrap(){for parent in step["parentIds"].as_array().unwrap(){tx.execute("INSERT INTO editor_edges(step_id,parent_id) VALUES(?1,?2)",params![map(step["id"].as_str().unwrap()),map(parent.as_str().unwrap())]).map_err(|e|e.to_string())?;}}
 for step in snapshot["steps"].as_array().unwrap(){if step["hidden"]==true{tx.execute("INSERT INTO editor_step_visibility(step_id,hidden) VALUES(?1,1)",[map(step["id"].as_str().unwrap())]).map_err(|e|e.to_string())?;}}
 let assets=snapshot["assets"].as_array().unwrap().iter().map(|a|{let id=a["id"].as_str().unwrap().to_string();(id.clone(),id)}).collect::<std::collections::HashMap<_,_>>();let steps=snapshot["steps"].as_array().unwrap().iter().map(|s|{let id=s["id"].as_str().unwrap();(id.to_string(),map(id))}).collect::<std::collections::HashMap<_,_>>();let step_jobs=snapshot["steps"].as_array().unwrap().iter().map(|s|(s["id"].as_str().unwrap().to_string(),s["jobId"].as_str().map(String::from))).collect::<std::collections::HashMap<_,_>>();let (history,jobs)=super::portable_history::remap(&history,project,&next,&assets,&steps,&step_jobs,&|d|{remap(d);d.validate()})?;
 for step in snapshot["steps"].as_array().unwrap(){tx.execute("UPDATE editor_steps SET job_id=?2 WHERE id=?1",params![map(step["id"].as_str().unwrap()),step["jobId"].as_str().and_then(|id|jobs.get(id))]).map_err(|e|e.to_string())?;}
 tx.execute("UPDATE editor_projects SET root_step_id=?2 WHERE id=?1",params![next,map(snapshot["project"]["rootStepId"].as_str().ok_or("Нет исходника проекта")?)]).map_err(|e|e.to_string())?;super::portable_history::insert(&tx,&next,&history)?;super::portable_history::validate(&tx)?;super::migration::root(&tx,&next)?;Ok(next)
}
pub fn export_image(app:&tauri::AppHandle,project:&str,asset_id:&str,turn:u8,mirrored:bool,target:&Path,format:&str,quality:u8)->Result<Value,String>{
 validate_export_extension(target,format)?;
 if turn>3||!(1..=100).contains(&quality){return Err("Неверная ориентация или качество экспорта.".into());}
 let asset=local::project_asset(app,project,asset_id)?;if asset["kind"]!="image"{return Err("Выберите готовое изображение, не маску.".into());}let source=store::owned_path(app,asset["relativePath"].as_str().ok_or("Неверный source")?)?;
 let image=geometry::orient_image(file_safety::decode_bounded(&source)?,turn,mirrored);let has_alpha=image.pixels().any(|(_,_,p)|p[3]!=255);
 let format=match format{"png"=>image::ImageFormat::Png,"jpeg"=>image::ImageFormat::Jpeg,"tiff"=>image::ImageFormat::Tiff,"bmp"=>image::ImageFormat::Bmp,_=>return Err("Неизвестный формат экспорта.".into())};
 if format==image::ImageFormat::Jpeg&&has_alpha{return Err("JPEG не сохраняет прозрачность. Выберите PNG или TIFF.".into());}
 file_safety::atomic_save(target,|out,_|{if format==image::ImageFormat::Jpeg{image::codecs::jpeg::JpegEncoder::new_with_quality(out,quality).encode_image(&image.to_rgb8()).map_err(|e|e.to_string())}else{image.write_to(out,format).map_err(|e|e.to_string())}})?;
 Ok(json!({"path":path_to_string(target),"sha256":compute_file_sha256(target)?,"width":image.width(),"height":image.height()}))
}
pub fn validate_export(app:&tauri::AppHandle,project:&str,asset_id:&str,turn:u8,format:&str,quality:u8)->Result<(),String>{
 if turn>3||!(1..=100).contains(&quality)||!["png","jpeg","tiff","bmp"].contains(&format){return Err("Неверная ориентация, формат или качество экспорта.".into());}
 let asset=local::project_asset(app,project,asset_id)?;if asset["kind"]!="image"{return Err("Для экспорта нужна фотография".into());}
 let source=store::owned_path(app,asset["relativePath"].as_str().ok_or("Нет пути кадра")?)?;
 if compute_file_sha256(&source)?!=asset["sha256"].as_str().unwrap_or(""){return Err("Файл кадра изменился. Экспорт не выполнен.".into());}
 if format=="jpeg"&&file_safety::decode_bounded(&source)?.pixels().any(|(_,_,p)|p[3]!=255){return Err("JPEG не сохраняет прозрачность. Выберите PNG или TIFF.".into());}
 Ok(())
}

pub fn validate_export_extension(target:&Path,format:&str)->Result<(),String>{
 let ext=target.extension().and_then(|v|v.to_str()).unwrap_or("").to_ascii_lowercase();
 let valid=match format{"png"=>ext=="png","jpeg"=>matches!(ext.as_str(),"jpg"|"jpeg"),"tiff"=>matches!(ext.as_str(),"tif"|"tiff"),"bmp"=>ext=="bmp",_=>false};
 if !valid{return Err("Расширение имени файла должно соответствовать выбранному формату экспорта.".into());}Ok(())
}
