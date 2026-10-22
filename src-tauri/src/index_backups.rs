//! Consistent archive snapshots and deferred restoration before any worker starts.
use crate::*;
use serde_json::{json, Value};
use std::sync::Mutex;

#[derive(Default)]
pub struct BackupRuntime {
    active: Mutex<Option<PathBuf>>,
    gate: Mutex<()>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
struct Selection { file_name: String, #[serde(default)] sha256: Option<String>, #[serde(default)] editor_directory:Option<String>, #[serde(default)] editor_manifest_sha256:Option<String> }
const ACTIVE: &str = "active-index.json";
const PENDING: &str = "pending-index-restore.json";
const DEFAULT: &str = "smart_photo_indexer.sqlite";
const MAX_BACKUP_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn selected_path(directory: &Path, selection: &Selection) -> Result<PathBuf,String> {
    let name=&selection.file_name;
    if name!=DEFAULT && !(name.starts_with("restored-") && name.ends_with(".sqlite")
        && name[9..name.len()-7].chars().all(|c|c.is_ascii_digit()) && name.len()>16 && name.len()<80) {
        return Err("Некорректное имя базы восстановления.".into());
    }
    let path=directory.join(name);
    let canonical=fs::canonicalize(&path).map_err(|e|format!("База архива недоступна: {e}"))?;
    if canonical.parent()!=Some(fs::canonicalize(directory).map_err(|e|e.to_string())?.as_path())
        || fs::symlink_metadata(&path).map_err(|e|e.to_string())?.file_type().is_symlink() {
        return Err("База восстановления должна находиться непосредственно в папке приложения.".into());
    }
    Ok(path)
}
fn read_selection(path:&Path)->Result<Selection,String>{
    if fs::metadata(path).map_err(|e|e.to_string())?.len()>4096 {return Err("Слишком большой файл выбора базы.".into());}
    serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
}
pub fn database_path(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let runtime=app.state::<BackupRuntime>();
    let mut active=runtime.active.lock().map_err(|_|"Блокировка базы недоступна.")?;
    if let Some(path)=active.as_ref(){return Ok(path.clone());}
    let directory=application_data_directory(app)?;
    let pointer=directory.join(ACTIVE);
    let path=if pointer.exists(){selected_path(&directory,&read_selection(&pointer)?)?}else{directory.join(DEFAULT)};
    *active=Some(path.clone()); Ok(path)
}
pub(crate) fn open_snapshot(path:&Path)->Result<Connection,String>{
    let db=Connection::open_with_flags(path,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e|e.to_string())?;
    db.create_scalar_function("unicode_lower",1,rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx|Ok(ctx.get::<String>(0)?.to_lowercase())).map_err(|e|e.to_string())?;
    Ok(db)
}
fn schema(db:&Connection)->Result<Vec<(String,String,Option<String>)>,String>{
    let mut stmt=db.prepare("SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    rows.collect::<Result<_,_>>().map_err(|e|e.to_string())
}
pub(crate) fn validate_snapshot(path:&Path)->Result<Value,String>{
    let stat=fs::metadata(path).map_err(|e|e.to_string())?;
    if !stat.is_file() || stat.len()==0 || stat.len()>MAX_BACKUP_BYTES {return Err("Резервная копия должна быть файлом SQLite размером до 2 ГБ.".into());}
    let db=open_snapshot(path)?;
    let version=current_schema_version(&db)?;
    if !(15..=INDEX_SCHEMA_VERSION).contains(&version) {return Err(format!("Поддерживается копия схемы 15–{INDEX_SCHEMA_VERSION}; исходная база не изменена."));}
    let reference=Connection::open_in_memory().map_err(|e|e.to_string())?;
    reference.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at_unix INTEGER NOT NULL
            );
        "#).map_err(|e|e.to_string())?;
    for migration in INDEX_MIGRATIONS.iter().filter(|m|m.version<=version){reference.execute_batch(migration.sql).map_err(|e|e.to_string())?;}
    if schema(&db)?!=schema(&reference)? {return Err("Структура копии отличается от структуры приложения. Восстановление отклонено.".into());}
    let versions:Vec<(i64,String)>={let mut stmt=db.prepare("SELECT version,name FROM schema_migrations ORDER BY version").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?};
    let expected:Vec<(i64,String)>=INDEX_MIGRATIONS.iter().filter(|m|m.version<=version).map(|m|(m.version,m.name.into())).collect();
    if versions!=expected{return Err("Неверный журнал миграций резервной копии.".into());}
    let integrity:String=db.query_row("PRAGMA integrity_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if integrity!="ok"{return Err(format!("Нарушена целостность копии: {integrity}"));}
    let foreign_keys:bool=db.prepare("PRAGMA foreign_key_check").map_err(|e|e.to_string())?.exists([]).map_err(|e|e.to_string())?;
    if foreign_keys{return Err("Нарушены связи в резервной копии.".into());}
    if version>=16 {
        let mut statement=db.prepare("SELECT COALESCE(description,''),description_format FROM images WHERE description_format IS NOT NULL").map_err(|e|e.to_string())?;
        let rows=statement.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;
        for row in rows{let (text,raw)=row.map_err(|e|e.to_string())?;let value=serde_json::from_str(&raw).map_err(|_|"Повреждено оформление описания в резервной копии.".to_string())?;description_format::validate(Some(value),&text)?;}
    }
    let images:i64=db.query_row("SELECT COUNT(*) FROM images",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let paths:i64=db.query_row("SELECT COUNT(*) FROM image_paths",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok(json!({"imageCount":images,"pathCount":paths,"schemaVersion":version,"sizeBytes":stat.len(),"requiresConversion":version<INDEX_SCHEMA_VERSION}))
}
pub(crate) fn snapshot(db:&Connection,target:&Path,scrub_credentials:bool)->Result<Value,String>{
    file_safety::atomic_save(target,|_,temporary|{
        // atomic_save owns an empty file; VACUUM INTO accepts an existing empty file.
        db.execute("VACUUM INTO ?1",[path_to_string(temporary)]).map_err(|e|e.to_string())?;
        if scrub_credentials {
            let mut copy=Connection::open(temporary).map_err(|e|e.to_string())?;
            copy.create_scalar_function("unicode_lower",1,rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,|ctx|Ok(ctx.get::<String>(0)?.to_lowercase())).map_err(|e|e.to_string())?;
            run_index_migrations(&mut copy)?;
            copy.execute_batch("PRAGMA secure_delete=ON; DELETE FROM provider_credentials; VACUUM;").map_err(|e|e.to_string())?;
        }
        validate_snapshot(temporary)?;
        Ok(())
    })?;
    validate_snapshot(target)
}
pub(crate) fn require_settled_current_cloud(db:&Connection)->Result<(),String>{
    let unsettled:i64=db.query_row("SELECT COUNT(*) FROM processing_attempt_context a WHERE json_extract(a.effective_config,'$.providerMode') IN ('fal','openrouter') AND a.status IN ('reserved','running','remotePending','unknown','responseSaved')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if unsettled>0{return Err("Перед восстановлением дождитесь ответов и сверьте незавершённые облачные запросы в текущей очереди. Их расход нельзя потерять при замене индекса.".into());}Ok(())
}
fn replace_pointer(path:&Path,selection:&Selection)->Result<(),String>{
    let pending=path.with_extension(format!("{}.partial",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&pending).map_err(|e|e.to_string())?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec(selection).map_err(|e|e.to_string())?).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;drop(file);
    #[cfg(windows)] {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW,MOVEFILE_REPLACE_EXISTING,MOVEFILE_WRITE_THROUGH};
        let from:Vec<u16>=pending.as_os_str().encode_wide().chain(Some(0)).collect();let to:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe{MoveFileExW(from.as_ptr(),to.as_ptr(),MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)}==0 {
            let error=std::io::Error::last_os_error();let _=fs::remove_file(&pending);return Err(error.to_string());
        }
    }
    #[cfg(not(windows))] fs::rename(&pending,path).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn apply_pending_before_workers(app:&tauri::AppHandle)->bool{
    let result=(||->Result<(),String>{
        let directory=application_data_directory(app)?;fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
        let pending=directory.join(PENDING);if !pending.exists(){return Ok(());}
        let selection=read_selection(&pending)?;if let Some(folder)=&selection.editor_directory{crate::photo_editor::backup::validate_prepared(app,folder,selection.editor_manifest_sha256.as_deref())?;}let candidate=selected_path(&directory,&selection)?;
        let active_path=database_path(app)?;crate::photo_editor::jobs::require_settled(app)?;
        // A crash after publishing ACTIVE requires only finishing the journal, never restoring twice.
        if active_path!=candidate {
            if selection.sha256.as_deref()!=Some(compute_file_sha256(&candidate)?.as_str()){return Err("Копия изменилась после подготовки. Прежняя база сохранена.".into());}
            validate_snapshot(&candidate)?;
            if active_path.exists(){
                let backups=directory.join("backups");fs::create_dir_all(&backups).map_err(|e|e.to_string())?;
                let before=backups.join(format!("before-restore-{}.sqlite",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
                let db=open_snapshot(&active_path)?;require_settled_current_cloud(&db)?;snapshot(&db,&before,false)?;
            }
            replace_pointer(&directory.join(ACTIVE),&Selection{file_name:selection.file_name,sha256:None,editor_directory:selection.editor_directory,editor_manifest_sha256:None})?;
            *app.state::<BackupRuntime>().active.lock().map_err(|_|"Блокировка базы недоступна.")?=Some(candidate);
        }
        fs::remove_file(&pending).map_err(|e|e.to_string())?;
        let _=fs::remove_file(directory.join("index-restore-error.txt")); Ok(())
    })();
    let ready=result.is_ok();
    if let Err(error)=result {
        if let Ok(directory)=application_data_directory(app){let _=fs::write(directory.join("index-restore-error.txt"),error.as_bytes());}
        eprintln!("Archive restoration postponed; previous database remains available.");
    }ready
}
#[tauri::command]
pub async fn choose_index_backup_path(restore:bool,include_full:Option<bool>, ui_locale:Option<String>)->Result<Option<String>,String>{
    let full=restore&&include_full.unwrap_or(false);
    let dialog=if full{rfd::AsyncFileDialog::new().add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.5d6a7db7a60704f8"),&["json","sqlite"])}else{rfd::AsyncFileDialog::new().add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.8fbb3366ae7a3e8c"),&["sqlite"])};
    let chosen=if restore{dialog.set_title(if full{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.fbd457a64a6b58b6")}else{crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.0c8798fd110f971d")}).pick_file().await}
        else{dialog.set_title(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.793b09aca69ff93f")).set_file_name("smart-photo-backup.sqlite").save_file().await};
    Ok(chosen.map(|f|path_to_string(f.path())))
}
#[tauri::command]
pub async fn export_index_backup(app:tauri::AppHandle,target_path:String)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<BackupRuntime>();let _gate=runtime.gate.lock().map_err(|_|"Блокировка резервного копирования недоступна.")?;
        let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;
        let mut info=snapshot(&db,Path::new(&target_path),true)?;info["path"]=json!(target_path);info["credentialsIncluded"]=json!(false);Ok(info)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn inspect_index_backup(source_path:String)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||validate_snapshot(Path::new(&source_path))).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn stage_index_restore(app:tauri::AppHandle,source_path:String)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||processing::cloud_fence(&app,||stage_restore_locked(&app,&source_path,None,None))).await.map_err(|e|e.to_string())?}
pub(crate) fn stage_restore_locked(app:&tauri::AppHandle,source_path:&str,editor_directory:Option<String>,editor_manifest_sha256:Option<String>)->Result<Value,String>{
        let runtime=app.state::<BackupRuntime>();let _gate=runtime.gate.lock().map_err(|_|"Блокировка восстановления недоступна.")?;
        let directory=application_data_directory(app)?;let pending=directory.join(PENDING);
        if pending.exists(){return Err("Восстановление уже подготовлено. Сначала отмените его или перезапустите приложение.".into());}
        let source=fs::canonicalize(source_path).map_err(|e|e.to_string())?;validate_snapshot(&source)?;
        let current=open_index_connection(app)?;require_settled_current_cloud(&current)?;crate::photo_editor::jobs::require_settled(app)?;let editor=crate::photo_editor::store::connection(app)?;let unfinished:bool=editor.query_row("SELECT EXISTS(SELECT 1 FROM editor_publications WHERE state NOT IN ('completed','failed','cancelled'))",[],|r|r.get(0)).map_err(|e|e.to_string())?;if unfinished{return Err("Сначала завершите сохранение копий редактора в архив.".into());}drop(editor);drop(current);
        let file_name=format!("restored-{}.sqlite",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos());
        let target=directory.join(&file_name);let source_db=open_snapshot(&source)?;
        let mut info=snapshot(&source_db,&target,true)?;
        let copy=Connection::open(&target).map_err(|e|e.to_string())?;
        interrupt_restored_work(&copy)?;drop(copy);
        let sha256=compute_file_sha256(&target)?;replace_pointer(&pending,&Selection{file_name,sha256:Some(sha256),editor_directory:editor_directory.or(editor_folder_name(app)?),editor_manifest_sha256})?;
        info["pending"]=json!(true);info["requiresRestart"]=json!(true);info["credentialsIncluded"]=json!(false);Ok(info)
}

// Only a private restored copy is passed here; never the live working database.
fn interrupt_restored_work(copy:&Connection)->Result<(),String>{
    copy.execute_batch("BEGIN IMMEDIATE;
            UPDATE processing_jobs SET status='interrupted',generation=generation+1,error='Восстановлено из резервной копии; требуется явное продолжение' WHERE status IN ('prepared','queued','running','paused','interrupted');
            UPDATE processing_jobs SET status='stopped',generation=generation+1,error='Восстановленная облачная очередь доступна только как история. Для нового распознавания создайте новое задание, чтобы не повторить оплату старых проходов.' WHERE id IN (SELECT job_id FROM processing_task_context WHERE json_extract(effective_config,'$.providerMode') IN ('fal','openrouter') UNION SELECT job_id FROM processing_attempt_context WHERE json_extract(effective_config,'$.providerMode') IN ('fal','openrouter'));
            UPDATE processing_tasks SET status='cancelled',run_revision=run_revision+1 WHERE status NOT IN ('succeeded','failed','cancelled') AND id IN (SELECT id FROM processing_task_context WHERE json_extract(effective_config,'$.providerMode') IN ('fal','openrouter'));
            UPDATE processing_attempts SET status='restoredHistory' WHERE status IN ('reserved','running','remotePending','unknown','responseSaved') AND id IN (SELECT id FROM processing_attempt_context WHERE json_extract(effective_config,'$.providerMode') IN ('fal','openrouter'));
            UPDATE file_operations SET status='failed',error='План отменён при восстановлении индекса' WHERE status IN ('planned','applying','needsRecovery');
            UPDATE processing_tasks SET rename_status='failed',error='План переименования отменён при восстановлении индекса' WHERE rename_status IN ('pending','applying');
            UPDATE action_logs SET action_type='batch_tags_preview_cancelled_restore' WHERE action_type='batch_tags_preview';
            DELETE FROM file_observations;
            DELETE FROM duplicate_plans;
            DELETE FROM provider_readiness;
            DELETE FROM processing_sources;
            UPDATE processing_jobs SET status='preparationFailed',error='Preparation invalidated by index restore' WHERE status='preparing';
            COMMIT;").map_err(|e|e.to_string())
}

#[tauri::command]
pub async fn get_index_restore_status(app:tauri::AppHandle)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let directory=application_data_directory(&app)?;
        let error=fs::read_to_string(directory.join("index-restore-error.txt")).ok();
        Ok(json!({"pending":directory.join(PENDING).exists(),"error":error}))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn cancel_index_restore(app:tauri::AppHandle)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<BackupRuntime>();let _gate=runtime.gate.lock().map_err(|_|"Блокировка восстановления недоступна.")?;
        let directory=application_data_directory(&app)?;let pending=directory.join(PENDING);
        if pending.exists(){fs::remove_file(pending).map_err(|e|e.to_string())?;}
        let _=fs::remove_file(directory.join("index-restore-error.txt"));Ok(())
    }).await.map_err(|e|e.to_string())?
}

pub(crate) fn restore_blocked(app:&tauri::AppHandle)->Result<bool,String>{Ok(application_data_directory(app)?.join(PENDING).exists())}
pub(crate) fn editor_folder_name(app:&tauri::AppHandle)->Result<Option<String>,String>{let pointer=application_data_directory(app)?.join(ACTIVE);let name=if pointer.exists(){read_selection(&pointer)?.editor_directory}else{None};if let Some(name)=&name{if !name.starts_with("photo-editor-restored-")||!name[22..].bytes().all(|b|b.is_ascii_digit())||name.len()<=22||name.len()>80{return Err("Некорректный каталог восстановленного редактора.".into());}}Ok(name)}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(version: i64) -> PathBuf {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("smart-photo-backup-schema-{}-{nonce}.sqlite", std::process::id()));
        let db = Connection::open(&path).unwrap();
        db.create_scalar_function("unicode_lower", 1, rusqlite::functions::FunctionFlags::SQLITE_UTF8 | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC, sqlite_unicode_lower).unwrap();
        db.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at_unix INTEGER NOT NULL
            );
        "#).unwrap();
        for migration in INDEX_MIGRATIONS.iter().filter(|migration| migration.version <= version) {
            db.execute_batch(migration.sql).unwrap();
            db.execute("INSERT INTO schema_migrations VALUES(?1,?2,1)", params![migration.version,migration.name]).unwrap();
        }
        db.execute("INSERT INTO images(id,identity_key,file_name,description,created_at_unix,updated_at_unix) VALUES('kept-image','kept-image','Кошки.png','Сохранённое описание',1,1)", []).unwrap();
        path
    }

    #[test]
    fn all_supported_archive_schemas_are_inspected_without_modifying_the_source() {
        for version in 15..=INDEX_SCHEMA_VERSION {
            let path = archive(version);
            let before = compute_file_sha256(&path).unwrap();
            let result = validate_snapshot(&path).unwrap_or_else(|error| panic!("schema {version}: {error}"));
            assert_eq!(result["schemaVersion"], version);
            assert_eq!(result["imageCount"], 1);
            assert_eq!(result["requiresConversion"], version < INDEX_SCHEMA_VERSION);
            assert_eq!(compute_file_sha256(&path).unwrap(), before);
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn accepting_intermediate_schemas_keeps_structure_and_version_guards() {
        let path = archive(21);
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch("CREATE TABLE foreign_data(value TEXT);").unwrap();
        }
        let before = compute_file_sha256(&path).unwrap();
        assert!(validate_snapshot(&path).unwrap_err().contains("Структура"));
        assert_eq!(compute_file_sha256(&path).unwrap(), before);
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch("DROP TABLE foreign_data;").unwrap();
            db.execute("UPDATE schema_migrations SET version=?1 WHERE version=21", [INDEX_SCHEMA_VERSION+1]).unwrap();
        }
        let before = compute_file_sha256(&path).unwrap();
        assert!(validate_snapshot(&path).unwrap_err().contains("Поддерживается"));
        assert_eq!(compute_file_sha256(&path).unwrap(), before);
        fs::remove_file(path).unwrap();
    }
    #[test]fn restored_cloud_history_keeps_money_but_does_not_own_live_capacity(){
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
        db.execute("INSERT INTO images(id,identity_key,file_name,created_at_unix,updated_at_unix) VALUES('i','owned','owned.png',1,1)",[]).unwrap();
        db.execute("INSERT INTO processing_jobs(id,request_key,config,status,created_at_unix,updated_at_unix) VALUES('j','owned','{\"providerMode\":\"local\"}','paused',1,1)",[]).unwrap();
        db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','owned','sha','{}','pending')",[]).unwrap();
        db.execute("INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros,response,remote_id,execution_context) VALUES('cloud','t',1,0,'unknown',1,37,'{\"saved\":true}','known-remote','{\"config\":{\"providerMode\":\"fal\"},\"credentialRef\":\"old\"}')",[]).unwrap();
        assert!(require_settled_current_cloud(&db).is_err());interrupt_restored_work(&db).unwrap();assert!(require_settled_current_cloud(&db).is_ok());
        let(status,reserve,response,remote):(String,i64,String,String)=db.query_row("SELECT status,reserved_micros,response,remote_id FROM processing_attempts",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();assert_eq!(status,"restoredHistory");assert_eq!(reserve,37);assert_eq!(response,"{\"saved\":true}");assert_eq!(remote,"known-remote");assert_eq!(db.query_row("SELECT status FROM processing_jobs",[],|r|r.get::<_,String>(0)).unwrap(),"stopped");
    }

}
