//! Automatic photo sources must not traverse Windows service data.
use std::path::Path;

pub(crate) fn excluded(path: &Path) -> bool {
    path.to_string_lossy().split(['/', '\\']).any(|part|
        ["$RECYCLE.BIN", "RECYCLER", "System Volume Information"]
            .iter().any(|name| part.eq_ignore_ascii_case(name)))
}

pub(crate) fn require_photo_source(path: &Path) -> Result<(), String> {
    if excluded(path) { Err("Служебный каталог Windows не используется как источник фотографий.".into()) }
    else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_components_are_excluded_without_hiding_user_files() {
        for path in [r"D:\$RECYCLE.BIN\S-1\$I.png", r"\\?\d:\$recycle.bin", r"D:\System Volume Information\a.jpg", "C:/RECYCLER/a.png"] {
            assert!(excluded(Path::new(path)), "{path}");
        }
        for path in [r"D:\photos\$I123.png", r"D:\.hidden\photo.png", r"D:\My $RECYCLE.BIN pictures\a.png"] {
            assert!(!excluded(Path::new(path)), "{path}");
        }
    }
}
