use super::contracts::Draft;
use crate::*;
use serde_json::{json,Value};

pub fn root(app:&tauri::AppHandle)->Result<PathBuf,String>{
    let profile=application_data_directory(app)?;fs::create_dir_all(&profile).map_err(|e|e.to_string())?;
    let profile=profile.canonicalize().map_err(|e|e.to_string())?;
    let custom=super::storage::custom_root(app)?;let external=custom.is_some();let root=custom.unwrap_or_else(||profile.join(index_backups::editor_folder_name(app).ok().flatten().unwrap_or_else(||"photo-editor".into())));fs::create_dir_all(&root).map_err(|e|e.to_string())?;
    let root=root.canonicalize().map_err(|e|e.to_string())?;
    if !external&&root.parent()!=Some(profile.as_path()){return Err("Хранилище редактора должно находиться внутри профиля приложения.".into());}
    for folder in ["assets","masks","previews"]{let child=root.join(folder);fs::create_dir_all(&child).map_err(|e|e.to_string())?;if child.canonicalize().map_err(|e|e.to_string())?.parent()!=Some(root.as_path()){return Err("Каталог кадров перенаправлен за пределы хранилища редактора.".into());}}
    Ok(root)
}
pub fn owned_path(app:&tauri::AppHandle,relative:&str)->Result<PathBuf,String>{
    let p=Path::new(relative);
    if p.components().any(|c|!matches!(c,Component::Normal(_)))||p.components().count()!=2||!["assets","masks","previews"].contains(&p.components().next().unwrap().as_os_str().to_string_lossy().as_ref()){
        return Err("Путь не принадлежит хранилищу редактора.".into());
    }
    let base=root(app)?;let path=base.join(p);
    let parent=path.parent().ok_or("Неверный owned path")?.canonicalize().map_err(|e|e.to_string())?;
    if !parent.starts_with(&base){return Err("Каталог редактора перенаправлен за пределы профиля.".into());}
    if fs::symlink_metadata(&path).is_ok(){let resolved=path.canonicalize().map_err(|e|e.to_string())?;if !resolved.starts_with(&base){return Err("Файл редактора перенаправлен за пределы профиля.".into());}}
    Ok(path)
}
pub fn connection(app:&tauri::AppHandle)->Result<Connection,String>{
    if super::storage::MIGRATING.load(std::sync::atomic::Ordering::Acquire){return Err("Хранилище проектов переносится. Дождитесь завершения.".into());}
    // First-use recovery is serialized and finishes before any editor writer.
    // Subsequent connections must not delete an active writer's partial file.
    static INITIALIZED:std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>>=std::sync::OnceLock::new();
    let directory=root(app)?;
    let mut initialized=INITIALIZED.get_or_init(Default::default).lock().map_err(|_|"Editor initialization lock failed.")?;
    let file=directory.join("editor.sqlite");
    if file.exists()&&file.canonicalize().map_err(|e|e.to_string())?.parent()!=Some(directory.as_path()){return Err("База редактора перенаправлена за пределы хранилища.".into());}
    let mut db=Connection::open(file).map_err(|e|e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e|e.to_string())?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;").map_err(|e|e.to_string())?;
    if !initialized.contains(&directory){db.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e|e.to_string())?;}
    let version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    if version>3{return Err("Этот проект сохранён более новой версией редактора. Архив фотографий не изменён.".into());}
    if version==0{
        let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        tx.execute_batch(SCHEMA).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;
    }
    if !initialized.contains(&directory){
        db.execute_batch(EXTRA_SCHEMA).map_err(|e|e.to_string())?;
        db.execute_batch(super::portable_history::SCHEMA).map_err(|e|e.to_string())?;
        let has_response_meta:bool={let mut q=db.prepare("PRAGMA table_info(editor_attempts)").map_err(|e|e.to_string())?;let names=q.query_map([],|r|r.get::<_,String>(1)).map_err(|e|e.to_string())?;names.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?.iter().any(|name|name=="response_meta")};
        if !has_response_meta{db.execute_batch("ALTER TABLE editor_attempts ADD COLUMN response_meta TEXT;").map_err(|e|e.to_string())?;}
        db.execute_batch("CREATE TABLE IF NOT EXISTS editor_pricing_cache(credential_ref TEXT NOT NULL,route TEXT NOT NULL,value TEXT NOT NULL,checked_at INTEGER NOT NULL,PRIMARY KEY(credential_ref,route)); CREATE INDEX IF NOT EXISTS editor_attempts_job ON editor_attempts(job_id,created_at_unix);").map_err(|e|e.to_string())?;
        let integrity:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if integrity!="ok"{return Err("Хранилище редактора повреждено. Исходные фотографии не изменены.".into());}
        let foreign_key_errors:i64=db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if foreign_key_errors!=0{return Err("В хранилище редактора нарушены связи. Автоматическая перезапись отключена.".into());}
        if version==1 {
            let snapshot=directory.join("editor.schema-v1.before-migration.sqlite");
            if !snapshot.exists(){db.execute("VACUUM INTO ?1",[path_to_string(&snapshot)]).map_err(|e|e.to_string())?;}
            let saved=Connection::open_with_flags(&snapshot,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
            let integrity:String=saved.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
            if integrity!="ok"{return Err("Предмиграционная копия базы не прошла проверку".into());}
        }
        if version==2&&super::migration::publication_keys_needed(&db)?{let snapshot=directory.join("editor.schema-v2.before-target-keys.sqlite");if !snapshot.exists(){db.execute("VACUUM INTO ?1",[path_to_string(&snapshot)]).map_err(|e|e.to_string())?;}}
        if version==2{let before=directory.join("editor.schema-v2.before-checkpoints.sqlite");if !before.exists(){db.execute("VACUUM INTO ?1",[path_to_string(&before)]).map_err(|e|e.to_string())?;}}
        if version==3&&super::migration::replacement_reservations_needed(&db)?{let snapshot=directory.join("editor.schema-v3.before-replacement-reservations.sqlite");if !snapshot.exists(){db.execute("VACUUM INTO ?1",[path_to_string(&snapshot)]).map_err(|e|e.to_string())?;}}
        super::migration::apply(&mut db)?;
        super::lifecycle::migrate(&mut db)?;
        super::migration::upgrade_replacement_reservations(&mut db)?;
        recover_files(app,&db)?;
        initialized.insert(directory);
    }
    Ok(db)
}
fn recover_files(app:&tauri::AppHandle,db:&Connection)->Result<(),String>{
    let directory=root(app)?;
    for folder in ["assets","masks","previews"]{
        let parent=directory.join(folder).canonicalize().map_err(|e|e.to_string())?;
        if !parent.starts_with(&directory){return Err("Owned-каталог редактора перенаправлен.".into());}
        for entry in fs::read_dir(&parent).map_err(|e|e.to_string())?{
            let entry=entry.map_err(|e|e.to_string())?;let name=entry.file_name().to_string_lossy().to_string();
            if name.starts_with(".smart-photo-")&&name.ends_with(".partial")&&entry.file_type().map_err(|e|e.to_string())?.is_file(){fs::remove_file(entry.path()).map_err(|e|e.to_string())?;}
        }
    }
    let operations={let mut q=db.prepare("SELECT id,relative_path,metadata FROM editor_file_operations WHERE state IN ('planned','fileSaved')").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    for (id,relative,raw) in operations{
        let metadata:Value=serde_json::from_str(&raw).map_err(|_|"Журнал файлов редактора повреждён.".to_string())?;
        let path=owned_path(app,&relative)?;
        let expected=metadata["sha256"].as_str();
        if path.is_file()&&expected.is_some()&&compute_file_sha256(&path)?==expected.unwrap(){db.execute("UPDATE editor_file_operations SET state='fileSaved',error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;}
        // A verified file alone is not a ready result: asset/step registration
        // still requires the normal idempotent operation commit.
    }
    Ok(())
}
pub(super) const EXTRA_SCHEMA:&str="CREATE TABLE IF NOT EXISTS editor_step_drafts(step_id TEXT PRIMARY KEY REFERENCES editor_steps(id),draft TEXT NOT NULL); CREATE TABLE IF NOT EXISTS editor_project_tombstones(project_id TEXT PRIMARY KEY REFERENCES editor_projects(id),deleted_at_unix INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS editor_step_visibility(step_id TEXT PRIMARY KEY REFERENCES editor_steps(id),hidden INTEGER NOT NULL);";
pub(super) const SCHEMA:&str=r#"
 CREATE TABLE IF NOT EXISTS editor_assets(id TEXT PRIMARY KEY,kind TEXT NOT NULL,relative_path TEXT NOT NULL UNIQUE,preview_path TEXT NOT NULL,sha256 TEXT NOT NULL,width INTEGER NOT NULL,height INTEGER NOT NULL,bytes INTEGER NOT NULL,content_type TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS editor_projects(id TEXT PRIMARY KEY,title TEXT NOT NULL,source_name TEXT NOT NULL,source_path TEXT NOT NULL,source_sha256 TEXT NOT NULL,active_step_id TEXT,revision INTEGER NOT NULL DEFAULT 0,draft TEXT NOT NULL,created_at_unix INTEGER NOT NULL,updated_at_unix INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS editor_steps(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES editor_projects(id),asset_id TEXT NOT NULL REFERENCES editor_assets(id),operation TEXT NOT NULL,title TEXT NOT NULL,parameters TEXT NOT NULL DEFAULT '{}',annotations TEXT NOT NULL DEFAULT '[]',job_id TEXT,created_at_unix INTEGER NOT NULL);
 CREATE INDEX IF NOT EXISTS editor_steps_project ON editor_steps(project_id,created_at_unix);
 CREATE TABLE IF NOT EXISTS editor_edges(step_id TEXT NOT NULL REFERENCES editor_steps(id),parent_id TEXT NOT NULL REFERENCES editor_steps(id),PRIMARY KEY(step_id,parent_id));
 CREATE TABLE IF NOT EXISTS editor_project_assets(project_id TEXT NOT NULL REFERENCES editor_projects(id),asset_id TEXT NOT NULL REFERENCES editor_assets(id),role TEXT NOT NULL,PRIMARY KEY(project_id,asset_id,role));
 CREATE TABLE IF NOT EXISTS editor_file_operations(id TEXT PRIMARY KEY,relative_path TEXT NOT NULL,state TEXT NOT NULL,metadata TEXT NOT NULL,created_at_unix INTEGER NOT NULL,error TEXT);
 CREATE TABLE IF NOT EXISTS editor_jobs(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES editor_projects(id),state TEXT NOT NULL,operation TEXT NOT NULL,model_id TEXT NOT NULL,source_asset_id TEXT NOT NULL REFERENCES editor_assets(id),snapshot TEXT NOT NULL,request_id TEXT,status_url TEXT,response_url TEXT,cancel_url TEXT,credential_ref TEXT,reserved_micros INTEGER NOT NULL DEFAULT 0,spent_micros INTEGER,error TEXT,result_asset_ids TEXT NOT NULL DEFAULT '[]',created_at_unix INTEGER NOT NULL,updated_at_unix INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS editor_attempts(id TEXT PRIMARY KEY,job_id TEXT NOT NULL REFERENCES editor_jobs(id),status TEXT NOT NULL,reserved_micros INTEGER NOT NULL,cost_micros INTEGER,request_id TEXT,usage TEXT,response_meta TEXT,created_at_unix INTEGER NOT NULL);
 CREATE INDEX IF NOT EXISTS editor_attempts_job ON editor_attempts(job_id,created_at_unix);
 CREATE TABLE IF NOT EXISTS editor_publications(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES editor_projects(id),asset_id TEXT NOT NULL REFERENCES editor_assets(id),state TEXT NOT NULL,target_path TEXT NOT NULL UNIQUE,metadata TEXT NOT NULL,created_at_unix INTEGER NOT NULL,error TEXT);
 CREATE TABLE IF NOT EXISTS editor_catalog_versions(id TEXT PRIMARY KEY,catalog TEXT NOT NULL,created_at_unix INTEGER NOT NULL,active INTEGER NOT NULL DEFAULT 0);
 CREATE TABLE IF NOT EXISTS editor_notes(id TEXT PRIMARY KEY,project_id TEXT REFERENCES editor_projects(id),model_id TEXT,note TEXT NOT NULL,created_at_unix INTEGER NOT NULL);
 PRAGMA user_version=1;
"#;

pub fn asset_value(db:&Connection,id:&str)->Result<Value,String>{
    db.query_row("SELECT id,kind,relative_path,preview_path,sha256,width,height,bytes,content_type FROM editor_assets WHERE id=?1",[id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"relativePath":r.get::<_,String>(2)?,"previewPath":r.get::<_,String>(3)?,"sha256":r.get::<_,String>(4)?,"width":r.get::<_,u32>(5)?,"height":r.get::<_,u32>(6)?,"bytes":r.get::<_,u64>(7)?,"contentType":r.get::<_,String>(8)?}))).map_err(|_|"Кадр редактора недоступен.".into())
}
pub fn project_snapshot(app:&tauri::AppHandle,project_id:&str)->Result<Value,String>{
    let mut db=connection(app)?;
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let deleted:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_tombstones WHERE project_id=?1)",[project_id],|r|r.get(0)).map_err(|e|e.to_string())?;if deleted{return Err("Проект удалён. Исходные фотографии в архиве сохранены.".into());}
    super::migration::root(&tx,project_id)?;
    let lifecycle=super::lifecycle::observe(&tx,project_id)?;
    let mut snapshot=super::lifecycle::capture(&tx,project_id)?;snapshot["project"]["lifecycle"]=lifecycle;
    tx.commit().map_err(|e|e.to_string())?;
    for asset in snapshot["assets"].as_array_mut().ok_or("Нет ресурсов проекта")?{let preview=owned_path(app,asset["previewPath"].as_str().ok_or("Неверный preview")?)?;if !preview.is_file(){return Err("Превью редактора отсутствует. Сохранённые кадры не изменены.".into());}app.asset_protocol_scope().allow_file(&preview).map_err(|e|e.to_string())?;asset["previewPath"]=json!(path_to_string(&preview));}
    Ok(snapshot)
}

pub fn open_source(app:&tauri::AppHandle,source:&Path)->Result<String,String>{
    let source_sha=compute_file_sha256(source)?;
    let own_publication:Option<String>=connection(app)?.query_row("SELECT p.project_id FROM editor_publications p JOIN editor_project_lifecycle l ON l.project_id=p.project_id WHERE p.state='completed' AND json_extract(p.metadata,'$.mode')='replace' AND json_extract(p.metadata,'$.targetKey')=?1 AND json_extract(p.metadata,'$.sha256')=?2 AND p.project_id NOT IN(SELECT project_id FROM editor_project_tombstones) ORDER BY p.created_at_unix DESC LIMIT 1",params![super::migration::publication_target_key(&path_to_string(source)),source_sha],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if let Some(id)=own_publication{project_snapshot(app,&id)?;return Ok(id);}
    let base_id=stable_text_id("editor-project",&format!("{}|{source_sha}",path_to_string(source)));
    let existing:Option<String>=connection(app)?.query_row("SELECT id FROM editor_projects WHERE source_path=?1 AND source_sha256=?2 AND id NOT IN (SELECT project_id FROM editor_project_tombstones) ORDER BY CASE WHEN id=?3 OR id LIKE ?3||'-%' THEN 0 ELSE 1 END,created_at_unix ASC LIMIT 1",params![path_to_string(source),source_sha,base_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;if let Some(id)=existing{project_snapshot(app,&id)?;return Ok(id);}
    let deleted:bool=connection(app)?.query_row("SELECT EXISTS(SELECT 1 FROM editor_projects WHERE id=?1)",[&base_id],|r|r.get(0)).map_err(|e|e.to_string())?;let project_id=if deleted{format!("{base_id}-{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos())}else{base_id};
    let db=connection(app)?;
    if db.query_row("SELECT 1 FROM editor_projects WHERE id=?1",[&project_id],|r|r.get::<_,i32>(0)).optional().map_err(|e|e.to_string())?.is_some(){project_snapshot(app,&project_id)?;return Ok(project_id);}
    let metadata=fs::metadata(source).map_err(|e|e.to_string())?;
    if metadata.len()>256*1024*1024{return Err("Исходник превышает предел копирования 256 МиБ. Оригинал не изменён.".into());}
    let extension=extension(source).unwrap_or_else(||"png".into());
    let asset_id=format!("editor-asset-{source_sha}");let relative=format!("assets/{source_sha}.{extension}");let preview_relative=format!("previews/{source_sha}.png");
    let destination=owned_path(app,&relative)?;let preview_path=owned_path(app,&preview_relative)?;
    let op=format!("source-{source_sha}");
    db.execute("INSERT OR IGNORE INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES(?1,?2,'planned',?3,?4)",params![op,relative,json!({"sha256":source_sha,"preview":preview_relative}).to_string(),unix_now()]).map_err(|e|e.to_string())?;
    if !destination.exists(){file_safety::atomic_save(&destination,|out,_|{let mut input=fs::File::open(source).map_err(|e|e.to_string())?;std::io::copy(&mut input,out).map_err(|e|e.to_string())?;Ok(())})?;}
    if compute_file_sha256(&destination)?!=source_sha||compute_file_sha256(source)?!=source_sha{return Err("Исходник изменился при копировании. Редактор не создал проект.".into());}
    // Caller holds the shared single image-worker permit. Bounds precede allocation.
    let dimensions=image::ImageReader::open(&destination).map_err(|e|e.to_string())?.with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())?;
    if u64::from(dimensions.0)*u64::from(dimensions.1)*8+16*1024*1024>384*1024*1024{return Err("Подготовка кадра превышает бюджет памяти редактора.".into());}
    let image=file_safety::decode_bounded(&destination)?;let (width,height)=(image.width(),image.height());
    if !preview_path.exists(){let preview=image.resize(1600,1250,image::imageops::FilterType::Triangle);file_safety::atomic_save(&preview_path,|out,_|preview.write_to(out,image::ImageFormat::Png).map_err(|e|e.to_string()))?;}
    drop(image);
    let mut db=connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO editor_assets(id,kind,relative_path,preview_path,sha256,width,height,bytes,content_type) VALUES(?1,'image',?2,?3,?4,?5,?6,?7,?8)",params![asset_id,relative,preview_relative,source_sha,width,height,metadata.len(),image_mime_type(source)]).map_err(|e|e.to_string())?;
    let now=unix_now();let draft=Draft::initial(&asset_id);let step_id=format!("{project_id}-source");
    tx.execute("INSERT OR IGNORE INTO editor_projects(id,title,source_name,source_path,source_sha256,active_step_id,draft,created_at_unix,updated_at_unix) VALUES(?1,?2,?2,?3,?4,?5,?6,?7,?7)",params![project_id,file_name(source),path_to_string(source),source_sha,step_id,serde_json::to_string(&draft).map_err(|e|e.to_string())?,now]).map_err(|e|e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'source')",params![project_id,asset_id]).map_err(|e|e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO editor_steps(id,project_id,asset_id,operation,title,created_at_unix) VALUES(?1,?2,?3,'source','Исходник',?4)",params![step_id,project_id,asset_id,now]).map_err(|e|e.to_string())?;
    super::migration::root(&tx,&project_id)?;
    tx.execute("UPDATE editor_file_operations SET state='committed',error=NULL WHERE id=?1",[op]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())?;Ok(project_id)
}

pub(super) fn require_mutable(db:&Connection,project:&str)->Result<(),String>{
 if super::references::is_pending(project)?{return Err("Дождитесь завершения прикрепления изображения.".into());}
 let busy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?1 AND state NOT IN ('completed','failed','cancelled','unknownSubmit','interrupted'))",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
 if busy{return Err("В этом проекте выполняется правка. Дождитесь результата либо отмените задачу; просмотр и закрытие доступны.".into());}Ok(())
}

pub fn save_draft(app:&tauri::AppHandle,project_id:&str,expected_revision:u64,draft:Draft)->Result<Value,String>{
    draft.validate()?;
    if super::instructions::local(&draft.operation)&&draft.model_id.is_some()&&!draft.annotation_references.is_empty(){let limit=super::references::route_limit(app,&draft)?;if super::references::capacity(app,project_id,&draft,None)?.len()>limit{return Err("Включённые дополнительные картинки превышают общий лимит модели. Изменение не применено.".into());}}
    let mut db=connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    require_mutable(&tx,project_id)?;validate_draft_resources(&tx,project_id,&draft)?;
    let (revision,previous):(u64,String)=tx.query_row("SELECT revision,draft FROM editor_projects WHERE id=?1",[project_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    let serialized=serde_json::to_string(&draft).map_err(|e|e.to_string())?;
    if revision!=expected_revision&&previous!=serialized{return Err("Черновик уже изменился. Ваш текст сохранён в открытом окне; загрузите актуальную версию перед повтором.".into());}
    let next=if previous==serialized{revision}else{revision.checked_add(1).ok_or("Предел revision")?};let now=unix_now();
    tx.execute("UPDATE editor_projects SET draft=?2,revision=?3,updated_at_unix=?4 WHERE id=?1",params![project_id,serialized,next,now]).map_err(|e|e.to_string())?;stash_step_draft(&tx,project_id,&draft)?;let lifecycle=super::lifecycle::observe(&tx,project_id)?;tx.commit().map_err(|e|e.to_string())?;
    Ok(json!({"projectId":project_id,"revision":next,"savedAtUnix":now,"lifecycle":lifecycle}))
}

pub(super) fn stash_step_draft(db:&Connection,project:&str,draft:&Draft)->Result<(),String>{let step:Option<String>=db.query_row("SELECT s.id FROM editor_projects p JOIN editor_steps s ON s.id=p.active_step_id WHERE p.id=?1 AND s.asset_id=?2",params![project,draft.asset_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;if let Some(step)=step{db.execute("INSERT INTO editor_step_drafts(step_id,draft) VALUES(?1,?2) ON CONFLICT(step_id) DO UPDATE SET draft=excluded.draft",params![step,serde_json::to_string(draft).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;}Ok(())}
pub(super) fn step_draft(db:&Connection,step:&str)->Result<Option<Draft>,String>{let raw:Option<String>=db.query_row("SELECT draft FROM editor_step_drafts WHERE step_id=?1",[step],|r|r.get(0)).optional().map_err(|e|e.to_string())?;raw.map(|r|{let draft:Draft=serde_json::from_str(&r).map_err(|e|e.to_string())?;draft.validate()?;Ok(draft)}).transpose()}

pub(super) fn validate_draft_resources(db:&Connection,project:&str,draft:&Draft)->Result<(),String>{
 draft.validate()?;let asset=|id:&str,kind:&str|->Result<Value,String>{let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_assets WHERE project_id=?1 AND asset_id=?2)",params![project,id],|r|r.get(0)).map_err(|e|e.to_string())?;let a=asset_value(db,id)?;if !belongs||a["kind"]!=kind{return Err("Кадр или маска не принадлежат этому проекту.".into());}Ok(a)};
 let base=asset(&draft.asset_id,"image")?;for refs in draft.annotation_references.values(){for r in refs{asset(&r.asset_id,"image")?;}}for frame in draft.edit_history.undo.iter().chain(&draft.edit_history.redo){for refs in frame.annotation_references.values(){for r in refs{asset(&r.asset_id,"image")?;}}}for r in &draft.references{asset(&r.asset_id,"image")?;}
 let annotations=|annotations:&[Value]|->Result<(),String>{for a in annotations{if a["assetId"]!=draft.asset_id{return Err("Разметка относится к другому кадру.".into());}if a["kind"]=="mask"{let mask=asset(a["maskAssetId"].as_str().ok_or("Нет ID маски")?,"mask")?;if mask["width"]!=base["width"]||mask["height"]!=base["height"]{return Err("Размер маски отличается от кадра.".into());}}}Ok(())};annotations(&draft.annotations)?;for frame in draft.edit_history.undo.iter().chain(&draft.edit_history.redo){annotations(&frame.annotations)?;if let Some(step)=&frame.step_id{let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_steps WHERE project_id=?1 AND id=?2 AND asset_id=?3)",params![project,step,draft.asset_id],|r|r.get(0)).map_err(|e|e.to_string())?;if !belongs{return Err("История правок принадлежит другому шагу проекта".into());}}}
 for key in ["__annotationUndo","__annotationRedo"]{if let Some(stack)=draft.form_values.get(key).and_then(Value::as_array){for frame in stack{annotations(frame.as_array().ok_or("Неверная история разметки")?)?;}}}Ok(())
}
