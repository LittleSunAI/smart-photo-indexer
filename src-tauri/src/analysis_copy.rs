//! Recognition's file phase: keep the original and publish one verified copy.
use crate::*;
pub fn prepare(db:&Connection,item:&mut SmartRenamePreviewItemDto,copy:&profile_preferences::AnalysisCopy)->Result<String,String>{
 if copy.destination=="folder"{let folder=Path::new(&copy.folder);if !folder.is_absolute()||!folder.is_dir(){return Err("Папка копий недоступна. Выберите существующую папку в настройках.".into());}item.new_path=path_to_string(&fs::canonicalize(folder).map_err(|e|e.to_string())?.join(&item.new_file_name));}
 else if copy.destination!="beside"{return Err("Неизвестное место сохранения копии.".into());}
 let parent=Path::new(&item.new_path).parent().ok_or("Нет папки назначения")?.to_path_buf();
 let sha=compute_file_sha256(Path::new(&item.old_path))?;let prefix=item.new_file_name.get(..10).ok_or("Неверная дата умного имени")?;let extension=extension(Path::new(&item.old_path)).ok_or("Нет расширения изображения")?;
 let mut available=None;
 for salt in 0..10_000{let code=smart_rename_unique8(Path::new(&item.old_path),item.size_bytes.ok_or("Нет размера исходника")?,item.modified_at_unix.ok_or("Нет даты исходника")?,Some(&sha),salt);let name=format!("{prefix}_{code}.{extension}");let path=path_to_string(&parent.join(&name));let reserved:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM file_operations WHERE target_path=?1 COLLATE NOCASE AND status IN('planned','applying','needsRecovery'))",[&path],|r|r.get(0)).map_err(|e|e.to_string())?;if !reserved&&!Path::new(&path).exists(){available=Some((path,name));break;}item.candidate_collisions+=1;}
 let(path,name)=available.ok_or("Не найдено свободное умное имя для копии")?;item.new_path=path;item.new_file_name=name;item.status="ready".into();item.message="Создать копию; исходник останется на месте.".into();
 let id=file_operations::prepare_rename_plan(db,item)?;
 db.execute("UPDATE file_operations SET kind='analysisCopy' WHERE id=?1 AND status='planned'",[&id]).map_err(|e|e.to_string())?;Ok(id)
}
fn commit(db:&mut Connection,id:&str)->Result<(),String>{
 let tx=db.transaction().map_err(|e|e.to_string())?;
 let(image,source,target,sha):(String,String,String,String)=tx.query_row("SELECT image_id,source_path,target_path,sha256 FROM file_operations WHERE id=?1 AND kind='analysisCopy'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
 let indexed=read_indexed_sha256(&tx,&image)?;if indexed.as_deref()!=Some(&sha){return Err("Запись исходника изменилась; копия сохранена для сверки.".into());}
 tx.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES(?1,?2,1,?3,?3) ON CONFLICT(path) DO UPDATE SET is_current=1,missing_at_unix=NULL,last_seen_at_unix=excluded.last_seen_at_unix WHERE image_paths.image_id=excluded.image_id",params![image,target,unix_now()]).map_err(|e|e.to_string())?;
 let owner:String=tx.query_row("SELECT image_id FROM image_paths WHERE path=?1",[&target],|r|r.get(0)).map_err(|e|e.to_string())?;if owner!=image{return Err("Путь копии принадлежит другой записи библиотеки.".into());}
 tx.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) VALUES('analysis_copy_created',?1,?2,?3,'Копия с умным именем сохранена; исходник сохранён',?4)",params![image,source,target,unix_now()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE file_operations SET status='committed',error=NULL,updated_at_unix=?2 WHERE id=?1",params![id,unix_now()]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())
}
fn stage(target:&Path,id:&str)->Result<PathBuf,String>{Ok(target.parent().ok_or("Нет папки назначения")?.join(format!(".smart-photo-{id}.partial")))}
pub fn apply(db:&mut Connection,id:&str)->Result<String,String>{
 let _ops=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File operation lock failed")?;let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;
 let(source,target,sha,status):(String,String,String,String)=db.query_row("SELECT source_path,target_path,sha256,status FROM file_operations WHERE id=?1 AND kind='analysisCopy'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
 if status=="committed"{return Ok(target);}
 if !["planned","failed"].contains(&status.as_str()){return Err("Копия требует восстановления перед повтором.".into());}
 let mut original=guarded_file::GuardedFile::open(Path::new(&source),false)?;if original.sha()?!=sha{return Err("Исходный файл изменился; копия не создана.".into());}
 let temporary=stage(Path::new(&target),id)?;
 db.execute("UPDATE file_operations SET status='applying',error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
 let mut published=false;let mut cleaned=false;
 let result=(||{let mut output=guarded_file::GuardedFile::create_new(&temporary)?;
  let written=(||{db.execute("UPDATE file_operations SET stage_identity=?2 WHERE id=?1",params![id,output.identity()?]).map_err(|e|e.to_string())?;original.copy_to(&mut output)?;if output.sha()?!=sha{return Err("Проверка содержимого копии не прошла.".into());}output.rename(Path::new(&target))?;Ok(())})();
  if let Err(error)=written{cleaned=output.remove().is_ok();return Err(error);}published=true;drop(output);commit(db,id)?;Ok(target)
 })();
 if let Err(error)=&result{db.execute("UPDATE file_operations SET status=?3,error=?2 WHERE id=?1",params![id,error,if !published&&cleaned{"failed"}else{"needsRecovery"}]).map_err(|e|e.to_string())?;}result
}
pub fn recover(db:&mut Connection)->Result<(),String>{
 let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;
 let ids:Vec<String>={let mut query=db.prepare("SELECT id FROM file_operations WHERE kind='analysisCopy' AND status IN('applying','needsRecovery')").map_err(|e|e.to_string())?;let rows=query.query_map([],|r|r.get(0)).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?};
 for id in ids{if let Err(error)=recover_one(db,&id){db.execute("UPDATE file_operations SET status='needsRecovery',error=?2,updated_at_unix=?3 WHERE id=?1",params![id,error,unix_now()]).map_err(|e|e.to_string())?;}}Ok(())
}
fn recover_one(db:&mut Connection,id:&str)->Result<(),String>{
 let(source,target,sha,identity):(String,String,String,Option<String>)=db.query_row("SELECT source_path,target_path,sha256,stage_identity FROM file_operations WHERE id=?1 AND kind='analysisCopy'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
 let target=Path::new(&target);let temporary=stage(target,id)?;
 match fs::symlink_metadata(target){
  Ok(_)=>{let mut published=guarded_file::GuardedFile::open(target,false)?;if published.sha()?!=sha||temporary.try_exists().map_err(|e|e.to_string())?{return Err("Назначение занято или осталось несколько результатов; файлы сохранены для сверки.".into());}return commit(db,id);},
  Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>return Err(e.to_string())
 }
 let parent=target.parent().ok_or("Нет папки назначения")?;fs::read_dir(parent).map_err(|e|format!("Папка копии недоступна: {e}"))?;
 let mut original=guarded_file::GuardedFile::open(Path::new(&source),false)?;if original.sha()?!=sha{return Err("Исходник изменился; временная копия оставлена для сверки.".into());}
 match fs::symlink_metadata(&temporary){
  Ok(_)=>{let mut output=guarded_file::GuardedFile::open(&temporary,true)?;if identity.as_deref()!=Some(output.identity()?.as_str()){return Err("Принадлежность временного файла не подтверждена; он сохранён.".into());}
   if output.sha()?==sha{output.rename(target)?;drop(output);return commit(db,id);}output.remove()?;},
  Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>return Err(e.to_string())
 }
 db.execute("UPDATE file_operations SET status='failed',stage_identity=NULL,error='Прерванное копирование отменено; исходник проверен, можно повторить файловую фазу',updated_at_unix=?2 WHERE id=?1",params![id,unix_now()]).map_err(|e|e.to_string())?;Ok(())
}

#[cfg(all(test,windows))]mod file_phase_tests{
 use super::*;
 fn fixture()->(Connection,PathBuf,String){
  let root=crate::test_support::temp_dir("smart-photo-copy");let source=root.join("original.png");fs::write(&source,b"immutable original pixels").unwrap();let sha=compute_file_sha256(&source).unwrap();
  let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();db.execute("INSERT INTO images(id,identity_key,file_name,sha256,status,created_at_unix,updated_at_unix) VALUES('i','i','original.png',?1,'processed',1,1)",[&sha]).unwrap();db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('i',?1,1,1,1)",[path_to_string(&source)]).unwrap();(db,root,sha)
 }
 fn plan(db:&Connection,root:&Path,id:&str,sha:&str)->PathBuf{let target=root.join(format!("{id}.png"));db.execute("INSERT INTO file_operations(id,kind,image_id,source_path,target_path,sha256,size_bytes,status,created_at_unix,updated_at_unix) VALUES(?1,'analysisCopy','i',?2,?3,?4,25,'applying',1,1)",params![id,path_to_string(&root.join("original.png")),path_to_string(&target),sha]).unwrap();target}
 fn staged(db:&Connection,root:&Path,id:&str,target:&Path,complete:bool)->PathBuf{let temporary=stage(target,id).unwrap();let mut output=guarded_file::GuardedFile::create_new(&temporary).unwrap();db.execute("UPDATE file_operations SET stage_identity=?2 WHERE id=?1",params![id,output.identity().unwrap()]).unwrap();if complete{guarded_file::GuardedFile::open(&root.join("original.png"),false).unwrap().copy_to(&mut output).unwrap();}temporary}
 fn status(db:&Connection,id:&str)->String{db.query_row("SELECT status FROM file_operations WHERE id=?1",[id],|r|r.get(0)).unwrap()}
 #[test]fn incomplete_owned_stage_is_removed_and_only_file_phase_can_retry(){let(mut db,root,sha)=fixture();let target=plan(&db,&root,"partial",&sha);let temporary=staged(&db,&root,"partial",&target,false);recover(&mut db).unwrap();assert_eq!(status(&db,"partial"),"failed");assert!(!temporary.exists());assert!(!target.exists());apply(&mut db,"partial").unwrap();assert_eq!(compute_file_sha256(&target).unwrap(),sha);assert_eq!(compute_file_sha256(&root.join("original.png")).unwrap(),sha);recover(&mut db).unwrap();assert_eq!(db.query_row("SELECT COUNT(*) FROM action_logs WHERE action_type='analysis_copy_created'",[],|r|r.get::<_,u32>(0)).unwrap(),1);fs::remove_dir_all(root).unwrap();}
 #[test]fn complete_owned_stage_is_published_and_committed_once(){let(mut db,root,sha)=fixture();let target=plan(&db,&root,"complete",&sha);let temporary=staged(&db,&root,"complete",&target,true);recover(&mut db).unwrap();assert_eq!(status(&db,"complete"),"committed");assert!(!temporary.exists());assert_eq!(compute_file_sha256(&target).unwrap(),sha);assert_eq!(db.query_row("SELECT COUNT(*) FROM image_paths WHERE image_id='i' AND is_current=1",[],|r|r.get::<_,u32>(0)).unwrap(),2);recover(&mut db).unwrap();assert_eq!(db.query_row("SELECT COUNT(*) FROM action_logs WHERE action_type='analysis_copy_created'",[],|r|r.get::<_,u32>(0)).unwrap(),1);fs::remove_dir_all(root).unwrap();}
 #[test]fn foreign_or_legacy_stage_is_never_deleted_or_published(){let(mut db,root,sha)=fixture();let target=plan(&db,&root,"foreign",&sha);let temporary=staged(&db,&root,"foreign",&target,false);guarded_file::GuardedFile::open(&temporary,true).unwrap().remove().unwrap();fs::write(&temporary,b"foreign replacement").unwrap();let legacy=plan(&db,&root,"legacy",&sha);let legacy_stage=stage(&legacy,"legacy").unwrap();fs::copy(root.join("original.png"),&legacy_stage).unwrap();recover(&mut db).unwrap();assert_eq!(status(&db,"foreign"),"needsRecovery");assert_eq!(status(&db,"legacy"),"needsRecovery");assert_eq!(fs::read(&temporary).unwrap(),b"foreign replacement");assert!(legacy_stage.exists());assert!(!target.exists());assert!(!legacy.exists());fs::remove_dir_all(root).unwrap();}
 #[test]fn foreign_target_does_not_block_recovery_of_an_independent_copy(){let(mut db,root,sha)=fixture();let foreign=plan(&db,&root,"occupied",&sha);fs::write(&foreign,b"foreign target").unwrap();let valid=plan(&db,&root,"valid",&sha);staged(&db,&root,"valid",&valid,true);recover(&mut db).unwrap();assert_eq!(status(&db,"occupied"),"needsRecovery");assert_eq!(status(&db,"valid"),"committed");assert_eq!(fs::read(foreign).unwrap(),b"foreign target");assert_eq!(compute_file_sha256(&valid).unwrap(),sha);fs::remove_dir_all(root).unwrap();}
}
