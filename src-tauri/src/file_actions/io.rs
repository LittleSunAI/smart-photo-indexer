//! Byte-preserving file stages. A journal callback is flushed before destructive stages.
use super::types::*;
use crate::*;
use crate::guarded_file::GuardedFile;

pub fn modified_ns(metadata: &fs::Metadata) -> String {
    metadata.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().to_string()).unwrap_or_default()
}

/// Attribute-only, fully shared handle: it can inspect an already guarded file or a directory.
pub fn identity_at(path: &Path) -> Result<String, String> {
    #[cfg(windows)] {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::*;
        let file = fs::OpenOptions::new().access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS).open(path).map_err(|e| e.to_string())?;
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(format!("{}:{}:{}:{}:{}",info.dwVolumeSerialNumber,info.nFileIndexHigh,info.nFileIndexLow,
            info.ftCreationTime.dwHighDateTime,info.ftCreationTime.dwLowDateTime))
    }
    #[cfg(not(windows))] { let _=path; Err("Файловые действия доступны в Windows.".into()) }
}
pub fn capture(path: &str) -> Result<CapturedFile, String> {
    if !Path::new(path).is_absolute() || !is_supported_image(Path::new(path)) || source_paths::excluded(Path::new(path)) {
        return Err("Выберите обычный файл фотографии.".into());
    }
    // Check the given path before canonicalization, which would erase a symlink.
    let guard = GuardedFile::observe(Path::new(path))?;
    let canonical = fs::canonicalize(path).map_err(|e| e.to_string())?;
    if source_paths::excluded(&canonical){return Err("Служебный каталог Windows исключён из файловых действий.".into());}
    let metadata = guard.metadata()?;
    Ok(CapturedFile { path: path_to_string(&canonical), identity: guard.identity()?,
        size_bytes: metadata.len(), modified_ns: modified_ns(&metadata) })
}
pub fn fingerprint(guard: &mut GuardedFile) -> Result<Fingerprint, String> {
    let meta=guard.metadata()?;
    Ok(Fingerprint { identity: guard.identity()?, sha256: guard.sha()?, size_bytes: meta.len(), modified_ns: modified_ns(&meta) })
}
pub fn at(path: &Path) -> Result<Option<Fingerprint>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind()==std::io::ErrorKind::NotFound => {
            // A disconnected/inaccessible parent is not a free filename.
            fs::read_dir(path.parent().ok_or("Нет папки назначения")?).map_err(|e| e.to_string())?;
            Ok(None)
        }
        Err(e) => Err(e.to_string()),
        Ok(_) => { let mut guard=GuardedFile::open(path,false)?; fingerprint(&mut guard).map(Some) }
    }
}
pub fn matches(path: &Path, expected: &Fingerprint) -> bool { at(path).ok().flatten().as_ref()==Some(expected) }
pub fn folder(path: &str) -> Result<String, String> {
    let p=Path::new(path);
    if !p.is_absolute() || !p.is_dir() || source_paths::excluded(p) { return Err("Выберите существующую папку назначения.".into()); }
    let canonical=fs::canonicalize(p).map_err(|e|e.to_string())?;
    if source_paths::excluded(&canonical){return Err("Служебный каталог Windows не может быть папкой назначения.".into());}
    Ok(path_to_string(&canonical))
}
pub fn next_free(target:&Path)->Result<String,String>{
    let mut index=1u64;loop{let candidate=numbered(target,index);if available(&candidate)?{return Ok(path_to_string(&candidate));}
        index=index.checked_add(1).ok_or("Не найдено свободное имя")?;}
}
fn available(path:&Path)->Result<bool,String>{
    match fs::symlink_metadata(path){
        Ok(_)=>Ok(false),
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{
            fs::read_dir(path.parent().ok_or("Нет папки назначения")?).map_err(|e|e.to_string())?;Ok(true)
        }
        Err(error)=>Err(error.to_string()),
    }
}
pub fn same_volume(source_identity: &str, target_parent: &Path) -> Result<bool,String> {
    Ok(source_identity.split(':').next()==identity_at(target_parent)?.split(':').next())
}
pub fn numbered(target: &Path, index: u64) -> PathBuf {
    let stem=target.file_stem().unwrap_or_default().to_string_lossy();
    let suffix=target.extension().map(|e|format!(".{}",e.to_string_lossy())).unwrap_or_default();
    target.with_file_name(format!("{stem} ({index}){suffix}"))
}
pub fn info(path: &str, fingerprint: &Fingerprint) -> FileInfo {
    FileInfo { name: file_name(Path::new(path)), path: path.into(), size_bytes: fingerprint.size_bytes,
        modified_at: fingerprint.modified_ns.parse::<u128>().ok().map(|v|(v/1_000_000_000) as u64) }
}
pub fn persist(db: &Connection, p: &Payload) -> Result<(), String> {
    db.execute("INSERT INTO file_operations(id,kind,image_id,source_path,target_path,sha256,size_bytes,modified_at_unix,status,created_at_unix,updated_at_unix,payload) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'applying',?9,?9,?10) ON CONFLICT(id) DO UPDATE SET image_id=excluded.image_id,target_path=excluded.target_path,payload=excluded.payload,updated_at_unix=excluded.updated_at_unix",
        params![p.id(),p.kind.journal(),p.image_id,p.source_path,p.target_path,p.source_fingerprint.sha256,
        u64_to_i64_saturating(p.source_fingerprint.size_bytes),
        p.source_fingerprint.modified_ns.parse::<u128>().ok().map(|v|(v/1_000_000_000) as i64),unix_now(),serde_json::to_string(p).map_err(|e|e.to_string())?])
        .map_err(|e|e.to_string())?;
    Ok(())
}
pub fn phase(db: &Connection, p: &mut Payload, name: &str) -> Result<(), String> { p.phase=name.into(); persist(db,p) }

