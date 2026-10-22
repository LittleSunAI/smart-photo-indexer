use rusqlite::{Connection,params,OptionalExtension};
use serde_json::{json,Value};

pub const V2:&str=r#"
ALTER TABLE editor_projects ADD COLUMN root_step_id TEXT REFERENCES editor_steps(id);
CREATE TABLE editor_job_activation(job_id TEXT PRIMARY KEY REFERENCES editor_jobs(id),project_id TEXT NOT NULL REFERENCES editor_projects(id),source_step_id TEXT NOT NULL REFERENCES editor_steps(id),state TEXT NOT NULL CHECK(state IN ('ineligible','eligible','consumed','revoked')),activated_step_id TEXT REFERENCES editor_steps(id),updated_at_unix INTEGER NOT NULL);
CREATE UNIQUE INDEX editor_activation_owner ON editor_job_activation(project_id) WHERE state='eligible';
CREATE TABLE editor_job_outputs(job_id TEXT NOT NULL REFERENCES editor_jobs(id),output_index INTEGER NOT NULL,asset_id TEXT REFERENCES editor_assets(id),step_id TEXT REFERENCES editor_steps(id),accepted INTEGER NOT NULL,provider_descriptor TEXT NOT NULL DEFAULT '{}',PRIMARY KEY(job_id,output_index));
CREATE TABLE editor_gc_targets(kind TEXT NOT NULL,owner_id TEXT NOT NULL,relative_path TEXT NOT NULL,expected_sha256 TEXT NOT NULL,state TEXT NOT NULL,updated_at_unix INTEGER NOT NULL,PRIMARY KEY(kind,owner_id,relative_path));
ALTER TABLE editor_publications RENAME TO editor_publications_v1;
CREATE TABLE editor_publications(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES editor_projects(id),asset_id TEXT NOT NULL REFERENCES editor_assets(id),state TEXT NOT NULL,target_path TEXT NOT NULL,metadata TEXT NOT NULL,created_at_unix INTEGER NOT NULL,error TEXT);
INSERT INTO editor_publications SELECT * FROM editor_publications_v1;
DROP TABLE editor_publications_v1;
CREATE UNIQUE INDEX editor_publication_target ON editor_publications(json_extract(metadata,'$.targetKey')) WHERE state NOT IN ('failed','cancelled');
"#;

