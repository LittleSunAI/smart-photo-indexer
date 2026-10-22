//! File export and archive registration have separate, durable checkpoints.
use crate::*;
use super::{library,store};
use serde_json::{json,Value};
pub(crate) fn require_live_intent(db:&Connection,id:&str)->Result<(),String>{
 let state:String=db.query_row("SELECT state FROM editor_publications WHERE id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
 if ["cancelled","failed"].contains(&state.as_str()){return Err("Путь изменён. Подготовьте сохранение заново.".into());}Ok(())
}
pub fn list(app:&tauri::AppHandle)->Result<Value,String>{let db=store::connection(app)?;let mut q=db.prepare("SELECT id,project_id,asset_id,state,target_path,error,COALESCE(json_extract(metadata,'$.mode'),'copy') FROM editor_publications ORDER BY rowid DESC").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"projectId":r.get::<_,String>(1)?,"assetId":r.get::<_,String>(2)?,"state":r.get::<_,String>(3)?,"path":r.get::<_,String>(4)?,"error":r.get::<_,Option<String>>(5)?,"mode":r.get::<_,String>(6)?}))).map_err(|e|e.to_string())?;Ok(json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?))}
pub fn create(app:&tauri::AppHandle,project:&str,id:&str,asset_id:&str,turn:u8,mirrored:bool,target:&Path,format:&str,quality:u8)->Result<Value,String>{
 library::validate_export_extension(target,format)?;
 if id.len()<8||id.len()>100||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'){return Err("Неверный ID публикации.".into());}
 library::validate_export(app,project,asset_id,turn,format,quality)?;
 let parent=target.parent().ok_or("Нет папки назначения")?.canonicalize().map_err(|e|e.to_string())?;let target=parent.join(target.file_name().ok_or("Нет имени файла")?);
 if target.starts_with(store::root(app)?){return Err("Копия для архива должна сохраняться вне внутреннего хранилища редактора.".into());}
 let parameters=json!({"assetId":asset_id,"workingTurn":turn,"mirrored":mirrored,"format":format,"quality":quality});let db=store::connection(app)?;
 let original:Option<(String,String,String)>=db.query_row("SELECT project_id,target_path,metadata FROM editor_publications WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
 let resolved=if original.as_ref().is_some_and(|(owner,path,raw)|owner==project&&(super::migration::publication_target_key(path)!=super::migration::publication_target_key(&path_to_string(&target))||serde_json::from_str::<Value>(raw).ok().is_some_and(|meta|meta["parameters"]!=parameters))){stable_text_id("editor-publication",&format!("{id}|{project}|{}|{}",path_to_string(&target),parameters))}else{id.to_string()};let id=resolved.as_str();
 let old:Option<(String,String,String)>=db.query_row("SELECT project_id,target_path,metadata FROM editor_publications WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
  processing::cloud_fence(app,||{if index_backups::restore_blocked(app)?{return Err("Подготовлено восстановление архива. Сначала отмените его или перезапустите приложение.".into());}if let Some((owner,path,raw))=old{let metadata:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;if owner!=project||super::migration::publication_target_key(&path)!=super::migration::publication_target_key(&path_to_string(&target))||metadata["parameters"]!=parameters{return Err("ID публикации уже связан с другой копией.".into());}}else{if target.exists(){return Err("Файл уже существует. Выберите другое имя; исходник не заменяется.".into());}db.execute("INSERT INTO editor_publications(id,project_id,asset_id,state,target_path,metadata,created_at_unix) VALUES(?1,?2,?3,'planned',?4,?5,?6)",params![id,project,asset_id,path_to_string(&target),json!({"parameters":parameters,"targetKey":super::migration::publication_target_key(&path_to_string(&target))}).to_string(),unix_now()]).map_err(|e|e.to_string())?;}
 Ok(())})?;reconcile(app,id)
}
pub fn reconcile(app:&tauri::AppHandle,id:&str)->Result<Value,String>{
 if store::connection(app)?.query_row("SELECT COALESCE(json_extract(metadata,'$.mode'),'copy')='replace' FROM editor_publications WHERE id=?1",[id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return super::replacement::resume(app,id);}
 // Caller holds the shared image worker. No WebView, HTTP or new inference here.
 let db=store::connection(app)?;let (project,asset,state,target,raw):(String,String,String,String,String)=db.query_row("SELECT project_id,asset_id,state,target_path,metadata FROM editor_publications WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|e|e.to_string())?;
 if state=="completed"{return Ok(json!({"id":id,"state":state,"path":target}));}
 if ["failed","cancelled"].contains(&state.as_str()){return Err("Это намерение завершено без файла. Выберите параметры и создайте новое сохранение.".into());}
 let result=(||->Result<Value,String>{let mut metadata:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;let target=PathBuf::from(&target);let p=&metadata["parameters"];let format=p["format"].as_str().ok_or("Нет формата публикации")?;let staging_relative=format!("assets/publication-{id}.{format}");let staging=store::owned_path(app,&staging_relative)?;
  if metadata["sha256"].is_null(){
   // After a crash, regenerate to a different owned staging name before trusting
   // an existing destination. Never replace the destination during recovery.
   let source=if staging.exists(){store::owned_path(app,&format!("assets/publication-{id}-verify-{}.{format}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()))?}else{staging.clone()};
   let saved=library::export_image(app,&project,&asset,p["workingTurn"].as_u64().ok_or("Нет поворота")? as u8,p["mirrored"].as_bool().unwrap_or(false),&source,format,p["quality"].as_u64().ok_or("Нет качества")? as u8)?;
   if source!=staging{if compute_file_sha256(&staging)?!=saved["sha256"].as_str().ok_or("Нет SHA")?{return Err("Внутренний файл публикации изменился. Перезапись запрещена.".into());}fs::remove_file(source).map_err(|e|e.to_string())?;}
   metadata["sha256"]=saved["sha256"].clone();metadata["width"]=saved["width"].clone();metadata["height"]=saved["height"].clone();metadata["stagingRelative"]=json!(staging_relative);db.execute("UPDATE editor_publications SET metadata=?2 WHERE id=?1",params![id,metadata.to_string()]).map_err(|e|e.to_string())?;
  }
  let expected=metadata["sha256"].as_str().ok_or("Нет SHA публикации")?.to_string();
  processing::cloud_fence(app,||{
   let _files=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File operation lock failed")?;
   let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;
   require_live_intent(&db,id)?;
   if !target.try_exists().map_err(|e|e.to_string())?{
    let mut source=guarded_file::GuardedFile::open(&staging,false)?;
    if source.sha()?!=expected{return Err("Файл публикации повреждён.".into());}
    let token=stable_text_id("pub-stage",id).replace(':',"-");
    let temporary=target.parent().ok_or("Нет папки назначения")?.join(format!(".smart-photo-{token}-{}.partial",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    let mut output=guarded_file::GuardedFile::create_new(&temporary)?;
    let published=(||{source.copy_to(&mut output)?;if output.sha()?!=expected{return Err("Проверка сохранения не прошла.".into());}output.rename(&target)})();
    if let Err(error)=published{let _=output.remove();return Err(error);}
   }
   if compute_file_sha256(&target)?!=expected{return Err("По выбранному пути находится другой файл. Он не был заменён.".into());}
   db.execute("UPDATE editor_publications SET state='indexPending',error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
   Ok(())
  })?;
  let info=fs::metadata(&target).map_err(|e|e.to_string())?;let input=IndexImageInputDto{path:path_to_string(&target),file_name:target.file_name().ok_or("Нет имени")?.to_string_lossy().into_owned(),extension:extension(&target),size_bytes:Some(info.len()),modified_at_unix:info.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|t|t.as_secs()),width:metadata["width"].as_u64().map(|v|v as u32),height:metadata["height"].as_u64().map(|v|v as u32)};
  let indexed=sync_scanned_images_to_index_internal(app.clone(),vec![input])?;metadata["indexImageIds"]=json!(indexed.indexed_images.iter().map(|i|&i.image_id).collect::<Vec<_>>());
  db.execute("UPDATE editor_publications SET state='completed',metadata=?2,error=NULL WHERE id=?1",params![id,metadata.to_string()]).map_err(|e|e.to_string())?;
  use tauri::Emitter;let _=app.emit("smart-photo-indexer:editor-publications-changed",json!({"id":id}));Ok(json!({"id":id,"state":"completed","path":path_to_string(&target),"sha256":expected,"imageIds":metadata["indexImageIds"]}))
 })();if let Err(error)=&result{let exists=Path::new(&target).exists();db.execute("UPDATE editor_publications SET error=?2,state=CASE WHEN state='planned' AND ?3=0 THEN 'failed' ELSE state END WHERE id=?1",params![id,error,exists]).map_err(|e|e.to_string())?;}result
}
pub fn recover(app:&tauri::AppHandle)->Result<(),String>{super::replacement::recover(app)?;let db=store::connection(app)?;let ids={let mut q=db.prepare("SELECT id FROM editor_publications WHERE state IN ('fileSaved','indexPending')").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;rows};for id in ids{let _=reconcile(app,&id);}Ok(())}
pub fn cancel(app:&tauri::AppHandle,id:&str)->Result<Value,String>{
 if store::connection(app)?.query_row("SELECT COALESCE(json_extract(metadata,'$.mode'),'copy')='replace' FROM editor_publications WHERE id=?1",[id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return super::replacement::cancel(app,id);}
 let db=store::connection(app)?;let(state,target):(String,String)=db.query_row("SELECT state,target_path FROM editor_publications WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
 if ["fileSaved","indexPending","completed"].contains(&state.as_str())||Path::new(&target).exists(){return Err("Файл уже записан или исход записи требует проверки. Продолжите сохранение индекса; файл не удалён.".into());}
 db.execute("UPDATE editor_publications SET state='cancelled',error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
 use tauri::Emitter;let _=app.emit("smart-photo-indexer:editor-publications-changed",json!({"id":id}));Ok(json!({"id":id,"state":"cancelled"}))
}
