use crate::*;
use std::{collections::{BTreeMap, HashSet}, sync::{OnceLock, RwLock, Mutex, Condvar}, time::Duration};

const MAX_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_FILES: usize = 10_000;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
pub static GATE: RwLock<()> = RwLock::new(());
#[derive(Default)]
struct Maintenance { checked_at_unix: u64, error: Option<String> }
fn maintenance() -> &'static Mutex<Maintenance> { static STATUS: OnceLock<Mutex<Maintenance>> = OnceLock::new(); STATUS.get_or_init(Default::default) }
type Inflight = (Mutex<HashSet<String>>, Condvar);
fn inflight() -> &'static Inflight { static STATE: OnceLock<Inflight> = OnceLock::new(); STATE.get_or_init(Default::default) }
pub struct KeyGuard(String);
impl Drop for KeyGuard {
    fn drop(&mut self) { let (keys, changed) = inflight(); if let Ok(mut keys) = keys.lock() { keys.remove(&self.0); changed.notify_all(); } }
}
pub fn lock_key(key: String) -> Result<KeyGuard, String> {
    let (keys, changed) = inflight(); let mut keys = keys.lock().map_err(|_| "Thumbnail request lock failed.")?;
    while keys.contains(&key) { keys = changed.wait(keys).map_err(|_| "Thumbnail request lock failed.")?; }
    keys.insert(key.clone()); Ok(KeyGuard(key))
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))] { metadata.file_type().is_symlink() }
}

pub(crate) fn owned_name(name: &str) -> bool {
    let key=name.strip_prefix("srgb-alpha-v1-").and_then(|value|value.strip_suffix(".png"))
        .or_else(||name.strip_prefix("srgb-v3-").and_then(|value|value.strip_suffix(".jpg")));
    let Some(key)=key else {return legacy_name(name);};
    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn legacy_name(name: &str) -> bool {
    let Some(key) = name.strip_prefix("srgb-v2-").and_then(|value| value.strip_suffix(".jpg")) else { return false; };
    let Some((hash, rest)) = key.split_once('_') else { return false; };
    let Some((dimensions, revision)) = rest.split_once('-') else { return false; };
    let Some((width, height)) = dimensions.split_once('x') else { return false; };
    hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) && [width,height,revision].iter().all(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
}

pub fn directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data = application_data_directory(app)?;
    fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let data = fs::canonicalize(&data).map_err(|e| e.to_string())?;
    let directory = crate::photo_editor::storage::read(app)?.cache_root.map(PathBuf::from).unwrap_or_else(||data.join("thumbnails"));
    if !directory.is_absolute(){return Err("Папка кэша должна иметь абсолютный путь".into());}
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let metadata = fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || is_link(&metadata) { return Err("Папка кэша не должна быть ссылкой или перенаправлением.".into()); }
    let resolved=fs::canonicalize(&directory).map_err(|e| e.to_string())?;
    let expected_parent=fs::canonicalize(directory.parent().ok_or("Нет папки кэша")?).map_err(|e|e.to_string())?;
    if resolved.parent()!=Some(expected_parent.as_path()){return Err("Папка кэша перенаправлена за пределы выбранного каталога.".into());}
    Ok(resolved)
}

pub fn touch(path: &Path) {
    if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_times(fs::FileTimes::new().set_modified(SystemTime::now()));
    }
}

pub fn validate_entry(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || is_link(&metadata) => Err("Миниатюра кэша не должна быть ссылкой или папкой.".into()),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Statistics {
    files: usize, bytes: u64, removed_files: usize, removed_bytes: u64, skipped_files: usize,
    max_bytes: u64, max_files: usize, max_age_days: u32,
    last_maintenance_at_unix: u64,
    maintenance_error: Option<String>,
}

// Stream entries and retain only the newest bounded set. Source photos, SQLite,
// unknown filenames, temporary writes and reparse points are never deleted.
fn walk(directory: &Path, clean: bool, clear: bool, max_bytes: u64, max_files: usize, max_age: Duration) -> Result<Statistics, String> {
    let mut stats = Statistics { max_bytes, max_files, max_age_days: (max_age.as_secs()/86400) as u32, ..Default::default() };
    let mut newest: BTreeMap<(SystemTime, PathBuf), u64> = BTreeMap::new();
    for (index, entry) in fs::read_dir(directory).map_err(|e| e.to_string())?.enumerate() {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = match fs::symlink_metadata(entry.path()) { Ok(value) => value, Err(e) if e.kind()==std::io::ErrorKind::NotFound => continue, Err(e) => return Err(e.to_string()) };
        if !metadata.is_file() || is_link(&metadata) || !owned_name(&entry.file_name().to_string_lossy()) { stats.skipped_files += 1; continue; }
        let path = entry.path();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        if clean && (clear || modified.elapsed().is_ok_and(|age| age > max_age)) {
            fs::remove_file(&path).map_err(|e| format!("Не удалось очистить миниатюру: {e}"))?;
            stats.removed_files += 1; stats.removed_bytes += metadata.len(); continue;
        }
        stats.files += 1; stats.bytes += metadata.len();
        if clean {
            newest.insert((modified, path), metadata.len());
            while stats.files > max_files || stats.bytes > max_bytes {
                let Some((( _, old), bytes)) = newest.pop_first() else { break; };
                fs::remove_file(&old).map_err(|e| format!("Не удалось сократить кэш: {e}"))?;
                stats.files -= 1; stats.bytes -= bytes; stats.removed_files += 1; stats.removed_bytes += bytes;
            }
        }
        if index % 128 == 127 { std::thread::sleep(Duration::from_millis(1)); }
    }
    Ok(stats)
}

pub fn start(app: &tauri::AppHandle) {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let app = app.clone();
        std::thread::spawn(move || loop {
            let result = (|| {
                let _guard = GATE.write().map_err(|_| "Thumbnail cache lock failed.")?;
                walk(&directory(&app)?, true, false, MAX_BYTES, MAX_FILES, MAX_AGE)
            })();
            if let Ok(mut status) = maintenance().lock() {
                status.checked_at_unix = unix_now(); status.error = result.err();
            }
            std::thread::sleep(Duration::from_secs(60));
        });
    });
}