pub fn publication_target_key(path:&str)->String{
 #[cfg(target_os="windows")]{let path=path.replace('/',"\\");let path=if let Some(tail)=path.strip_prefix(r"\\?\UNC\"){format!(r"\\{tail}")}else{path.strip_prefix(r"\\?\").unwrap_or(&path).to_string()};path.to_lowercase()}
 #[cfg(not(target_os="windows"))]{path.to_string()}
}
pub fn publication_keys_needed(db:&Connection)->Result<bool,String>{
 let sql:Option<String>=db.query_row("SELECT sql FROM sqlite_master WHERE name='editor_publication_target'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 Ok(sql.is_some_and(|sql|!sql.contains("json_extract(metadata,'$.targetKey')")))
}
// Completed copies still own their destination. A completed explicit replacement
// retains its receipt, but must not prevent a later, newly confirmed replacement.
const REPLACEMENT_RESERVATION_PREDICATE:&str="AND (state!='completed' OR COALESCE(json_extract(metadata,'$.mode'),'copy')!='replace')";
pub fn replacement_reservations_needed(db:&Connection)->Result<bool,String>{
 let sql:String=db.query_row("SELECT sql FROM sqlite_master WHERE name='editor_publication_target'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 Ok(!sql.contains(REPLACEMENT_RESERVATION_PREDICATE))
}
pub fn upgrade_replacement_reservations(db:&mut Connection)->Result<(),String>{
 if !replacement_reservations_needed(db)?{return Ok(());}
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute_batch("DROP INDEX editor_publication_target;").map_err(|e|e.to_string())?;
 tx.execute_batch(&format!("CREATE UNIQUE INDEX editor_publication_target ON editor_publications(json_extract(metadata,'$.targetKey')) WHERE state NOT IN ('failed','cancelled') {REPLACEMENT_RESERVATION_PREDICATE};")).map_err(|_|"Публикации резервируют одно имя. Индекс не изменён; данные сохранены.".to_string())?;
 tx.commit().map_err(|e|e.to_string())
}
fn fill_publication_keys(db:&Connection)->Result<(),String>{
 let rows={let mut q=db.prepare("SELECT id,target_path,metadata FROM editor_publications").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 for(id,path,raw)in rows{let mut metadata:Value=serde_json::from_str(&raw).map_err(|_|"Повреждены параметры прежней публикации")?;let map=metadata.as_object_mut().ok_or("Параметры публикации не являются объектом")?;map.insert("targetKey".into(),json!(publication_target_key(&path)));db.execute("UPDATE editor_publications SET metadata=?2 WHERE id=?1",params![id,metadata.to_string()]).map_err(|e|e.to_string())?;}
 Ok(())
}

pub fn root(db:&Connection,project:&str)->Result<String,String>{
 let saved:Option<String>=db.query_row("SELECT root_step_id FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 if let Some(id)=saved{return Ok(id);}
 // Original creators and package/duplicate remappers preserve the '-source' suffix.
 // Imported working-source IDs do not have that suffix. Never infer from list position.
 let candidates={let mut q=db.prepare("SELECT s.id FROM editor_steps s JOIN editor_assets a ON a.id=s.asset_id JOIN editor_projects p ON p.id=s.project_id WHERE s.project_id=?1 AND s.operation='source' AND a.sha256=p.source_sha256 AND NOT EXISTS(SELECT 1 FROM editor_edges e WHERE e.step_id=s.id)").map_err(|e|e.to_string())?;let rows=q.query_map([project],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 let known=candidates.iter().filter(|id|id.ends_with("-source")).cloned().collect::<Vec<_>>();
 let id=if known.len()==1{known[0].clone()}else if known.is_empty()&&candidates.len()==1{candidates[0].clone()}else{return Err("Не удалось однозначно определить исходный кадр старого проекта. Данные сохранены без изменения.".into());};
 db.execute("UPDATE editor_projects SET root_step_id=?2 WHERE id=?1 AND root_step_id IS NULL",params![project,id]).map_err(|e|e.to_string())?;Ok(id)
}
pub fn apply(db:&mut Connection)->Result<(),String>{
 let version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 if version==3{return Ok(());}if version==2{if !publication_keys_needed(db)?{return Ok(());}let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;tx.execute_batch("DROP INDEX editor_publication_target;").map_err(|e|e.to_string())?;fill_publication_keys(&tx)?;tx.execute_batch("CREATE UNIQUE INDEX editor_publication_target ON editor_publications(json_extract(metadata,'$.targetKey')) WHERE state NOT IN ('failed','cancelled');").map_err(|_|"Прежние публикации резервируют одно имя с разным регистром. Миграция отменена; данные сохранены.".to_string())?;tx.commit().map_err(|e|e.to_string())?;return Ok(());}if version!=1{return Err("Неподдерживаемая версия миграции редактора".into());}
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute_batch(V2).map_err(|e|e.to_string())?;
 fill_publication_keys(&tx)?;
 let projects={let mut q=tx.prepare("SELECT id FROM editor_projects").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 for project in projects{root(&tx,&project)?;}
 let jobs={let mut q=tx.prepare("SELECT id,project_id,snapshot FROM editor_jobs").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
 for(id,project,raw)in jobs{
  let snapshot:Value=serde_json::from_str(&raw).map_err(|_|"Повреждён снимок старой задачи")?;
  if let Some(parent)=snapshot["parentId"].as_str(){let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_steps WHERE id=?1 AND project_id=?2)",params![parent,project],|r|r.get(0)).map_err(|e|e.to_string())?;if !valid{return Err("Старая задача ссылается на чужой кадр".into());}
   tx.execute("INSERT INTO editor_job_activation(job_id,project_id,source_step_id,state,updated_at_unix) VALUES(?1,?2,?3,'ineligible',0)",params![id,project,parent]).map_err(|e|e.to_string())?;
  }
  let rows={let mut q=tx.prepare("SELECT id,asset_id FROM editor_steps WHERE job_id=?1 AND project_id=?2 ORDER BY rowid").map_err(|e|e.to_string())?;let rows=q.query_map(params![id,project],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
  for(step,asset)in rows{let index=step.strip_prefix(&format!("{id}-output-")).and_then(|s|s.parse::<u64>().ok()).ok_or("Неоднозначный порядковый номер старого результата")?;tx.execute("INSERT INTO editor_job_outputs(job_id,output_index,asset_id,step_id,accepted,provider_descriptor) VALUES(?1,?2,?3,?4,1,?5)",params![id,index,asset,step,json!({"legacy":true}).to_string()]).map_err(|e|e.to_string())?;}
 }
 let errors:i64=tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;if errors!=0{return Err("Нарушены связи при миграции редактора. Миграция отменена.".into());}
 validate_relationships(&tx)?;
 tx.execute_batch("PRAGMA user_version=2;").map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(())
}
pub fn validate_relationships(db:&Connection)->Result<(),String>{
 if !publication_keys_needed(db)?{let mut q=db.prepare("SELECT target_path,metadata FROM editor_publications").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;for row in rows{let(path,raw)=row.map_err(|e|e.to_string())?;let metadata:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;if metadata["targetKey"]!=json!(publication_target_key(&path)){return Err("Ключ имени публикации не соответствует сохранённому пути.".into());}}}
 let invalid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_projects p LEFT JOIN editor_steps s ON s.id=p.root_step_id WHERE s.id IS NULL OR s.project_id<>p.id) OR EXISTS(SELECT 1 FROM editor_job_activation a JOIN editor_jobs j ON j.id=a.job_id LEFT JOIN editor_steps s ON s.id=a.source_step_id LEFT JOIN editor_steps output ON output.id=a.activated_step_id WHERE j.project_id<>a.project_id OR s.id IS NULL OR s.project_id<>a.project_id OR (a.state='consumed' AND (output.id IS NULL OR output.project_id<>a.project_id))) OR EXISTS(SELECT 1 FROM editor_job_outputs o JOIN editor_jobs j ON j.id=o.job_id LEFT JOIN editor_steps s ON s.id=o.step_id WHERE o.accepted=1 AND (s.id IS NULL OR s.project_id<>j.project_id OR s.asset_id<>o.asset_id OR s.job_id<>o.job_id))",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 if invalid{return Err("Корневой кадр, активация или результат относятся к другому проекту. Данные не подключены.".into());}Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn completed_replacements_release_only_their_reservation_without_changing_history(){
  let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA user_version=3; CREATE TABLE editor_publications(id TEXT PRIMARY KEY,target_path TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL); CREATE UNIQUE INDEX editor_publication_target ON editor_publications(json_extract(metadata,'$.targetKey')) WHERE state NOT IN ('failed','cancelled');").unwrap();
  let metadata=json!({"targetKey":"original.png","mode":"replace","sha256":"retained-receipt"}).to_string();
  db.execute("INSERT INTO editor_publications VALUES('first','original.png','completed',?1)",[&metadata]).unwrap();
  assert!(replacement_reservations_needed(&db).unwrap());upgrade_replacement_reservations(&mut db).unwrap();upgrade_replacement_reservations(&mut db).unwrap();assert!(!replacement_reservations_needed(&db).unwrap());
  assert_eq!(db.query_row("SELECT metadata FROM editor_publications WHERE id='first'",[],|r|r.get::<_,String>(0)).unwrap(),metadata);
  db.execute("INSERT INTO editor_publications VALUES('second','original.png','planned',?1)",[&metadata]).unwrap();
  for state in ["planned","replacing","needsRecovery","indexPending"]{
   db.execute("UPDATE editor_publications SET state=?1 WHERE id='second'",[state]).unwrap();
   assert!(db.execute("INSERT INTO editor_publications VALUES('conflict','original.png','planned',?1)",[&metadata]).is_err());
  }
  db.execute("UPDATE editor_publications SET state='completed' WHERE id='second'",[]).unwrap();
  db.execute("INSERT INTO editor_publications VALUES('third','original.png','planned',?1)",[&metadata]).unwrap();
  for mode in [None,Some("copy")] {
   let key=if mode.is_some(){"explicit-copy"}else{"legacy-copy"};let mut meta=json!({"targetKey":key});if let Some(mode)=mode{meta["mode"]=json!(mode);}
   db.execute("INSERT INTO editor_publications VALUES(?1,?1,'completed',?2)",params![key,meta.to_string()]).unwrap();
   assert!(db.execute("INSERT INTO editor_publications VALUES(?1,?1,'planned',?2)",params![format!("conflict-{key}"),meta.to_string()]).is_err());
  }
  assert_eq!(db.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),3);
 }
 #[test]fn conflicting_reservation_upgrade_rolls_back_index_and_receipts(){
  let mut db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE editor_publications(id TEXT PRIMARY KEY,target_path TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL); CREATE UNIQUE INDEX editor_publication_target ON editor_publications(target_path); INSERT INTO editor_publications VALUES('a','first','planned','{\"targetKey\":\"same\"}'),('b','second','planned','{\"targetKey\":\"same\"}');").unwrap();
  assert!(upgrade_replacement_reservations(&mut db).is_err());assert!(replacement_reservations_needed(&db).unwrap());
  assert_eq!(db.query_row("SELECT COUNT(*) FROM editor_publications",[],|r|r.get::<_,u64>(0)).unwrap(),2);
  let sql:String=db.query_row("SELECT sql FROM sqlite_master WHERE name='editor_publication_target'",[],|r|r.get(0)).unwrap();assert!(sql.contains("(target_path)"));
 }
 #[test]fn publication_reservations_upgrade_casefold_and_release_only_terminal_paths(){
  let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA user_version=2; CREATE TABLE editor_publications(id TEXT PRIMARY KEY,target_path TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL); CREATE UNIQUE INDEX editor_publication_target ON editor_publications(target_path) WHERE state NOT IN ('failed','cancelled');").unwrap();
  db.execute("INSERT INTO editor_publications VALUES('old',?1,'planned','{\"parameters\":{}}')",[r"D:\Папка\Фото.PNG"]).unwrap();apply(&mut db).unwrap();apply(&mut db).unwrap();
  let key=publication_target_key(r"D:\Папка\Фото.PNG");assert_eq!(db.query_row("SELECT json_extract(metadata,'$.targetKey') FROM editor_publications WHERE id='old'",[],|r|r.get::<_,String>(0)).unwrap(),key);
  let other=publication_target_key(r"d:/ПАПКА/фото.png");if cfg!(target_os="windows"){assert_eq!(key,other);}else{assert_ne!(key,other);}
  assert!(db.execute("INSERT INTO editor_publications VALUES('conflict',?1,'planned',?2)",params![r"d:\папка\фото.png",json!({"targetKey":key}).to_string()]).is_err());
  db.execute("UPDATE editor_publications SET state='failed' WHERE id='old'",[]).unwrap();db.execute("INSERT INTO editor_publications VALUES('retry',?1,'planned',?2)",params![r"d:\папка\фото.png",json!({"targetKey":key}).to_string()]).unwrap();
  db.execute("UPDATE editor_publications SET state='cancelled' WHERE id='retry'",[]).unwrap();db.execute("INSERT INTO editor_publications VALUES('written',?1,'completed',?2)",params![r"d:\папка\фото.png",json!({"targetKey":key}).to_string()]).unwrap();assert!(db.execute("INSERT INTO editor_publications VALUES('overwrite',?1,'planned',?2)",params![r"d:\папка\фото.png",json!({"targetKey":key}).to_string()]).is_err());
 }
 #[cfg(target_os="windows")]
 #[test]fn ambiguous_old_publication_reservations_rollback_without_losing_history(){
  let mut db=Connection::open_in_memory().unwrap();db.execute_batch("PRAGMA user_version=2; CREATE TABLE editor_publications(id TEXT PRIMARY KEY,target_path TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL); CREATE UNIQUE INDEX editor_publication_target ON editor_publications(target_path) WHERE state NOT IN ('failed','cancelled'); INSERT INTO editor_publications VALUES('a','D:\\ФОТО.png','planned','{}'),('b','d:\\фото.png','planned','{}');").unwrap();
  assert!(apply(&mut db).is_err());assert!(publication_keys_needed(&db).unwrap());assert_eq!(db.query_row("SELECT COUNT(*) FROM editor_publications WHERE metadata='{}'",[],|r|r.get::<_,u64>(0)).unwrap(),2);
 }
 #[test]fn schema_upgrade_is_atomic_repeatable_and_has_independent_frame_roots(){
  let mut db=Connection::open_in_memory().unwrap();db.execute_batch(super::super::store::SCHEMA).unwrap();db.execute_batch(super::super::store::EXTRA_SCHEMA).unwrap();apply(&mut db).unwrap();apply(&mut db).unwrap();
  assert_eq!(db.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),2);
  assert!(db.prepare("SELECT root_step_id FROM editor_projects").is_ok());
 }
}
