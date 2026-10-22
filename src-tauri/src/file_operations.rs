use super::*;
pub(super) static OPERATIONS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) fn prepare_rename_plan(connection: &Connection, item: &SmartRenamePreviewItemDto) -> Result<String, String> {
    let source = Path::new(&item.old_path);
    let sha = compute_file_sha256(source)?;
    let image_id: String = connection.query_row(
        "SELECT image_id FROM image_paths WHERE path=?1 AND is_current=1", params![&item.old_path], |row| row.get(0)
    ).map_err(|error| format!("Index the file before preparing a rename: {error}"))?;
    if item.image_id.as_ref().is_some_and(|id| id != &image_id) {
        return Err("The preview file identity changed. Reload and prepare a new preview.".to_string());
    }
    let indexed_sha = read_indexed_sha256(connection, &image_id)?;
    if indexed_sha.as_deref() != Some(sha.as_str()) { return Err("The preview file changed since indexing.".to_string()); }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
    let id = format!("rename-{}-{nonce}", std::process::id());
    connection.execute(
        "INSERT INTO file_operations(id,kind,image_id,source_path,target_path,sha256,size_bytes,modified_at_unix,status,created_at_unix,updated_at_unix)
         VALUES (?1,'rename',?2,?3,?4,?5,?6,?7,'planned',?8,?8)",
        params![&id,image_id,&item.old_path,&item.new_path,sha,item.size_bytes.map(u64_to_i64_saturating),
            item.modified_at_unix.map(u64_to_i64_saturating),u64_to_i64_saturating(unix_now())]
    ).map_err(|error| format!("Cannot persist rename preview: {error}"))?;
    Ok(id)
}

#[derive(Debug)]
struct Operation {
    image_id: String, source: String, target: String, sha: String, size: i64, modified: Option<i64>, status: String,
}
fn read_operation(connection: &Connection, id: &str) -> Result<Operation, String> {
    connection.query_row(
        "SELECT image_id,source_path,target_path,sha256,size_bytes,modified_at_unix,status FROM file_operations WHERE id=?1 AND kind='rename'",
        params![id], |row| Ok(Operation { image_id: row.get(0)?, source: row.get(1)?, target: row.get(2)?,
            sha: row.get(3)?, size: row.get(4)?, modified: row.get(5)?, status: row.get(6)? })
    ).map_err(|error| format!("Rename plan was not found: {error}"))
}

fn commit_rename(connection: &mut Connection, id: &str, operation: &Operation) -> Result<i64, String> {
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let now = u64_to_i64_saturating(unix_now());
    transaction.execute(
        "UPDATE image_paths SET is_current=0,missing_at_unix=?3,last_seen_at_unix=?3 WHERE image_id=?1 AND path=?2",
        params![&operation.image_id,&operation.source,now]
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix,missing_at_unix)
         VALUES (?1,?2,1,?3,?3,NULL) ON CONFLICT(path) DO UPDATE SET is_current=1,last_seen_at_unix=excluded.last_seen_at_unix,missing_at_unix=NULL
         WHERE image_paths.image_id=excluded.image_id",
        params![&operation.image_id,&operation.target,now]
    ).map_err(|error| error.to_string())?;
    let target_image: String = transaction.query_row(
        "SELECT image_id FROM image_paths WHERE path=?1", params![&operation.target], |row| row.get(0)
    ).map_err(|error| error.to_string())?;
    if target_image != operation.image_id { return Err("Target belongs to a different indexed image; recovery needs reconciliation.".to_string()); }
    transaction.execute("UPDATE image_paths SET size_bytes=?1,modified_at_unix=?2 WHERE image_id=?3 AND path=?4",params![operation.size,operation.modified,operation.image_id,operation.target]).map_err(|e|e.to_string())?;
    transaction.execute(
        "UPDATE images SET file_name=?2,updated_at_unix=?3 WHERE id=?1",
        params![&operation.image_id,file_name(Path::new(&operation.target)),now]
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix)
         VALUES ('file_renamed',?1,?2,?3,?4,?5)",
        params![&operation.image_id,&operation.source,&operation.target,
            format!("Rename committed with SHA-256 verification; operation {id}"),now]
    ).map_err(|error| error.to_string())?;
    let log_id = transaction.last_insert_rowid();
    transaction.execute("UPDATE file_operations SET status='committed',error=NULL,updated_at_unix=?2 WHERE id=?1",
        params![id,now]).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(log_id)
}