pub struct Stage { pub source: GuardedFile, pub output: Option<GuardedFile> }
pub fn stage(db: &Connection, p: &mut Payload) -> Result<Stage,String> {
    let mut source=GuardedFile::open(Path::new(&p.source_path),p.kind!=Kind::Copy)?;
    if fingerprint(&mut source)? != p.source_fingerprint { return Err("Исходная фотография изменилась. Выберите её заново.".into()); }
    let target=PathBuf::from(p.target_path.as_ref().ok_or("Нет назначения")?);
    if p.kind==Kind::Move && same_volume(&p.source_fingerprint.identity,target.parent().ok_or("Нет папки")?)? {
        return Ok(Stage{source,output:None});
    }
    let temporary=target.parent().ok_or("Нет папки")?.join(format!(".smart-photo-{}.partial",p.id()));
    p.stage_path=Some(path_to_string(&temporary)); phase(db,p,"staging")?;
    let mut output=GuardedFile::create_new(&temporary)?;
    p.stage_identity=Some(output.identity()?); persist(db,p)?;
    source.copy_to(&mut output)?;
    if output.sha()?!=p.source_fingerprint.sha256 { return Err("Проверка содержимого копии не прошла. Исходник сохранён.".into()); }
    phase(db,p,"staged")?;
    Ok(Stage{source,output:Some(output)})
}
/// Called inside the existing coordinator -> files -> index fence, never while awaiting a dialog.
pub fn publish(db: &Connection, p: &mut Payload, stage: &mut Stage) -> Result<(),String> {
    let target=PathBuf::from(p.target_path.as_ref().ok_or("Нет назначения")?);
    let mut backup=None;
    if let Some(expected)=p.target_before.clone() {
        let mut old=GuardedFile::open(&target,true)?;
        if fingerprint(&mut old)?!=expected { return Err("Файл назначения изменился. Выберите действие ещё раз.".into()); }
        let backup_path=target.parent().ok_or("Нет папки")?.join(format!(".smart-photo-{}.backup",p.id()));
        p.backup_path=Some(path_to_string(&backup_path)); phase(db,p,"backupIntent")?;
        old.rename(&backup_path)?; phase(db,p,"backupSaved")?; backup=Some(old);
    }
    phase(db,p,"publishIntent")?;
    let published = match &stage.output { Some(output)=>output.rename(&target), None=>stage.source.rename(&target) };
    if let Err(error)=published {
        if let Some(old)=&backup { if !target.try_exists().map_err(|e|e.to_string())? { old.rename(&target)?; p.backup_path=None; persist(db,p)?; } }
        return Err(error);
    }
    p.filesystem_applied=true;
    if p.kind==Kind::Move && stage.output.is_none() { p.source_removed=true; }
    let guard=stage.output.as_mut().unwrap_or(&mut stage.source);
    p.published_fingerprint=Some(fingerprint(guard)?);
    phase(db,p,"published")?;
    // Keep the handle-bound previous target until metadata commit; its path/identity is durable.
    drop(backup);
    if p.kind==Kind::Move && stage.output.is_some() {
        phase(db,p,"sourceRemoveIntent")?;
        // Consume only the original opened file, never a similarly named replacement.
        // Removal is deferred to finish_move, which owns the source handle.
    }
    Ok(())
}
pub fn finish_move(db: &Connection,p:&mut Payload,stage:Stage) -> Result<(),String> {
    if p.kind==Kind::Move && !p.source_removed {
        stage.source.remove()?;
        p.source_removed=true; phase(db,p,"sourceRemoved")?;
    }
    Ok(())
}
pub fn cleanup(p:&Payload) -> Vec<String> {
    let mut warnings=Vec::new();
    for (path,identity) in [(p.stage_path.as_ref(),p.stage_identity.as_ref()),
        (p.backup_path.as_ref(),p.target_before.as_ref().map(|f|&f.identity))] {
        if let (Some(path),Some(expected))=(path,identity) {
            if !Path::new(path).exists(){continue;}
            let result=(||{let guard=GuardedFile::open(Path::new(path),true)?;
                if guard.identity()?!=*expected{return Err("Временный файл изменился; очистка пропущена.".into());}guard.remove()})();
            if let Err(error)=result { warnings.push(error); }
        }
    }
    warnings
}

