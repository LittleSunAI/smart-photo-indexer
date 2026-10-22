//! A queue row selects its indexed image, never a similarly named file from the current folder.
use crate::*;

pub(super) fn read(db:&Connection,task_id:&str)->Result<IndexSearchResultItemDto,String>{
 let(image_id,task_path):(String,String)=db.query_row("SELECT image_id,source_path FROM processing_tasks WHERE id=?1",[task_id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?.ok_or("Фотография задачи больше не доступна.")?;
 let path:String=db.query_row("SELECT path FROM image_paths WHERE image_id=?1 AND is_current=1 AND missing_at_unix IS NULL ORDER BY (path=?2) DESC,last_seen_at_unix DESC,id DESC LIMIT 1",params![image_id,task_path],|r|r.get(0)).optional().map_err(|e|e.to_string())?.ok_or("Актуальный файл фотографии не найден. Очередь сохранена.")?;
 let mut item=db.query_row("SELECT extension,size_bytes,modified_at_unix,status,description,is_favorite,description_version,tags_version,created_at_unix,ai_processed_at_unix FROM images WHERE id=?1",[&image_id],|r|Ok(IndexSearchResultItemDto{ai_processed_at_unix:r.get(9)?,image_id:image_id.clone(),path:path.clone(),file_name:file_name(Path::new(&path)),extension:r.get(0)?,size_bytes:optional_i64_to_u64(r.get(1)?),modified_at_unix:optional_i64_to_u64(r.get(2)?),status:r.get(3)?,description:r.get(4)?,is_favorite:r.get(5)?,description_version:r.get(6)?,tags_version:r.get(7)?,first_seen_at_unix:optional_i64_to_u64(r.get(8)?),last_seen_at_unix:None,tags:vec![],collections:vec![],score:0,matched_fields:vec![],file_metadata:None,description_format:None})).map_err(|e|e.to_string())?;
 item.tags=read_tags_for_image_from_connection(db,&image_id)?;item.collections=read_collection_names_for_image_from_connection(db,&image_id)?;item.file_metadata=image_metadata::stored(db,&image_id)?;item.description_format=description_format::read(db,&image_id)?;
 Ok(item)
}

#[tauri::command]
pub async fn get_processing_task_image(app:tauri::AppHandle,task_id:String)->Result<IndexSearchResultItemDto,String>{
 tauri::async_runtime::spawn_blocking(move||{let db=open_index_connection(&app)?;let item=read(&db,&task_id)?;if !Path::new(&item.path).is_file(){return Err("Файл фотографии отсутствует. Очередь сохранена.".into());}Ok(item)}).await.map_err(|e|e.to_string())?
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn task_identity_survives_rename_and_never_selects_reassigned_old_path(){
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
  db.execute_batch("INSERT INTO images(id,identity_key,file_name,status,description,created_at_unix,updated_at_unix) VALUES('a','a','old.png','processed','current description',1,1),('b','b','other.png','notProcessed',NULL,1,1);INSERT INTO processing_jobs(id,request_key,status,config,created_at_unix,updated_at_unix) VALUES('j','j','prepared','{}',1,1);INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','a','old.png','sha','{}','pending');INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('a','new.png',1,1,2),('b','old.png',1,1,3);").unwrap();
  let item=read(&db,"t").unwrap();assert_eq!(item.image_id,"a");assert_eq!(item.path,"new.png");assert_eq!(item.description.as_deref(),Some("current description"));
  db.execute("UPDATE image_paths SET is_current=0 WHERE image_id='a'",[]).unwrap();assert!(read(&db,"t").is_err());assert!(read(&db,"absent").is_err());let count:i64=db.query_row("SELECT COUNT(*) FROM processing_tasks",[],|r|r.get(0)).unwrap();assert_eq!(count,1);
 }
}
