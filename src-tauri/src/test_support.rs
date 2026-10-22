//! Private fixture directories must stay distinct even on a coarse Windows clock.
use std::{fs, io::ErrorKind, path::PathBuf, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    loop {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}-{sequence}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            // A directory left by a previous process is never reused or removed.
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("Could not create private test directory {}: {error}", path.display()),
        }
    }
}