/// Reconcile facts only. Never publish a new result or delete a remaining source on restart.
pub fn inspect_recovery(p:&mut Payload)->Result<bool,String>{
    if p.kind==Kind::Trash{
        if p.trash_receipt.is_none(){return Err("Исход корзины не подтверждён; повторное удаление не запускалось.".into());}
        if at(Path::new(&p.source_path))?.is_some(){return Err("Исходный путь снова занят. Требуется сверка корзины.".into());}
        p.filesystem_applied=true;p.source_removed=true;return Ok(true);
    }
    let target=PathBuf::from(p.target_path.as_ref().ok_or("Нет пути результата")?);
    let published=at(&target)?;
    let expected=p.published_fingerprint.as_ref().unwrap_or(&p.source_fingerprint);
    let valid=published.as_ref().is_some_and(|f|f.sha256==expected.sha256&&
        (Some(&f.identity)==p.stage_identity.as_ref()||f.identity==p.source_fingerprint.identity||Some(f)==p.published_fingerprint.as_ref()));
    if valid{
        p.published_fingerprint=published;p.filesystem_applied=true;
        if p.kind==Kind::Move{
            let original=at(Path::new(&p.source_path))?;
            if original.is_none(){p.source_removed=true;}
            else if !p.source_removed&&original.as_ref()!=Some(&p.source_fingerprint){return Err("Исходный путь изменился после публикации. Оба файла сохранены для сверки.".into());}
        }
        return Ok(true);
    }
    if let (Some(backup),Some(old))=(&p.backup_path,&p.target_before){
        if matches(Path::new(backup),old)&&at(&target)?.is_none(){
            GuardedFile::open(Path::new(backup),true)?.rename(&target)?;p.backup_path=None;
        }
    }
    if !matches(Path::new(&p.source_path),&p.source_fingerprint){return Err("Исходник или назначение изменились. Файлы оставлены для сверки.".into());}
    if p.backup_path.is_some(){return Err("Прежнее назначение сохранено; требуется сверка результата.".into());}
    let _=cleanup(p);Ok(false)
}
