//! Frozen, app-scoped project cleanup. No client-supplied deletion paths.
use crate::*;
use super::{library,lifecycle,store,EditorSessions};
use chrono::TimeZone;
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use std::collections::{BTreeSet,HashMap,HashSet};
use std::sync::{Mutex,OnceLock};

#[derive(Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Scope{kind:String,#[serde(default)]paths:Vec<String>,before_date:Option<String>,zone:Option<String>}
#[derive(Clone)]struct Plan{scope:Scope,digest:String,ids:Vec<String>,assets:HashSet<String>,created:u64,view:Value}
static PLANS:OnceLock<Mutex<HashMap<String,Plan>>>=OnceLock::new();
fn error(e:impl ToString)->String{e.to_string()}
pub fn summary(app:&tauri::AppHandle)->Result<Value,String>{let db=store::connection(app)?;let count:u64=db.query_row("SELECT COUNT(*) FROM editor_projects WHERE id NOT IN(SELECT project_id FROM editor_project_tombstones)",[],|r|r.get(0)).map_err(error)?;let mut q=db.prepare("SELECT DISTINCT relative_path,preview_path FROM editor_assets").map_err(error)?;let mut paths=HashSet::new();for row in q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(error)?{let(a,b)=row.map_err(error)?;paths.insert(a);paths.insert(b);}let mut bytes=0u64;for relative in paths{let path=store::owned_path(app,&relative)?;if path.is_file(){bytes=bytes.saturating_add(fs::metadata(path).map_err(error)?.len());}}Ok(json!({"count":count,"ownedBytes":bytes}))}
fn cutoff(scope:&Scope)->Result<Option<i64>,String>{
 if scope.kind!="before"{return Ok(None);}
 let date=chrono::NaiveDate::parse_from_str(scope.before_date.as_deref().ok_or("Выберите дату очистки.")?,"%Y-%m-%d").map_err(|_|"Неверная дата очистки.")?;
 let zone:chrono_tz::Tz=scope.zone.as_deref().ok_or("Не указан часовой пояс.")?.parse().map_err(|_|"Неизвестный часовой пояс.")?;
 let time=zone.from_local_datetime(&date.and_hms_opt(0,0,0).ok_or("Неверная дата")?).single().ok_or("Начало дня неоднозначно в выбранном часовом поясе. Выберите другую дату.")?;
 Ok(Some(time.timestamp()))
}
fn slice(db:&Connection,sql:&str)->Result<Value,String>{let raw:String=db.query_row(sql,[],|r|r.get(0)).map_err(error)?;serde_json::from_str(&raw).map_err(error)}
fn build(app:&tauri::AppHandle,scope:&Scope,open:&BTreeSet<String>)->Result<Plan,String>{
 if !["all","selected","before"].contains(&scope.kind.as_str()){return Err("Неизвестная область очистки.".into());}
 let limit=cutoff(scope)?;let mut sources=Vec::new();
 if scope.kind=="selected"{if scope.paths.is_empty()||scope.paths.len()>500{return Err("Выберите фотографии для очистки их проектов.".into());}for raw in &scope.paths{let path=validate_existing_image_file(raw)?.canonicalize().map_err(error)?;sources.push((super::migration::publication_target_key(&path_to_string(&path)),compute_file_sha256(&path)?));}}
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(error)?;
 let projects={let mut q=tx.prepare("SELECT id,title,source_path,source_sha256,revision,EXISTS(SELECT 1 FROM editor_project_tombstones t WHERE t.project_id=p.id) FROM editor_projects p ORDER BY id").map_err(error)?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,u64>(4)?,r.get::<_,bool>(5)?))).map_err(error)?;rows.collect::<Result<Vec<_>,_>>().map_err(error)?};
 let mut eligible=Vec::new();let mut protected=Vec::new();let mut selected=Vec::new();
 for(id,title,path,sha,revision,deleted)in &projects{
  let state=if *deleted{lifecycle::state(&tx,id).unwrap_or(Value::Null)}else{lifecycle::observe(&tx,id)?};
  let matches=match scope.kind.as_str(){"all"=>true,"before"=>state["modifiedAtUnix"].as_i64().is_some_and(|time|Some(time)<limit),_=>{
   let original=super::migration::publication_target_key(path);let mut yes=sources.iter().any(|(p,s)|*p==original&&s==sha);
   if !yes{let mut q=tx.prepare("SELECT target_path,metadata FROM editor_publications WHERE project_id=?1 AND state='completed'").map_err(error)?;let rows=q.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(error)?;for row in rows{let(p,m)=row.map_err(error)?;let m:Value=serde_json::from_str(&m).map_err(error)?;if m["mode"]=="replace"&&sources.iter().any(|(target,sha)|*target==super::migration::publication_target_key(&p)&&m["sha256"]==*sha){yes=true;}}}yes
  }};if !matches{continue;}
  let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?1 AND state NOT IN('completed','failed','cancelled')) OR EXISTS(SELECT 1 FROM editor_publications WHERE project_id=?1 AND state NOT IN('completed','failed','cancelled'))",[id],|r|r.get(0)).map_err(error)?;
  let row=json!({"id":id,"title":title,"revision":revision,"contentGeneration":state["contentGeneration"],"deleted":deleted});selected.push(row.clone());
  if open.contains(id)||pending{let mut row=row;row["reason"]=json!(if open.contains(id){"open"}else{"pending"});protected.push(row);}else{eligible.push(id.clone());}
 }
 let ids=eligible.iter().cloned().collect::<HashSet<_>>();let mut assets=HashSet::new();let mut paths=std::collections::BTreeMap::<String,u64>::new();
 let mut q=tx.prepare("SELECT id,relative_path,preview_path FROM editor_assets ORDER BY id").map_err(error)?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;drop(q);
 for(id,relative,preview)in &rows{
  let owners={let mut q=tx.prepare("SELECT DISTINCT project_id FROM editor_project_assets WHERE asset_id=?1").map_err(error)?;let rows=q.query_map([id],|r|r.get::<_,String>(0)).map_err(error)?;rows.collect::<Result<Vec<_>,_>>().map_err(error)?};
  if !owners.iter().any(|owner|ids.contains(owner)){continue;}assets.insert(id.clone());
  let strong={let mut q=tx.prepare("SELECT DISTINCT project_id FROM editor_resource_owners WHERE asset_id=?1 AND project_id NOT IN(SELECT project_id FROM editor_project_tombstones)").map_err(error)?;let rows=q.query_map([id],|r|r.get::<_,String>(0)).map_err(error)?;rows.collect::<Result<Vec<_>,_>>().map_err(error)?};
  let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs j,json_tree(j.snapshot) t WHERE t.value=?1 AND j.state NOT IN('completed','failed','cancelled')) OR EXISTS(SELECT 1 FROM editor_publications WHERE asset_id=?1 AND state NOT IN('completed','failed','cancelled'))",[id],|r|r.get(0)).map_err(error)?;
  if pending||strong.iter().any(|owner|!ids.contains(owner)){continue;}
  let path=store::owned_path(app,relative)?;if path.is_file(){paths.insert(relative.clone(),fs::metadata(path).map_err(error)?.len());}
  // A preview shared with an asset outside this scope is not reclaimable here.
  let outside=rows.iter().any(|(other,_,p)|p==preview&&other!=id&&!assets.contains(other)&&{
   let count:i64=tx.query_row("SELECT COUNT(*) FROM editor_project_assets WHERE asset_id=?1 AND project_id IN(SELECT id FROM editor_projects WHERE id NOT IN(SELECT project_id FROM editor_project_tombstones))",[other],|r|r.get(0)).unwrap_or(1);count>0
  });
  if !outside{let path=store::owned_path(app,preview)?;if path.is_file(){paths.insert(preview.clone(),fs::metadata(path).map_err(error)?.len());}}
 }
 let leases=app.state::<super::jobs::EditorJobs>().has_leases()?;
 let digest=json!({"scope":scope,"sources":sources,"open":open,"leases":leases,
  "projects":slice(&tx,"SELECT json_group_array(json_array(id,revision,draft,root_step_id,active_step_id)) FROM (SELECT * FROM editor_projects ORDER BY id)")?,
  "life":slice(&tx,"SELECT json_group_array(json_array(project_id,content_generation,saved_generation,retained_manifest)) FROM (SELECT * FROM editor_project_lifecycle ORDER BY project_id)")?,
  "owners":slice(&tx,"SELECT json_group_array(json_array(project_id,asset_id,boundary)) FROM (SELECT * FROM editor_resource_owners ORDER BY project_id,asset_id,boundary)")?,
  "jobs":slice(&tx,"SELECT json_group_array(json_array(id,state,updated_at_unix,snapshot)) FROM (SELECT * FROM editor_jobs ORDER BY id)")?,
  "publications":slice(&tx,"SELECT json_group_array(json_array(id,state,metadata)) FROM (SELECT * FROM editor_publications ORDER BY id)")?,
  "tombstones":slice(&tx,"SELECT json_group_array(project_id) FROM (SELECT * FROM editor_project_tombstones ORDER BY project_id)")?});
 let view=json!({"projects":selected,"eligibleCount":eligible.len(),"protected":protected,"estimatedBytes":paths.values().sum::<u64>(),"cutoffUnix":limit,"zone":scope.zone,"beforeDate":scope.before_date,"deferred":leases});
 tx.commit().map_err(error)?;Ok(Plan{scope:scope.clone(),digest:format!("{:x}",Sha256::digest(digest.to_string())),ids:eligible,assets,created:unix_now(),view})
}
fn with_open<T>(app:&tauri::AppHandle,f:impl FnOnce(&BTreeSet<String>)->Result<T,String>)->Result<T,String>{processing::cloud_fence(app,||{let state=app.state::<EditorSessions>();let sessions=state.sessions.lock().map_err(|_|"Session lock failed")?;let open=sessions.values().map(|s|s.project_id.clone()).collect();f(&open)})}
pub fn preview(app:&tauri::AppHandle,scope:Scope)->Result<Value,String>{with_open(app,|open|{let plan=build(app,&scope,open)?;let token=format!("cleanup-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_err(error)?.as_nanos());let mut plans=PLANS.get_or_init(Default::default).lock().map_err(|_|"Cleanup lock failed")?;plans.retain(|_,p|unix_now().saturating_sub(p.created)<900);if plans.len()>=32{plans.clear();}let mut view=plan.view.clone();view["token"]=json!(token);plans.insert(token,plan);Ok(view)})}
pub fn confirm(app:&tauri::AppHandle,token:&str)->Result<Value,String>{
 let plan=PLANS.get_or_init(Default::default).lock().map_err(|_|"Cleanup lock failed")?.remove(token).ok_or("Предпросмотр устарел. Проверьте проекты ещё раз.")?;
 if unix_now().saturating_sub(plan.created)>900{return Err("Предпросмотр устарел. Проверьте проекты ещё раз.".into());}
 with_open(app,|open|{let current=build(app,&plan.scope,open)?;if current.digest!=plan.digest||current.ids!=plan.ids{return Err("Проекты или их защита изменились. Проверьте список ещё раз перед удалением.".into());}
  let mut deleted_count=0;for id in &plan.ids{let db=store::connection(app)?;let deleted:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_tombstones WHERE project_id=?1)",[id],|r|r.get(0)).map_err(error)?;if !deleted{library::delete_project_locked(app,id)?;deleted_count+=1;}}
  let mut result=library::gc_locked(app,Some(&plan.assets))?;result["deletedProjects"]=json!(deleted_count);result["estimatedBytes"]=plan.view["estimatedBytes"].clone();result["protected"]=plan.view["protected"].clone();Ok(result)
 })
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn local_date_is_strict_midnight_in_iana_zone(){let scope=Scope{kind:"before".into(),paths:vec![],before_date:Some("2026-10-20".into()),zone:Some("Europe/Moscow".into())};assert_eq!(cutoff(&scope).unwrap(),Some(1792443600));let mut invalid=scope.clone();invalid.zone=Some("unknown".into());assert!(cutoff(&invalid).is_err());invalid.zone=Some("Europe/Moscow".into());invalid.before_date=Some("2026-02-30".into());assert!(cutoff(&invalid).is_err());}
}
