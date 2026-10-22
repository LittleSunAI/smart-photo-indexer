//! Storage preferences and copy/verify/switch migration. Originals and old roots remain intact.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
pub static MIGRATING: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub version: u32,
    pub output_mode: String,
    pub output_folder: Option<String>,
    pub editor_root: Option<String>,
    pub editor_generation: Option<String>,
    #[serde(default)]
    pub cache_root: Option<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            output_mode: "beside".into(),
            output_folder: None,
            editor_root: None,
            editor_generation: None,
            cache_root: None,
        }
    }
}
fn storage_link(m: &fs::Metadata) -> bool {
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
fn config(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(application_data_directory(app)?.join("photo-storage.json"))
}
pub fn read(app: &tauri::AppHandle) -> Result<Preferences, String> {
    let p = config(app)?;
    if !p.exists() {
        return Ok(Preferences::default());
    }
    if fs::metadata(&p).map_err(|e| e.to_string())?.len() > 65536 {
        return Err("Настройки хранения повреждены".into());
    }
    let v: Preferences = serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?)
        .map_err(|_| "Настройки хранения повреждены")?;
    if v.version != 1 || !["beside", "folder", "ask"].contains(&v.output_mode.as_str()) {
        return Err("Неподдерживаемые настройки хранения".into());
    }
    Ok(v)
}
fn write(app: &tauri::AppHandle, value: &Preferences) -> Result<(), String> {
    let target = config(app)?;
    let parent = target.parent().ok_or("Нет папки настроек")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if fs::symlink_metadata(&target).is_ok_and(|m| storage_link(&m)) {
        return Err("Файл настроек перенаправлен; изменение запрещено".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = parent.join(format!(".photo-storage-{nonce}.json"));
    file_safety::atomic_save(&temporary, |out, _| {
        std::io::Write::write_all(
            out,
            &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    })?;
    #[cfg(windows)]
    let result = {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(())
        }
    };
    #[cfg(not(windows))]
    let result = fs::rename(&temporary, &target).map_err(|e| e.to_string());
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub fn custom_root(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let p = read(app)?;
    if p.editor_generation != index_backups::editor_folder_name(app)? {
        return Ok(None);
    }
    let Some(path) = p.editor_root else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if !path.is_absolute() || !path.is_dir() {
        return Err("Каталог проектов недоступен. Подключите диск с проектами; новое пустое хранилище не создано.".into());
    }
    if storage_link(&fs::symlink_metadata(&path).map_err(|e| e.to_string())?) {
        return Err("Каталог проектов не может быть ссылкой".into());
    }
    Ok(Some(path.canonicalize().map_err(|e| e.to_string())?))
}
pub fn info(app: &tauri::AppHandle) -> Result<Value, String> {
    let p = read(app)?;
    Ok(
        json!({"preferences":p,"editorRoot":super::store::root(app)?,"thumbnailRoot":thumbnail_cache::directory(app)?,"selectedPreviewRoot":thumbnail_cache::directory(app)?.join("selected")}),
    )
}
pub fn save_preferences(
    app: &tauri::AppHandle,
    mode: &str,
    folder: Option<&str>,
) -> Result<Value, String> {
    if MIGRATING.load(Ordering::Acquire) {
        return Err("Дождитесь завершения переноса".into());
    }
    if !["beside", "folder", "ask"].contains(&mode) {
        return Err("Неизвестный режим сохранения".into());
    }
    let mut p = read(app)?;
    if mode == "folder" {
        let path = PathBuf::from(folder.ok_or("Выберите папку изображений")?);
        if !path.is_absolute() || !path.is_dir() {
            return Err("Папка изображений недоступна".into());
        }
        p.output_folder = Some(path_to_string(
            &path.canonicalize().map_err(|e| e.to_string())?,
        ));
    }
    p.output_mode = mode.into();
    write(app, &p)?;
    info(app)
}
pub fn output_target(
    app: &tauri::AppHandle,
    project: &str,
    format: &str,
) -> Result<Option<PathBuf>, String> {
    let p = read(app)?;
    if p.output_mode == "ask" {
        return Ok(None);
    }
    let db = super::store::connection(app)?;
    let (source, name): (String, String) = db
        .query_row(
            "SELECT source_path,source_name FROM editor_projects WHERE id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let parent = if p.output_mode == "folder" {
        PathBuf::from(
            p.output_folder
                .ok_or("Выберите папку готовых изображений в настройках")?,
        )
    } else {
        let source = PathBuf::from(source);
        if !source.is_absolute() {
            return Ok(None);
        }
        source.parent().ok_or("Нет папки исходника")?.to_path_buf()
    };
    if !parent.is_dir() {
        return Err("Папка сохранения недоступна. Выберите другое место в окне сохранения.".into());
    }
    let stem = Path::new(&name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("photo");
    let extension = if format == "jpeg" { "jpg" } else { format };
    for i in 1..10000 {
        let path = parent.join(format!("{stem}-edited-{i}.{extension}"));
        if !path.exists() {
            return Ok(Some(path));
        }
    }
    Err("Не удалось подобрать свободное имя изображения".into())
}

pub fn migrate_cache(app: &tauri::AppHandle, parent: &Path) -> Result<Value, String> {
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let source = thumbnail_cache::directory(app)?;
    if !parent.is_dir() || parent.starts_with(&source) {
        return Err("Выберите папку вне прежнего кэша".into());
    }
    let _gate = thumbnail_cache::GATE.write().map_err(|_| "Кэш занят")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let destination = parent.join(format!("SmartFoto-thumbnails-{nonce}"));
    fs::create_dir(&destination).map_err(|e| e.to_string())?;
    let mut count = 0;
    for entry in fs::read_dir(&source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !thumbnail_cache::owned_name(&name) {
            continue;
        }
        let from = entry.path();
        thumbnail_cache::validate_entry(&from)?;
        if !from.is_file() {
            continue;
        }
        let hash = compute_file_sha256(&from)?;
        let to = destination.join(name);
        file_safety::atomic_save(&to, |out, _| {
            let mut input = fs::File::open(&from).map_err(|e| e.to_string())?;
            std::io::copy(&mut input, out).map_err(|e| e.to_string())?;
            Ok(())
        })?;
        if compute_file_sha256(&to)? != hash {
            return Err("Проверка копии кэша не прошла; прежний каталог сохранён".into());
        }
        count += 1;
    }
    let mut p = read(app)?;
    p.cache_root = Some(path_to_string(&destination));
    write(app, &p)?;
    drop(_gate);
    let mut result = info(app)?;
    result["copiedFiles"] = json!(count);
    result["oldCacheRoot"] = json!(source);
    result["oldRootPreserved"] = json!(true);
    Ok(result)
}

pub fn migrate(app: &tauri::AppHandle, parent: &Path) -> Result<Value, String> {
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    if !parent.is_dir() {
        return Err("Выберите существующую папку".into());
    }
    let source = super::store::root(app)?;
    if parent.starts_with(&source) {
        return Err("Новое хранилище не размещается внутри прежнего".into());
    }
    let prepared = processing::cloud_fence(app, || {
        if index_backups::restore_blocked(app)? {
            return Err("Завершите или отмените подготовленное восстановление базы".into());
        }
        if app.state::<super::EditorSessions>().has_open_sessions()? {
            return Err("Закройте окна редактора перед переносом проектов. Основное окно можно оставить открытым.".into());
        }
        if super::references::has_pending()? {
            return Err("Дождитесь завершения прикрепления изображений".into());
        }
        super::jobs::require_settled(app)?;
        if processing::occupied_cloud(app, "fal")? > 0
            || processing::occupied_cloud(app, "openrouter")? > 0
        {
            return Err("Завершите и сверьте облачные задачи перед переносом".into());
        }
        let db = super::store::connection(app)?;
        let pending: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM editor_publications WHERE state<>'completed')",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if pending {
            return Err("Завершите сохранение результатов перед переносом".into());
        }
        if MIGRATING.swap(true, Ordering::AcqRel) {
            return Err("Перенос уже выполняется".into());
        }
        Ok(db)
    })?;
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            MIGRATING.store(false, Ordering::Release);
        }
    }
    let _reset = Reset;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let destination = parent.join(format!("SmartFoto-projects-{nonce}"));
    fs::create_dir(&destination).map_err(|e| e.to_string())?;
    for dir in ["assets", "masks", "previews"] {
        fs::create_dir(destination.join(dir)).map_err(|e| e.to_string())?;
    }
    let database = destination.join("editor.sqlite");
    prepared
        .execute("VACUUM INTO ?1", [path_to_string(&database)])
        .map_err(|e| e.to_string())?;
    let paths = {
        let mut q = prepared
            .prepare("SELECT relative_path,preview_path,sha256 FROM editor_assets")
            .map_err(|e| e.to_string())?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    let mut copied = std::collections::HashSet::new();
    let mut bytes = fs::metadata(&database).map_err(|e| e.to_string())?.len();
    for (file, preview, expected) in paths {
        for (relative, original) in [(file, true), (preview, false)] {
            if !copied.insert(relative.clone()) {
                continue;
            }
            let from = super::store::owned_path(app, &relative)?;
            if !from.is_file() {
                continue;
            }
            let sha = compute_file_sha256(&from)?;
            if original && sha != expected {
                return Err("SHA файла проекта отличается; перенос не включён, прежнее хранилище остаётся рабочим".into());
            }
            let to = destination.join(&relative);
            if to.parent().is_none_or(|p| !p.starts_with(&destination)) {
                return Err("Неверный путь файла проекта".into());
            }
            file_safety::atomic_save(&to, |out, _| {
                let mut input = fs::File::open(&from).map_err(|e| e.to_string())?;
                std::io::copy(&mut input, out).map_err(|e| e.to_string())?;
                Ok(())
            })?;
            if compute_file_sha256(&to)? != sha {
                return Err("Ошибка проверки копии; прежнее хранилище сохранено".into());
            }
            bytes += fs::metadata(to).map_err(|e| e.to_string())?.len();
        }
    }
    super::backup::validate_editor(&destination)?;
    let mut preferences = read(app)?;
    preferences.editor_root = Some(path_to_string(
        &destination.canonicalize().map_err(|e| e.to_string())?,
    ));
    preferences.editor_generation = index_backups::editor_folder_name(app)?;
    write(app, &preferences)?;
    MIGRATING.store(false, Ordering::Release);
    let mut result = info(app)?;
    result["oldRoot"] = json!(source);
    result["copiedBytes"] = json!(bytes);
    result["oldRootPreserved"] = json!(true);
    Ok(result)
}