pub(super) fn rename(app_handle: tauri::AppHandle, input: SmartRenameInputDto) -> Result<SmartRenameResultDto, String> {
    let _guard = OPERATIONS_LOCK.lock().map_err(|_| "File operation lock failed.".to_string())?;
    if input.pattern != "date_unique8" { return Err("Unsupported rename pattern.".to_string()); }
    let id = input.plan_id.as_deref().filter(|id| !id.is_empty())
        .ok_or("A persisted rename preview must be confirmed before renaming.")?;
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let operation = read_operation(&connection, id)?;
    if operation.status != "planned" { return Err("This rename preview has already been used. Prepare a new preview.".to_string()); }
    if input.current_path != operation.source || input.image_id.as_ref().is_some_and(|id| id != &operation.image_id) {
        return Err("The confirmed preview refers to a different file.".to_string());
    }
    let source = Path::new(&operation.source);
    let target = Path::new(&operation.target);
    if source.parent() != target.parent() { return Err("Rename target must be in the source directory.".to_string()); }
    let mut protected=crate::guarded_file::GuardedFile::open(source,true)?;
    if protected.sha()? != operation.sha {
        return Err("Source changed after the preview. Prepare a new preview.".to_string());
    }
    if fs::symlink_metadata(target).is_ok() { return Err("The preview target is now occupied. No file was replaced.".to_string()); }
    let source_image: String = connection.query_row("SELECT image_id FROM image_paths WHERE path=?1 AND is_current=1",
        params![&operation.source], |row| row.get(0)).map_err(|error| error.to_string())?;
    if source_image != operation.image_id { return Err("The indexed source identity changed after preview.".to_string()); }
    let target_image: Option<String> = connection.query_row("SELECT image_id FROM image_paths WHERE path=?1",
        params![&operation.target], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
    if target_image.as_ref().is_some_and(|id| id != &operation.image_id) {
        return Err("The target path belongs to another indexed image.".to_string());
    }
    let claimed = connection.execute("UPDATE file_operations SET status='applying',updated_at_unix=?2 WHERE id=?1 AND status='planned'",
        params![id,u64_to_i64_saturating(unix_now())]).map_err(|error| error.to_string())?;
    if claimed != 1 { return Err("The rename plan is already executing.".to_string()); }
    if let Err(error) = protected.rename(target) {
        connection.execute("UPDATE file_operations SET status='failed',error=?2 WHERE id=?1", params![id,&error])
            .map_err(|log_error| format!("{error}; cannot save operation error: {log_error}"))?;
        return Err(error);
    }
    let result = (|| {
        if protected.sha()? != operation.sha || source.exists() {
            return Err("Rename verification failed. The operation is preserved for recovery.".to_string());
        }
        commit_rename(&mut connection, id, &operation)
    })();
    let log_id = match result {
        Ok(log_id) => log_id,
        Err(error) => {
            connection.execute("UPDATE file_operations SET status='needsRecovery',error=?2 WHERE id=?1", params![id,&error])
                .map_err(|log_error| format!("{error}; cannot save recovery state: {log_error}"))?;
            return Err(error);
        }
    };
    let _=viewer_sessions::rename_path(&app_handle,&operation.source,&operation.target);
    Ok(SmartRenameResultDto { image_id: Some(operation.image_id), old_path: operation.source.clone(),
        new_path: operation.target.clone(), old_file_name: file_name(source), new_file_name: file_name(target),
        was_renamed: true, size_bytes: Some(operation.size as u64), modified_at_unix: operation.modified.map(|value| value as u64),
        action_log_id: Some(log_id), message: "Confirmed rename completed and committed; original contents verified.".to_string() })
}

pub(super) fn recover(connection: &mut Connection) -> Result<(), String> {
    let Ok(_guard) = OPERATIONS_LOCK.try_lock() else { return Ok(()); };
    analysis_copy::recover(connection)?;
    let ids: Vec<String> = {
        let mut statement = connection.prepare("SELECT id FROM file_operations WHERE kind='rename' AND status IN ('applying','needsRecovery')")
            .map_err(|error| error.to_string())?;
        let rows = statement.query_map([], |row| row.get(0)).map_err(|error| error.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|error| error.to_string())?
    };
    for id in ids {
        let operation = read_operation(connection, &id)?;
        let source = Path::new(&operation.source);
        let target = Path::new(&operation.target);
        if !source.exists() && target.is_file() && compute_file_sha256(target)? == operation.sha {
            commit_rename(connection, &id, &operation)?;
        } else if source.is_file() && !target.exists() && compute_file_sha256(source)? == operation.sha {
            connection.execute("UPDATE file_operations SET status='failed',error='Interrupted before the file move; source preserved' WHERE id=?1",
                params![id]).map_err(|error| error.to_string())?;
        } else {
            connection.execute("UPDATE file_operations SET status='needsRecovery',error='Ambiguous file state; originals were preserved for manual reconciliation' WHERE id=?1",
                params![id]).map_err(|error| error.to_string())?;
        }
    }
    // A duplicate stage is reversible until the system recycle call. Never resume deletion on startup.
    let staged:Vec<(String,String,String,String)>={let mut q=connection.prepare("SELECT id,source_path,target_path,sha256 FROM file_operations WHERE kind='duplicateTrash' AND status IN('applying','needsRecovery')").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?};
    for(id,source,target,sha)in staged{let staged_path=Path::new(&target);let source_path=Path::new(&source);
        if staged_path.exists(){let restored=crate::guarded_file::GuardedFile::open(staged_path,true).and_then(|mut g|{if g.sha()?!=sha{return Err("Содержимое временной копии изменилось".into());}g.rename(source_path)});connection.execute("UPDATE file_operations SET status=?2,error=?3 WHERE id=?1",params![id,if restored.is_ok(){"failed"}else{"needsRecovery"},restored.err().unwrap_or_else(||"Прерванное удаление отменено; копия восстановлена".into())]).map_err(|e|e.to_string())?;if !staged_path.exists(){if let Some(parent)=staged_path.parent(){let _=fs::remove_dir(parent);}}}
        else if !source_path.exists()&&missing_path_is_observable(source_path){commit_trash(connection,&id,&source)?;}
        else{let preserved=crate::guarded_file::GuardedFile::open(source_path,false).and_then(|mut g|g.sha()).as_deref()==Ok(sha.as_str());connection.execute("UPDATE file_operations SET status=?2,error=?3 WHERE id=?1",params![id,if preserved{"failed"}else{"needsRecovery"},if preserved{"Interrupted deletion; original verified and preserved"}else{"Original path is inaccessible or changed; reconciliation required"}]).map_err(|e|e.to_string())?;}
    }
    let pending_trash: Vec<(String, String, String)> = {
        let mut statement = connection.prepare("SELECT id,source_path,sha256 FROM file_operations WHERE kind='trash' AND status IN ('applying','needsRecovery')")
            .map_err(|error| error.to_string())?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(|error| error.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|error| error.to_string())?
    };
    for (id, source, sha) in pending_trash {
        if !Path::new(&source).exists() {
            commit_trash(connection, &id, &source)?;
        } else {
            let status = if compute_file_sha256(Path::new(&source))? == sha { "failed" } else { "needsRecovery" };
            connection.execute("UPDATE file_operations SET status=?2,error='Interrupted recycle operation; source retained for reconciliation' WHERE id=?1",
                params![id,status]).map_err(|error| error.to_string())?;
        }
    }
    repair_committed_trash_paths(connection)?;
    Ok(())
}

// Older commits marked a recycled alias missing but left it in current searches.
// Keep its history; retire only an alias still bearing that confirmed deletion.
fn repair_committed_trash_paths(connection:&Connection)->Result<usize,String>{
    let candidates:Vec<(i64,String)>={
        let mut statement=connection.prepare("SELECT DISTINCT p.id,p.path FROM file_operations f JOIN image_paths p ON p.path=f.source_path AND p.image_id=f.image_id WHERE f.kind IN('trash','duplicateTrash') AND f.status='committed' AND p.is_current=1 AND p.missing_at_unix IS NOT NULL AND p.missing_at_unix<=f.updated_at_unix AND p.last_seen_at_unix<=f.updated_at_unix").map_err(|e|e.to_string())?;
        let rows=statement.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?
    };
    let mut repaired=0;
    for(id,path)in candidates{
        if fs::metadata(&path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound)&&missing_path_is_observable(Path::new(&path)){
            repaired+=connection.execute("UPDATE image_paths SET is_current=0 WHERE id=?1 AND is_current=1 AND missing_at_unix IS NOT NULL AND EXISTS(SELECT 1 FROM file_operations f WHERE f.source_path=image_paths.path AND f.image_id=image_paths.image_id AND f.kind IN('trash','duplicateTrash') AND f.status='committed' AND image_paths.missing_at_unix<=f.updated_at_unix AND image_paths.last_seen_at_unix<=f.updated_at_unix)",[id]).map_err(|e|e.to_string())?;
        }
    }
    Ok(repaired)
}

pub(crate) fn commit_trash(connection: &mut Connection, id: &str, source: &str) -> Result<(), String> {
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    let now = u64_to_i64_saturating(unix_now());
    let image_id: Option<String> = transaction.query_row("SELECT image_id FROM file_operations WHERE id=?1",
        params![id], |row| row.get(0)).map_err(|error| error.to_string())?;
    transaction.execute("UPDATE image_paths SET is_current=0,missing_at_unix=?2,last_seen_at_unix=?2 WHERE path=?1",
        params![source,now]).map_err(|error| error.to_string())?;
    if let Some(image_id) = image_id.as_ref() {
        transaction.execute(
            "UPDATE images SET status_before_missing=CASE WHEN status<>'missing' THEN status ELSE status_before_missing END,status='missing'
             WHERE id=?1 AND NOT EXISTS(SELECT 1 FROM image_paths WHERE image_id=?1 AND is_current=1 AND missing_at_unix IS NULL)",
            params![image_id]).map_err(|error| error.to_string())?;
    }
    transaction.execute(
        "INSERT INTO action_logs(action_type,image_id,old_value,message,created_at_unix) VALUES ('file_trashed',?1,?2,?3,?4)",
        params![image_id,source,format!("File moved to the system recycle bin; operation {id}"),now]
    ).map_err(|error| error.to_string())?;
    transaction.execute("UPDATE file_operations SET status='committed',error=NULL,updated_at_unix=?2 WHERE id=?1",
        params![id,now]).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn trash_duplicate_locked(connection:&mut Connection,source:&str,guard:crate::guarded_file::GuardedFile,sha:&str,image_id:&str)->Result<(),String>{
    let path=Path::new(source);let parent=path.parent().ok_or("Нет папки копии")?;let nonce=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();let id=format!("duplicate-trash-{}-{nonce}",std::process::id());
    let directory=parent.join(format!(".smart-photo-recycle-{nonce}"));fs::create_dir(&directory).map_err(|e|e.to_string())?;let target=directory.join(path.file_name().ok_or("Нет имени копии")?);let target_text=path_to_string(&target);
    connection.execute("INSERT INTO file_operations(id,kind,image_id,source_path,target_path,sha256,size_bytes,status,created_at_unix,updated_at_unix) VALUES(?1,'duplicateTrash',?2,?3,?4,?5,?6,'applying',?7,?7)",params![id,image_id,source,target_text,sha,u64_to_i64_saturating(fs::metadata(path).map_err(|e|e.to_string())?.len()),unix_now()]).map_err(|e|e.to_string())?;
    if let Err(error)=guard.rename(&target){connection.execute("UPDATE file_operations SET status='failed',error=?2 WHERE id=?1",params![id,error]).map_err(|e|e.to_string())?;let _=fs::remove_dir(&directory);return Err(error);}
    drop(guard);
    if let Err(error)=crate::file_actions::recycle::existing(&target){let restored=crate::guarded_file::GuardedFile::open(&target,true).and_then(|g|g.rename(path));let state=if restored.is_ok(){"failed"}else{"needsRecovery"};connection.execute("UPDATE file_operations SET status=?2,error=?3 WHERE id=?1",params![id,state,error.to_string()]).map_err(|e|e.to_string())?;if restored.is_ok(){let _=fs::remove_dir(&directory);}return Err(format!("Корзина недоступна: {error}. Безвозвратное удаление не выполнялось."));}
    if let Err(error)=commit_trash(connection,&id,source){connection.execute("UPDATE file_operations SET status='needsRecovery',error=?2 WHERE id=?1",params![id,error]).map_err(|e|e.to_string())?;return Err(error);}
    let _=fs::remove_dir(&directory);Ok(())
}

pub(super) fn trash(app_handle: tauri::AppHandle, path: String) -> Result<DeleteImageResultDto, String> {
    let _guard = OPERATIONS_LOCK.lock().map_err(|_| "File operation lock failed.".to_string())?;
    let source = validate_existing_image_file(&path)?;
    if fs::symlink_metadata(&source).map_err(|error| error.to_string())?.file_type().is_symlink() {
        return Err("Remove image links with the system file manager; the image original was preserved.".to_string());
    }
    let source_text = path_to_string(&source);
    let sha = compute_file_sha256(&source)?;
    let mut connection = open_index_connection(&app_handle)?;
    run_index_migrations(&mut connection)?;
    let image_id: Option<String> = connection.query_row("SELECT image_id FROM image_paths WHERE path=?1",
        params![&source_text], |row| row.get(0)).optional().map_err(|error| error.to_string())?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
    let id = format!("trash-{}-{nonce}", std::process::id());
    connection.execute(
        "INSERT INTO file_operations(id,kind,image_id,source_path,sha256,size_bytes,status,created_at_unix,updated_at_unix)
         VALUES (?1,'trash',?2,?3,?4,?5,'applying',?6,?6)",
        params![&id,image_id,&source_text,sha,u64_to_i64_saturating(fs::metadata(&source).map_err(|error| error.to_string())?.len()),
            u64_to_i64_saturating(unix_now())]
    ).map_err(|error| error.to_string())?;
    if let Err(error) = crate::file_actions::recycle::existing(&source) {
        connection.execute("UPDATE file_operations SET status='failed',error=?2 WHERE id=?1",
            params![&id,error.to_string()]).map_err(|log_error| format!("{error}; cannot save operation error: {log_error}"))?;
        return Err(format!("Cannot move image to the recycle bin: {error}"));
    }
    if let Err(error) = commit_trash(&mut connection, &id, &source_text) {
        connection.execute("UPDATE file_operations SET status='needsRecovery',error=?2 WHERE id=?1", params![&id,&error])
            .map_err(|log_error| format!("{error}; cannot save recovery state: {log_error}"))?;
        return Err(error);
    }
    Ok(DeleteImageResultDto { deleted_path: source_text, deleted_at_unix: unix_now() })
}

#[cfg(test)]
mod trash_alias_tests{
    use super::*;
    fn database()->Connection{
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        db.execute("INSERT INTO images(id,identity_key,file_name,sha256,description,status,is_favorite,created_at_unix,updated_at_unix) VALUES('photo','photo','source.png','owned-sha','Retained description','processed',1,1,1)",[]).unwrap();db
    }
    fn path(db:&Connection,path:&str,missing:Option<i64>,last_seen:i64){db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix,missing_at_unix) VALUES('photo',?1,1,1,?2,?3)",params![path,last_seen,missing]).unwrap();}
    fn operation(db:&Connection,id:&str,path:&str,kind:&str,status:&str){db.execute("INSERT INTO file_operations(id,kind,image_id,source_path,sha256,size_bytes,status,created_at_unix,updated_at_unix) VALUES(?1,?2,'photo',?3,'owned-sha',1,?4,1,2)",params![id,kind,path,status]).unwrap();}
    #[test]
    fn committed_trash_retires_only_its_alias_and_keeps_logical_metadata(){
        let mut db=database();path(&db,"source.png",None,1);path(&db,"keep.png",None,1);operation(&db,"discard","source.png","duplicateTrash","applying");
        commit_trash(&mut db,"discard","source.png").unwrap();
        assert_eq!(db.query_row("SELECT is_current FROM image_paths WHERE path='source.png'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert_eq!(db.query_row("SELECT is_current FROM image_paths WHERE path='keep.png'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        let kept:(String,String,i64)=db.query_row("SELECT status,description,is_favorite FROM images WHERE id='photo'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!(kept,("processed".into(),"Retained description".into(),1));
        operation(&db,"last","keep.png","trash","applying");commit_trash(&mut db,"last","keep.png").unwrap();
        assert_eq!(db.query_row("SELECT status_before_missing FROM images WHERE id='photo' AND status='missing'",[],|r|r.get::<_,String>(0)).unwrap(),"processed");
        assert_eq!(db.query_row("SELECT COUNT(*) FROM action_logs WHERE action_type='file_trashed'",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    }
    #[test]
    fn legacy_repair_preserves_restored_reobserved_failed_and_ordinary_missing_paths(){
        let db=database();let root=crate::test_support::temp_dir("smart-photo-trash-alias-test");
        for(name,kind,state,last_seen)in[("legacy-duplicate","duplicateTrash","committed",1),("legacy-plain","trash","committed",1),("failed","trash","failed",1),("ordinary-missing","rename","committed",1),("restored","trash","committed",1),("reobserved","duplicateTrash","committed",3)]{
            let filename=root.join(name);let text=path_to_string(&filename);path(&db,&text,Some(2),last_seen);operation(&db,name,&text,kind,state);if name=="restored"{fs::write(filename,b"restored owned bytes").unwrap();}
        }
        assert_eq!(repair_committed_trash_paths(&db).unwrap(),2);assert_eq!(repair_committed_trash_paths(&db).unwrap(),0);
        for name in ["failed","ordinary-missing","restored","reobserved"]{assert_eq!(db.query_row("SELECT is_current FROM image_paths WHERE path=?1",[path_to_string(&root.join(name))],|r|r.get::<_,i64>(0)).unwrap(),1);}
        assert_eq!(fs::read(root.join("restored")).unwrap(),b"restored owned bytes");fs::remove_file(root.join("restored")).unwrap();fs::remove_dir(root).unwrap();
    }
}
