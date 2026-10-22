//! Explicit original replacement. Never changes the copy/export protection policy.
use crate::*;
use serde_json::{json,Value};
use super::{store,library};
use tauri::Emitter;

pub fn preview(app:&tauri::AppHandle,project:&str,asset:&str,turn:u8,mirrored:bool,quality:u8)->Result<Value,String>{
 let db=store::connection(app)?;
 let(path,root_sha,revision):(String,String,u64)=db.query_row("SELECT source_path,source_sha256,revision FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
 let source=Path::new(&path);let mut original=guarded_file::GuardedFile::open(source,false).map_err(|e|format!("Исходник недоступен. Сохраните копию. {e}"))?;
 let sha=original.sha()?;
 let previous:Option<String>=db.query_row("SELECT json_extract(metadata,'$.sha256') FROM editor_publications WHERE project_id=?1 AND state='completed' AND json_extract(metadata,'$.mode')='replace' ORDER BY rowid DESC LIMIT 1",[project],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 if sha!=previous.unwrap_or(root_sha){return Err("Исходник изменён вне редактора. Для сохранения результата выберите «Сохранить копию».".into());}
 let format=match extension(source).as_deref(){Some("png")=>"png",Some("jpg"|"jpeg")=>"jpeg",Some("tif"|"tiff")=>"tiff",Some("bmp")=>"bmp",_=>return Err("Формат исходника не поддерживает замену. Сохраните копию в подходящем формате.".into())};
 library::validate_export(app,project,asset,turn,format,quality)?;
 let id=format!("replace-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
 let relative=format!("assets/{id}.{}",if format=="jpeg"{"jpg"}else{format});let rendered=store::owned_path(app,&relative)?;
 let output=library::export_image(app,project,asset,turn,mirrored,&rendered,format,quality)?;
 let metadata=json!({"mode":"replace","targetKey":super::migration::publication_target_key(&path),"confirmed":false,"projectRevision":revision,"parameters":{"assetId":asset,"workingTurn":turn,"mirrored":mirrored,"format":format,"quality":quality},"originalSha256":sha,"originalRevision":index_observations::content_revision(source)?,"stagingRelative":relative,"sha256":output["sha256"],"width":output["width"],"height":output["height"]});
 processing::cloud_fence(app,||{if index_backups::restore_blocked(app)?{return Err("Сначала завершите восстановление библиотеки.".into());}db.execute("INSERT INTO editor_publications(id,project_id,asset_id,state,target_path,metadata,created_at_unix) VALUES(?1,?2,?3,'planned',?4,?5,?6)",params![id,project,asset,path,metadata.to_string(),unix_now()]).map_err(|e|e.to_string())?;Ok(())})?;
 Ok(json!({"id":id,"path":path,"fileName":file_name(source),"format":format,"width":output["width"],"height":output["height"]}))
}
fn read(app:&tauri::AppHandle,id:&str)->Result<(String,String,String,Value),String>{store::connection(app)?.query_row("SELECT project_id,target_path,state,metadata FROM editor_publications WHERE id=?1 AND json_extract(metadata,'$.mode')='replace'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,serde_json::from_str(&r.get::<_,String>(3)?).unwrap_or(Value::Null)))).map_err(|e|e.to_string())}
fn stage_paths(path:&Path,id:&str)->Result<(PathBuf,PathBuf,PathBuf),String>{let folder=path.parent().ok_or("Нет папки исходника")?.join(format!(".smart-photo-{id}"));Ok((folder.clone(),folder.join("original"),folder.join("replacement")))}
pub fn apply(app:&tauri::AppHandle,project:&str,id:&str)->Result<Value,String>{
 let(owner,path,state,mut metadata)=read(app,id)?;if owner!=project{return Err("Сохранение относится к другому проекту.".into());}if state=="completed"{return resume(app,id);}
 if state!="planned"{return resume(app,id);}
 let revision:u64=store::connection(app)?.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if metadata["projectRevision"].as_u64()!=Some(revision){return Err("Проект изменился после предпросмотра. Повторите выбор сохранения.".into());}
 let original_sha=metadata["originalSha256"].as_str().ok_or("Нет отпечатка исходника")?.to_string();let new_sha=metadata["sha256"].as_str().ok_or("Нет отпечатка результата")?.to_string();
 let staged=store::owned_path(app,metadata["stagingRelative"].as_str().ok_or("Нет подготовленного результата")?)?;
 let result=processing::cloud_fence(app,||{
  if index_backups::restore_blocked(app)?{return Err("Сначала завершите восстановление библиотеки.".into());}
  let _operations=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File operation lock failed")?;let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;
  let source=Path::new(&path);let mut original=guarded_file::GuardedFile::open(source,true)?;
  if original.sha()!=Ok(original_sha.clone())||index_observations::content_revision(source)?!=metadata["originalRevision"].as_str().unwrap_or(""){return Err("Исходник изменился после подтверждаемого предпросмотра. Сохраните копию.".into());}
  let mut rendered=guarded_file::GuardedFile::open(&staged,false)?;if rendered.sha()?!=new_sha{return Err("Подготовленный результат повреждён. Исходник сохранён.".into());}
  // Preserve the exact previous bytes inside the project before touching the external file.
  let original_relative=format!("assets/{id}-original.{}",extension(source).ok_or("Нет расширения")?);let retained=store::owned_path(app,&original_relative)?;let mut retained_file=guarded_file::GuardedFile::create_new(&retained)?;original.copy_to(&mut retained_file)?;if retained_file.sha()?!=original_sha{return Err("Не удалось сохранить исходник внутри проекта.".into());}drop(retained_file);
  let(folder,backup,temporary)=stage_paths(source,id)?;
  metadata["confirmed"]=json!(true);metadata["originalRelative"]=json!(original_relative);
  let db=store::connection(app)?;let changed=db.execute("UPDATE editor_publications SET state='replacing',metadata=?2,error=NULL WHERE id=?1 AND state='planned'",params![id,metadata.to_string()]).map_err(|e|e.to_string())?;if changed!=1{return Err("Сохранение уже выполняется.".into());}
  fs::create_dir(&folder).map_err(|e|e.to_string())?;
  let mut output=guarded_file::GuardedFile::create_new(&temporary)?;rendered.copy_to(&mut output)?;if output.sha()?!=new_sha{return Err("Не прошла проверка временного результата. Исходник сохранён.".into());}
  original.rename(&backup)?;
  if let Err(error)=output.rename(source){let restored=original.rename(source);return Err(format!("Результат не опубликован: {error}. {}",if restored.is_ok(){"Исходник восстановлен."}else{"Исходник сохранён для восстановления; существующий файл не заменён."}));}
  db.execute("UPDATE editor_publications SET state='indexPending',error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
  // The original also remains in the immutable project store; cleanup is identity-bound.
  let _=original.remove();drop(output);let _=fs::remove_dir(&folder);Ok(())
 });
 if let Err(error)=result{let _=store::connection(app)?.execute("UPDATE editor_publications SET error=?2 WHERE id=?1",params![id,error]);return Err(error);}
 match resume(app,id){Ok(value)=>Ok(value),Err(error)=>{let(_,_,state,_)=read(app,id)?;if state=="indexPending"{let _=app.emit("smart-photo-indexer:editor-publications-changed",json!({"id":id}));Ok(json!({"id":id,"path":path,"state":"indexPending","warning":error}))}else{Err(error)}}}
}
pub fn resume(app:&tauri::AppHandle,id:&str)->Result<Value,String>{
 let(_,path,mut state,mut metadata)=read(app,id)?;if state=="completed"{return Ok(json!({"id":id,"path":path,"state":state}));}
 if state=="planned"||metadata["confirmed"]!=true{return Err("Замена ещё не подтверждена. Откройте сохранение изображения в проекте.".into());}
 if state=="replacing"||state=="needsRecovery"{
  recover_one(app,id)?;state=read(app,id)?.2;
 }
 if state!="indexPending"{return Err("Замена не завершена; сохранённые файлы оставлены для восстановления.".into());}
 let expected=metadata["sha256"].as_str().ok_or("Нет SHA результата")?.to_string();
 let mut file=guarded_file::GuardedFile::open(Path::new(&path),false)?;if file.sha()?!=expected{return Err("Сохранённый файл изменился до обновления библиотеки. Повторная запись изображения не выполнялась.".into());}
 let info=fs::metadata(&path).map_err(|e|e.to_string())?;
 let indexed=sync_scanned_images_to_index_internal(app.clone(),vec![IndexImageInputDto{path:path.clone(),file_name:file_name(Path::new(&path)),extension:extension(Path::new(&path)),size_bytes:Some(info.len()),modified_at_unix:modified_at_unix(&info),width:metadata["width"].as_u64().map(|v|v as u32),height:metadata["height"].as_u64().map(|v|v as u32)}])?;
 metadata["indexImageIds"]=json!(indexed.indexed_images.iter().map(|i|&i.image_id).collect::<Vec<_>>());
 store::connection(app)?.execute("UPDATE editor_publications SET state='completed',metadata=?2,error=NULL WHERE id=?1 AND state='indexPending'",params![id,metadata.to_string()]).map_err(|e|e.to_string())?;
 let _=viewer_sessions::refresh_contents(app,&path);let _=app.emit("smart-photo-indexer:editor-original-replaced",json!({"path":path,"indexResult":indexed}));let _=app.emit("smart-photo-indexer:editor-publications-changed",json!({"id":id}));Ok(json!({"id":id,"path":path,"state":"completed","sha256":expected}))
}
fn recover_one(app:&tauri::AppHandle,id:&str)->Result<(),String>{
 processing::cloud_fence(app,||{let _operations=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File operation lock failed")?;let(_,path,_,metadata)=read(app,id)?;let source=Path::new(&path);let(folder,backup,_)=stage_paths(source,id)?;
 let expected=metadata["sha256"].as_str().ok_or("Нет SHA результата")?;let old=metadata["originalSha256"].as_str().ok_or("Нет SHA исходника")?;
 let source_hash=compute_file_sha256(source).ok();let backup_hash=compute_file_sha256(&backup).ok();
 let (state,message)=if source_hash.as_deref()==Some(expected){("indexPending",None)}else if source_hash.is_none()&&!source.exists()&&source.parent().is_some_and(|p|fs::read_dir(p).is_ok())&&backup_hash.as_deref()==Some(old){let mut guard=guarded_file::GuardedFile::open(&backup,true)?;if guard.sha()?!=old{return Err("Резервный исходник изменился.".into());}guard.rename(source)?;("failed",Some("Прерванная замена отменена; исходник восстановлен."))}else if source_hash.as_deref()==Some(old)&&!backup.exists(){("failed",Some("Запись была прервана до замены; исходник сохранён."))}else{("needsRecovery",Some("Неоднозначное состояние файлов. Исходник и результат сохранены; перезапись запрещена."))};
 store::connection(app)?.execute("UPDATE editor_publications SET state=?2,error=?3 WHERE id=?1",params![id,state,message]).map_err(|e|e.to_string())?;
 if state=="indexPending"&&backup_hash.as_deref()==Some(old){if let Ok(mut guard)=guarded_file::GuardedFile::open(&backup,true){if guard.sha().as_deref()==Ok(old){let _=guard.remove();}}let _=fs::remove_dir(folder);}Ok(())})
}
pub fn recover(app:&tauri::AppHandle)->Result<(),String>{let db=store::connection(app)?;let ids:Vec<String>={let mut q=db.prepare("SELECT id FROM editor_publications WHERE json_extract(metadata,'$.mode')='replace' AND state IN('replacing','needsRecovery','indexPending')").map_err(|e|e.to_string())?;let r=q.query_map([],|r|r.get(0)).map_err(|e|e.to_string())?;r.collect::<Result<_,_>>().map_err(|e|e.to_string())?};for id in ids{let _=resume(app,&id);}Ok(())}
pub fn cancel(app:&tauri::AppHandle,id:&str)->Result<Value,String>{let(_,_,state,metadata)=read(app,id)?;if state!="planned"||metadata["confirmed"]==true{return Err("Замена уже началась. Используйте восстановление сохранения.".into());}store::connection(app)?.execute("UPDATE editor_publications SET state='cancelled' WHERE id=?1",[id]).map_err(|e|e.to_string())?;Ok(json!({"id":id,"state":"cancelled"}))}