#[tauri::command]
pub async fn get_thumbnail_cache_statistics(app: tauri::AppHandle) -> Result<Statistics, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = GATE.read().map_err(|_| "Thumbnail cache lock failed.")?;
        let mut stats = walk(&directory(&app)?, false, false, MAX_BYTES, MAX_FILES, MAX_AGE)?;
        if let Ok(status) = maintenance().lock() { stats.last_maintenance_at_unix = status.checked_at_unix; stats.maintenance_error = status.error.clone(); }
        Ok(stats)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clear_thumbnail_cache(app: tauri::AppHandle) -> Result<Statistics, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = GATE.write().map_err(|_| "Thumbnail cache lock failed.")?;
        walk(&directory(&app)?, true, true, MAX_BYTES, MAX_FILES, MAX_AGE)
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_enforces_oldest_first_count_bytes_age_and_preserves_foreign_files() {
        let base = std::env::temp_dir().join(format!("smart-photo-cache-unit-{}-{}", std::process::id(), unix_now()));
        fs::create_dir(&base).unwrap();
        for i in 0..5 {
            let path = base.join(format!("srgb-v3-{i:064x}.jpg")); fs::write(&path, vec![0; 100]).unwrap();
            fs::OpenOptions::new().write(true).open(path).unwrap().set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(100+i))).unwrap();
        }
        let foreign=base.join("original.jpg"); fs::write(&foreign, "protected").unwrap();
        let stats=walk(&base,true,false,250,3,Duration::MAX).unwrap();
        assert_eq!(stats.files,2); assert_eq!(stats.bytes,200); assert_eq!(stats.removed_files,3);
        assert!(base.join(format!("srgb-v3-{:064x}.jpg",4)).exists());
        let stats=walk(&base,true,false,250,3,Duration::from_secs(1)).unwrap(); assert_eq!(stats.removed_files,2); assert_eq!(stats.files,0);
        assert_eq!(fs::read_to_string(&foreign).unwrap(), "protected");
        fs::remove_file(foreign).unwrap();
        // Windows may still hold a deleted fixture file briefly (e.g. a scanner).
        // Retry only empty-directory cleanup; every cache/foreign-file assertion
        // above remains strict, and unexpected entries/errors still fail.
        for attempt in 0..20 {
            match fs::remove_dir(&base) {
                Ok(()) => break,
                Err(error) if error.raw_os_error()==Some(145)
                    && fs::read_dir(&base).unwrap().next().is_none() && attempt<19 => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("Cannot remove empty cache test fixture: {error}"),
            }
        }
    }
    #[test]
    fn ownership_requires_exact_versioned_thumbnail_name() {
        assert!(owned_name(&format!("srgb-v3-{:064x}.jpg", 1)));
        assert!(owned_name(&format!("srgb-alpha-v1-{:064x}.png", 1)));
        assert!(!owned_name(&format!("srgb-alpha-v1-{:064x}.jpg", 1)));
        assert!(!owned_name(&format!("srgb-alpha-v1-{:064x}.png.backup", 1)));
        assert!(owned_name("srgb-v2-0123456789abcdef_360x260-1791481974786219500.jpg"));
        for name in ["cats.jpg","smart_photo_indexer.sqlite",".smart-photo-1.partial","srgb-v3-cats.jpg","srgb-v2-0123456789abcdef_360x260-.jpg","../srgb-v3-000.jpg"] { assert!(!owned_name(name)); }
    }
}
