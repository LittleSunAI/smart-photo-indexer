//! Durable, version-checked batch edits. Originals and descriptions are never written here.
use crate::*;
use serde_json::{json,Value};
use std::collections::HashSet;
#[derive(Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Input{image_ids:Vec<String>,add:Vec<String>,remove:Vec<String>}
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct Layer{tag:String,source:String,created:u64}
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct Hidden{tag:String,created:u64}
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
struct Before{image_id:String,#[serde(default)]file_name:Option<String>,#[serde(default)]path:Option<String>,version:i64,manual:i64,layers:Vec<Layer>,hidden:Vec<Hidden>,visible:Vec<String>,after:Vec<String>}
#[derive(Serialize,Deserialize)]
struct Plan{add:Vec<String>,remove:Vec<String>,images:Vec<Before>}
fn connection(app:&tauri::AppHandle)->Result<Connection,String>{let mut db=open_index_connection(app)?;run_index_migrations(&mut db)?;Ok(db)}
#[tauri::command]
pub async fn get_batch_tag_summary(app:tauri::AppHandle,image_ids:Vec<String>)->Result<Value,String>{
 tauri::async_runtime::spawn_blocking(move||summary(&connection(&app)?,&image_ids)).await.map_err(|e|e.to_string())?
}
fn summary(db:&Connection,image_ids:&[String])->Result<Value,String>{
 let ids:std::collections::BTreeSet<_>=image_ids.iter().cloned().collect();
 if ids.is_empty()||ids.len()>256||ids.iter().any(|id|id.is_empty()||id.len()>256){return Err("Выберите от 1 до 256 проиндексированных фотографий.".into());}
 let tx=db.unchecked_transaction().map_err(|e|e.to_string())?;
 let mut tags=std::collections::BTreeSet::new();
 for id in &ids{
  let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM images WHERE id=?1)",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
  if !exists{return Err("Выборка устарела: фотография отсутствует в индексе.".into());}
  tags.extend(read_tags_for_image(&tx,id)?);
 }
 tx.commit().map_err(|e|e.to_string())?;Ok(json!({"imageIds":ids,"tags":tags}))
}
#[tauri::command]
pub async fn get_last_batch_tags_action(app:tauri::AppHandle)->Result<Option<i64>,String>{
 tauri::async_runtime::spawn_blocking(move||{
  let db=connection(&app)?;
  let latest:Option<(i64,String)>=db.query_row("SELECT id,action_type FROM action_logs WHERE action_type IN('batch_tags_committed','batch_tags_undone') ORDER BY id DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
  Ok(latest.and_then(|(id,kind)|if kind=="batch_tags_committed"{Some(id)}else{None}))
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn preview_batch_tags(app:tauri::AppHandle,input:Input)->Result<Value,String>{
 tauri::async_runtime::spawn_blocking(move||preview(&mut connection(&app)?,input)).await.map_err(|e|e.to_string())?
}
fn preview(db:&mut Connection,input:Input)->Result<Value,String>{
 if input.image_ids.is_empty() || input.image_ids.len()>256 || input.image_ids.iter().any(|s|s.is_empty() || s.len()>256){return Err("Выберите от 1 до 256 проиндексированных фотографий.".into());}
 if input.add.len()+input.remove.len()>30 || input.add.iter().chain(&input.remove).any(|s|s.len()>256 || s.chars().any(char::is_control)){return Err("Слишком много тегов или слишком длинный тег.".into());}
 let add=normalize_search_tags(&input.add);let remove=normalize_search_tags(&input.remove);
 if add.is_empty() && remove.is_empty(){return Err("Укажите теги для добавления или удаления.".into());}
 if add.iter().any(|tag|remove.contains(tag)){return Err("Один тег нельзя одновременно добавить и удалить.".into());}
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let mut images=Vec::new();let mut ids=HashSet::new();
 for id in input.image_ids{if !ids.insert(id.clone()){continue;}
  let (version,manual)=tx.query_row("SELECT tags_version,manual_tags_set FROM images WHERE id=?1",[&id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).map_err(|_|"Выборка устарела: фотография отсутствует в индексе.".to_string())?;
  let layers={let mut statement=tx.prepare("SELECT t.name,l.source,l.created_at_unix FROM image_tag_layers l JOIN tags t ON t.id=l.tag_id WHERE l.image_id=?1 LIMIT 1001").map_err(|e|e.to_string())?;
   let rows=statement.query_map([&id],|r|Ok(Layer{tag:r.get(0)?,source:r.get(1)?,created:r.get(2)?})).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
  let hidden={let mut statement=tx.prepare("SELECT t.name,s.created_at_unix FROM image_tag_suppressions s JOIN tags t ON t.id=s.tag_id WHERE s.image_id=?1 LIMIT 1001").map_err(|e|e.to_string())?;
   let rows=statement.query_map([&id],|r|Ok(Hidden{tag:r.get(0)?,created:r.get(1)?})).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
  if layers.len()>1000 || hidden.len()>1000{return Err("Слишком большая история тегов для одной пакетной операции.".into());}
  let visible=read_tags_for_image(&tx,&id)?;let mut after=visible.iter().filter(|tag|!remove.contains(tag)).cloned().collect::<Vec<_>>();for tag in &add{if !after.contains(tag){after.push(tag.clone());}}after.sort();
  let file_name=tx.query_row("SELECT file_name FROM images WHERE id=?1",[&id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
  let path=tx.query_row("SELECT path FROM image_paths WHERE image_id=?1 ORDER BY is_current DESC,last_seen_at_unix DESC,id LIMIT 1",[&id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
  images.push(Before{image_id:id,file_name,path,version,manual,layers,hidden,visible,after});
 }
 let plan=Plan{add,remove,images};let payload=serde_json::to_string(&plan).map_err(|e|e.to_string())?;if payload.len()>2_000_000{return Err("Разделите выборку на меньшие пакеты.".into());}
 tx.execute("INSERT INTO action_logs(action_type,old_value,message,created_at_unix) VALUES('batch_tags_preview',?1,'Предпросмотр пакетных тегов',?2)",params![payload,unix_now()]).map_err(|e|e.to_string())?;
 let plan_id=tx.last_insert_rowid();tx.commit().map_err(|e|e.to_string())?;
 Ok(json!({"planId":plan_id,"images":plan.images.iter().map(|image|json!({"imageId":image.image_id,"before":image.visible,"after":image.after})).collect::<Vec<_>>()}))
}
#[tauri::command]
pub async fn commit_batch_tags(app:tauri::AppHandle,plan_id:i64,undo:bool)->Result<Vec<IndexMetadataUpdateResultDto>,String>{
 tauri::async_runtime::spawn_blocking(move||commit(&mut connection(&app)?,plan_id,undo)).await.map_err(|e|e.to_string())?
}
fn commit(db:&mut Connection,plan_id:i64,undo:bool)->Result<Vec<IndexMetadataUpdateResultDto>,String>{
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 let (kind,payload,created):(String,String,u64)=tx.query_row("SELECT action_type,old_value,created_at_unix FROM action_logs WHERE id=?1",[plan_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_|"План пакетных тегов недоступен.".to_string())?;
 if kind!=if undo{"batch_tags_committed"}else{"batch_tags_preview"}{return Err("Этот план уже выполнен или отменён.".into());}
 if !undo && unix_now().saturating_sub(created)>86400{return Err("Предпросмотр устарел. Создайте новый.".into());}
 if payload.len()>2_000_000{return Err("План пакетной операции превышает ограничение размера.".into());}
 let plan:Plan=serde_json::from_str(&payload).map_err(|_|"Повреждённый план пакетной операции.".to_string())?;
 if plan.images.is_empty() || plan.images.len()>256 || plan.add.len()+plan.remove.len()>30{return Err("Недопустимые размеры сохранённого плана.".into());}
 // Check the whole frozen selection before any write. A conflict rolls back every photograph.
 for image in &plan.images{
  let version:i64=tx.query_row("SELECT tags_version FROM images WHERE id=?1",[&image.image_id],|r|r.get(0)).map_err(|_|"Фотография удалена из индекса.".to_string())?;
  if version!=image.version+i64::from(undo){return Err("Теги изменены после предпросмотра. Операция отменена для всех фотографий; обновите выборку.".into());}
 }
 let now=unix_now();let mut results=Vec::new();
 for image in &plan.images{
  let id=&image.image_id;
  if undo{
   tx.execute("DELETE FROM image_tag_layers WHERE image_id=?1",[id]).map_err(|e|e.to_string())?;tx.execute("DELETE FROM image_tag_suppressions WHERE image_id=?1",[id]).map_err(|e|e.to_string())?;
   for layer in &image.layers{tag_layers::add_layer(&tx,id,&layer.tag,&layer.source,layer.created)?;}
   for hidden in &image.hidden{
    tx.execute("INSERT INTO tags(name,kind,created_at_unix) VALUES(?1,'user',?2) ON CONFLICT(name) DO NOTHING",params![hidden.tag,hidden.created]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO image_tag_suppressions(image_id,tag_id,created_at_unix) SELECT ?1,id,?3 FROM tags WHERE name=?2",params![id,hidden.tag,hidden.created]).map_err(|e|e.to_string())?;
   }
  }else{
   for tag in &plan.remove{
    tx.execute("DELETE FROM image_tag_layers WHERE image_id=?1 AND source='user' AND tag_id IN(SELECT id FROM tags WHERE name=?2)",params![id,tag]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO tags(name,kind,created_at_unix) VALUES(?1,'user',?2) ON CONFLICT(name) DO NOTHING",params![tag,now]).map_err(|e|e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO image_tag_suppressions(image_id,tag_id,created_at_unix) SELECT ?1,id,?3 FROM tags WHERE name=?2",params![id,tag,now]).map_err(|e|e.to_string())?;
   }
   for tag in &plan.add{tag_layers::add_layer(&tx,id,tag,"user",now)?;}
  }
  tag_layers::rebuild(&tx,id)?;
  tx.execute("UPDATE images SET tags_version=tags_version+1,manual_tags_set=?2,updated_at_unix=?3,status=CASE WHEN status IN('error','missing','relocated','queued') THEN status WHEN COALESCE(description,'')<>'' OR EXISTS(SELECT 1 FROM image_tags WHERE image_id=?1) THEN 'processed' ELSE 'notProcessed' END WHERE id=?1",params![id,if undo{image.manual}else{1},now]).map_err(|e|e.to_string())?;
  tx.execute("INSERT INTO image_metadata_history(image_id,source,payload,created_at_unix) VALUES(?1,'manual',?2,?3)",params![id,json!({"fields":["tags"],"batchPlanId":plan_id,"undo":undo,"tags":read_tags_for_image(&tx,id)?}).to_string(),now]).map_err(|e|e.to_string())?;
  let (file_name,status,description,dv,tv,processed_at):(String,String,Option<String>,i64,i64,Option<u64>)=tx.query_row("SELECT file_name,status,description,description_version,tags_version,ai_processed_at_unix FROM images WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|e|e.to_string())?;
  let path:String=tx.query_row("SELECT path FROM image_paths WHERE image_id=?1 ORDER BY is_current DESC,id LIMIT 1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?.unwrap_or_default();
  results.push(IndexMetadataUpdateResultDto{ai_processed_at_unix:processed_at,description_format:description_format::read(&tx,id)?,image_id:id.clone(),path,file_name,status,description,description_version:dv,tags_version:tv,tags:read_tags_for_image(&tx,id)?,updated_at_unix:now,action_log_id:plan_id,image_count:0,path_count:0,tag_count:0});
 }

 // Keep the mutable undo marker for compatibility; immutable operation events belong to the journal.
 if undo{
  tx.execute("INSERT INTO action_logs(action_type,old_value,new_value,message,created_at_unix) SELECT 'batch_tags_applied_event',old_value,json_set(new_value,'$.planId',id),'Пакетные теги сохранены',created_at_unix FROM action_logs marker WHERE id=?1 AND NOT EXISTS(SELECT 1 FROM action_logs event WHERE event.action_type='batch_tags_applied_event' AND json_extract(event.new_value,'$.planId')=marker.id)",[plan_id]).map_err(|e|e.to_string())?;
 }
 tx.execute("INSERT INTO action_logs(action_type,old_value,new_value,message,created_at_unix) SELECT ?2,old_value,?3,?4,?5 FROM action_logs WHERE id=?1",params![plan_id,if undo{"batch_tags_undo_event"}else{"batch_tags_applied_event"},json!({"planId":plan_id,"count":results.len(),"undo":undo}).to_string(),if undo{"Изменение тегов отменено"}else{"Пакетные теги сохранены"},now]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE action_logs SET action_type=?2,new_value=?3,message=?4 WHERE id=?1",params![plan_id,if undo{"batch_tags_undone"}else{"batch_tags_committed"},json!({"count":results.len(),"undo":undo}).to_string(),if undo{"Пакетные теги отменены"}else{"Пакетные теги сохранены"}]).map_err(|e|e.to_string())?;
 let image_count=count_rows(&tx,"images")?;let path_count=count_rows(&tx,"image_paths")?;let tag_count=count_rows(&tx,"tags")?;
 for result in &mut results{result.image_count=image_count;result.path_count=path_count;result.tag_count=tag_count;}
 tx.commit().map_err(|e|e.to_string())?;Ok(results)
}

#[cfg(test)]mod journal_tests{
 use super::*;
 #[test]fn legacy_plan_without_names_still_applies_and_undo_preserves_first_event(){
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
  for id in ["first","second"]{db.execute("INSERT INTO images(id,identity_key,file_name,status,created_at_unix,updated_at_unix) VALUES(?1,?1,?2,'notProcessed',1,1)",params![id,format!("{id}.png")]).unwrap();}
  let plan=preview(&mut db,Input{image_ids:vec!["first".into(),"second".into()],add:vec!["restorable-tag".into()],remove:vec![]}).unwrap();let id=plan["planId"].as_i64().unwrap();
  let payload:String=db.query_row("SELECT old_value FROM action_logs WHERE id=?1",[id],|r|r.get(0)).unwrap();let mut legacy:Value=serde_json::from_str(&payload).unwrap();for image in legacy["images"].as_array_mut().unwrap(){image.as_object_mut().unwrap().remove("fileName");image.as_object_mut().unwrap().remove("path");}db.execute("UPDATE action_logs SET old_value=?2 WHERE id=?1",params![id,legacy.to_string()]).unwrap();
  let result=commit(&mut db,id,false).unwrap();assert!(result.iter().all(|r|r.tags==vec!["restorable-tag"]));
  let before=action_journal::read(&db,ActionJournalInputDto{image_id:Some("first".into()),limit:Some(80),offset:None,include_search_history:Some(false)}).unwrap();assert_eq!(before.entries.len(),1);let original=serde_json::to_string(&before.entries[0]).unwrap();
  let undone=commit(&mut db,id,true).unwrap();assert!(undone.iter().all(|r|r.tags.is_empty()));let after=action_journal::read(&db,ActionJournalInputDto{image_id:Some("first".into()),limit:Some(80),offset:None,include_search_history:Some(false)}).unwrap();assert_eq!(after.entries.len(),2);assert_eq!(serde_json::to_string(after.entries.iter().find(|e|e.action_type=="batch_tags_applied_event").unwrap()).unwrap(),original);assert_eq!(after.entries.iter().filter(|e|e.action_type=="batch_tags_undo_event").count(),1);assert!(commit(&mut db,id,true).is_err());
 }
}

#[cfg(test)]mod selection_summary_tests{
 use super::*;
 fn seed()->Connection{let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();for id in ["a","b","other"]{db.execute("INSERT INTO images(id,identity_key,file_name,status,created_at_unix,updated_at_unix) VALUES(?1,?1,?1,'notProcessed',1,1)",[id]).unwrap();}db}
 fn plan(db:&mut Connection,ids:&[&str],add:&[&str],remove:&[&str])->i64{preview(db,Input{image_ids:ids.iter().map(|s|s.to_string()).collect(),add:add.iter().map(|s|s.to_string()).collect(),remove:remove.iter().map(|s|s.to_string()).collect()}).unwrap()["planId"].as_i64().unwrap()}
 #[test]fn summary_is_union_of_unique_ids_and_never_silently_truncates(){
  let mut db=seed();let p=plan(&mut db,&["a"],&["shared","only-a"],&[]);commit(&mut db,p,false).unwrap();let p=plan(&mut db,&["b"],&["shared","only-b"],&[]);commit(&mut db,p,false).unwrap();
  let s=summary(&db,&["a".into(),"b".into(),"a".into()]).unwrap();assert_eq!(s["imageIds"],json!(["a","b"]));assert_eq!(s["tags"],json!(["only-a","only-b","shared"]));
  assert!(summary(&db,&[]).is_err());assert!(summary(&db,&["a".into(),"missing".into()]).is_err());assert!(summary(&db,&(0..257).map(|i|i.to_string()).collect::<Vec<_>>()).is_err());
 }
 #[test]fn one_save_add_remove_remains_atomic_and_undo_version_checked(){
  let mut db=seed();let p=plan(&mut db,&["a","b","other"],&["old","shared"],&[]);commit(&mut db,p,false).unwrap();
  let stale=plan(&mut db,&["a","b"],&["new"],&["old"]);let foreign=plan(&mut db,&["b"],&["foreign"],&[]);commit(&mut db,foreign,false).unwrap();assert!(commit(&mut db,stale,false).is_err());assert_eq!(read_tags_for_image_from_connection(&db,"a").unwrap(),vec!["old","shared"]);
  let fresh=plan(&mut db,&["a","b"],&["new"],&["old"]);let results=commit(&mut db,fresh,false).unwrap();assert_eq!(results.len(),2);assert_eq!(read_tags_for_image_from_connection(&db,"other").unwrap(),vec!["old","shared"]);assert!(commit(&mut db,fresh,false).is_err());
  commit(&mut db,fresh,true).unwrap();assert_eq!(read_tags_for_image_from_connection(&db,"a").unwrap(),vec!["old","shared"]);assert!(read_tags_for_image_from_connection(&db,"b").unwrap().contains(&"foreign".to_string()));
  let p=plan(&mut db,&["a"],&["newer"],&[]);commit(&mut db,p,false).unwrap();let foreign=plan(&mut db,&["a"],&["late"],&[]);commit(&mut db,foreign,false).unwrap();assert!(commit(&mut db,p,true).is_err());
 }
}
