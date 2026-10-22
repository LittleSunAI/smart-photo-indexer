#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChooseSaveOriginalImagePathInputDto {
    source_path: String,
    default_file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveOriginalImageCopyInputDto {
    source_path: String,
    target_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveOriginalImageCopyResultDto {
    target_path: String,
    file_name: String,
    size_bytes: u64,
    saved_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteImageResultDto {
    deleted_path: String,
    deleted_at_unix: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShareImageFileResultDto {
    file_name: String,
    mime_type: String,
    bytes: Vec<u8>,
    size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenericImageActionResultDto {
    path: String,
    action: String,
    completed_at_unix: u64,
}

fn validate_existing_image_file(path_text: &str) -> Result<PathBuf, String> {
    let trimmed = path_text.trim();
    if trimmed.is_empty() {
        return Err("Image path is empty.".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(format!("Image file does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if !IMAGE_EXTENSIONS.iter().any(|candidate| candidate == &extension.as_str()) {
        return Err(format!("Unsupported image format: {}", extension));
    }

    Ok(path)
}

fn viewer_file_name(path: &Path, fallback: &str) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[tauri::command]
async fn edit_image_file(path: String) -> Result<GenericImageActionResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_path = validate_existing_image_file(&path)?;
        #[cfg(target_os = "windows")]
        {
            // "open" may point back to this viewer after installing its associations.
            if shell_execute_file_verb(&source_path, "edit").is_err() {
                let system = PathBuf::from(std::env::var_os("SYSTEMROOT").ok_or("Windows directory is unavailable.")?);
                let mut candidates = vec![system.join("System32").join("mspaint.exe")];
                if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                    candidates.push(PathBuf::from(local).join("Microsoft").join("WindowsApps").join("mspaint.exe"));
                }
                candidates.push(system.join("SysWOW64").join("mspaint.exe"));
                let mut launched = false; let mut last_error = None;
                // Current Windows installs Paint as a Store app with an execution alias.
                // Use exact OS paths, never search the project directory for an executable.
                for editor in candidates.into_iter().filter(|path|path.is_file()) {
                    match std::process::Command::new(&editor).arg(&source_path).spawn() {
                        Ok(_) => { launched = true; break; }
                        Err(error) => last_error = Some(error),
                    }
                }
                if !launched {
                    return Err(match last_error {
                        Some(error) => format!("Не удалось запустить установленный редактор Paint: {error}"),
                        None => "В Windows не настроен внешний редактор изображений и не найден Paint. Установите редактор или настройте для фотографий действие «Изменить».".into(),
                    });
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        { return Err("Configure an external image editor for this platform.".to_string()); }
        Ok(GenericImageActionResultDto { path: path_to_string(&source_path), action: "edit".into(), completed_at_unix: unix_now() })
    }).await.map_err(|error| format!("Editor worker failed: {error}"))?
}

fn wallpaper_compatible_path(app_handle: &tauri::AppHandle, source_path: &Path) -> Result<PathBuf, String> {
    let expected = compute_file_sha256(source_path)?;
    let image = file_safety::decode_bounded(source_path)?;
    let app_data_dir = application_data_directory(&app_handle)?;
    let wallpaper_dir = app_data_dir.join("wallpaper");
    fs::create_dir_all(&wallpaper_dir)
        .map_err(|error| format!("Cannot create wallpaper folder {}: {error}", wallpaper_dir.display()))?;
    // Every wallpaper uses the same bounded orientation/ICC conversion as thumbnails and AI.
    // A unique, atomically published cache file protects previous wallpaper files and concurrent calls.
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
    let target_path = wallpaper_dir.join(format!("srgb-v2-{}-{nonce}.png", &expected[..16]));
    file_safety::atomic_save(&target_path, |output, _| {
        image.write_to(output, image::ImageFormat::Png).map_err(|error| error.to_string())?;
        if compute_file_sha256(source_path)? != expected { return Err("Исходный файл изменился; фон рабочего стола не установлен.".into()); }
        Ok(())
    })?;
    Ok(target_path)
}

#[tauri::command]
async fn choose_save_original_image_path(input: ChooseSaveOriginalImagePathInputDto) -> Result<ChooseSaveImagePathResultDto, String> {
    let source_path = validate_existing_image_file(&input.source_path)?;

    let mut dialog = rfd::AsyncFileDialog::new();

    if let Some(parent) = source_path.parent() {
        if parent.exists() && parent.is_dir() {
            dialog = dialog.set_directory(parent);
        }
    }

    let file_name = {
        let default_file_name = input.default_file_name.trim();
        if default_file_name.is_empty() {
            viewer_file_name(&source_path, "image")
        } else {
            default_file_name.to_string()
        }
    };

    dialog = dialog.set_file_name(file_name);

    if let Some(extension) = source_path.extension().and_then(|value| value.to_str()) {
        let extension = extension.to_lowercase();
        match extension.as_str() {
            "jpg" | "jpeg" => {
                dialog = dialog.add_filter("JPEG image", &["jpg", "jpeg"]);
            }
            "png" => {
                dialog = dialog.add_filter("PNG image", &["png"]);
            }
            "webp" => {
                dialog = dialog.add_filter("WebP image", &["webp"]);
            }
            "gif" => {
                dialog = dialog.add_filter("GIF image", &["gif"]);
            }
            "bmp" => {
                dialog = dialog.add_filter("BMP image", &["bmp"]);
            }
            "tif" | "tiff" => {
                dialog = dialog.add_filter("TIFF image", &["tif", "tiff"]);
            }
            "avif" => {
                dialog = dialog.add_filter("AVIF image", &["avif"]);
            }
            "heic" | "heif" => {
                dialog = dialog.add_filter("HEIF image", &["heic", "heif"]);
            }
            _ => {}
        }
    }

    Ok(ChooseSaveImagePathResultDto {
        selected_path: dialog.save_file().await.map(|file| path_to_string(file.path())),
        opened_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn save_original_image_copy(input: SaveOriginalImageCopyInputDto) -> Result<SaveOriginalImageCopyResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || save_original_image_copy_internal(input))
        .await.map_err(|error| format!("Image copy worker failed: {error}"))?
}

fn save_original_image_copy_internal(input: SaveOriginalImageCopyInputDto) -> Result<SaveOriginalImageCopyResultDto, String> {
    let source_path = validate_existing_image_file(&input.source_path)?;

    let target_text = input.target_path.trim();
    if target_text.is_empty() {
        return Err("Target path is empty.".to_string());
    }

    let target_path = PathBuf::from(target_text);
    if source_path == target_path {
        return Err("Source and target paths are the same.".to_string());
    }

    let expected = compute_file_sha256(&source_path)?;
    file_safety::atomic_save(&target_path, |output, temporary| {
        let mut input = fs::File::open(&source_path).map_err(|error| error.to_string())?;
        std::io::copy(&mut input, output).map_err(|error| format!("Cannot copy image: {error}"))?;
        if compute_file_sha256(temporary)? != expected || compute_file_sha256(&source_path)? != expected {
            return Err("Source changed while copying. The output was not published.".to_string());
        }
        Ok(())
    })?;

    let metadata = fs::metadata(&target_path)
        .map_err(|error| format!("Cannot read saved image metadata {}: {error}", target_path.display()))?;

    Ok(SaveOriginalImageCopyResultDto {
        target_path: path_to_string(&target_path),
        file_name: viewer_file_name(&target_path, "image"),
        size_bytes: metadata.len(),
        saved_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn trash_image_file(app_handle: tauri::AppHandle, path: String) -> Result<DeleteImageResultDto, String> {
    let worker_handle = app_handle.clone();
    let result = tauri::async_runtime::spawn_blocking(move || file_operations::trash(worker_handle, path))
        .await.map_err(|error| format!("Recycle bin worker failed: {error}"))??;
    viewer_sessions::remove_path(&app_handle, &result.deleted_path)?;
    Ok(result)
}

#[tauri::command]
async fn read_image_file_for_share(path: String) -> Result<ShareImageFileResultDto, String> {
    tauri::async_runtime::spawn_blocking(move||read_image_file_for_share_blocking(path)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn share_image_file(window: tauri::WebviewWindow, path: String) -> Result<GenericImageActionResultDto, String> {
    #[cfg(target_os = "windows")]
    {
        let owner = window.hwnd().map_err(|error|error.to_string())?.0 as usize;
        let (source, file, title) = tauri::async_runtime::spawn_blocking(move || {
            let source = validate_existing_image_file(&path)?;
            let title = viewer_file_name(&source, "Изображение");
            let file = windows_share::prepare(&source)?;
            Ok::<_, String>((source, file, title))
        }).await.map_err(|error|error.to_string())??;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let owner_window = window.clone();
        window.run_on_main_thread(move || {
            let result = if owner_window.hwnd().is_ok_and(|value|value.0 as usize == owner) {
                owner_window.set_focus().map_err(|error|error.to_string())
                    .and_then(|_|windows_share::show(owner, file, title))
            } else { Err("Окно просмотра уже закрыто.".into()) };
            let _ = sender.send(result);
        }).map_err(|error|error.to_string())?;
        receiver.await.map_err(|error|error.to_string())??;
        Ok(GenericImageActionResultDto { path: path_to_string(&source), action: "sharePanelOpened".into(), completed_at_unix: unix_now() })
    }
    #[cfg(not(target_os = "windows"))]
    { let _=(window,path); Err("Системная панель «Поделиться» доступна в Windows.".into()) }
}
fn read_image_file_for_share_blocking(path: String) -> Result<ShareImageFileResultDto, String> {
    let source_path = validate_existing_image_file(&path)?;
    const MAX_SHARE_BYTES:u64=32*1024*1024;
    let file=fs::File::open(&source_path).map_err(|e|e.to_string())?;
    if file.metadata().map_err(|e|e.to_string())?.len()>MAX_SHARE_BYTES{return Err("Файл больше 32 МБ. Поделитесь им через папку, чтобы не перегружать просмотрщик.".into());}
    let mut bytes=Vec::new();file.take(MAX_SHARE_BYTES+1).read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read image for share {}: {error}", source_path.display()))?;
    if bytes.len() as u64>MAX_SHARE_BYTES{return Err("Файл изменился и превышает предел передачи 32 МБ.".into());}
    let metadata = fs::metadata(&source_path)
        .map_err(|error| format!("Cannot read image metadata {}: {error}", source_path.display()))?;

    let mime_type = mime_guess::from_path(&source_path)
        .first_raw()
        .map(str::to_string)
        .unwrap_or_else(|| image_mime_type(&source_path));

    Ok(ShareImageFileResultDto {
        file_name: viewer_file_name(&source_path, "image"),
        mime_type,
        bytes,
        size_bytes: metadata.len(),
    })
}

#[tauri::command]
async fn print_image_file(window: tauri::WebviewWindow, path: String) -> Result<GenericImageActionResultDto, String> {
    #[cfg(target_os = "windows")]
    windows_print::trace("command-entry");
    #[cfg(target_os = "windows")]
    let owner = window.hwnd().map_err(|error|error.to_string())?.0 as usize;
    #[cfg(target_os = "windows")]
    windows_print::trace("owner-obtained");
    #[cfg(not(target_os = "windows"))]
    let owner = {let _ = window;0};
    tauri::async_runtime::spawn_blocking(move||print_image_file_blocking(path,owner)).await.map_err(|e|e.to_string())?
}
fn print_image_file_blocking(path: String, owner:usize) -> Result<GenericImageActionResultDto, String> {
    let source_path = validate_existing_image_file(&path)?;

    let completed = {
        #[cfg(target_os = "windows")]
        { windows_print::print(&source_path,owner)? }
        #[cfg(not(target_os = "windows"))]
        { let _=owner;return Err("Print action is currently implemented only for Windows.".to_string()); }
    };

    Ok(GenericImageActionResultDto {
        path: path_to_string(&source_path),
        action: if completed {"print"} else {"printCancelled"}.to_string(),
        completed_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn set_desktop_wallpaper(app_handle: tauri::AppHandle, path: String) -> Result<GenericImageActionResultDto, String> {
    tauri::async_runtime::spawn_blocking(move||set_desktop_wallpaper_blocking(app_handle,path)).await.map_err(|e|e.to_string())?
}
fn set_desktop_wallpaper_blocking(app_handle: tauri::AppHandle, path: String) -> Result<GenericImageActionResultDto, String> {
    let source_path = validate_existing_image_file(&path)?;

    #[cfg(target_os = "windows")]
    {
        let wallpaper_path = wallpaper_compatible_path(&app_handle, &source_path)?;
        set_windows_desktop_wallpaper(&wallpaper_path)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Err("Desktop wallpaper action is currently implemented only for Windows.".to_string());
    }

    Ok(GenericImageActionResultDto {
        path: path_to_string(&source_path),
        action: "set_desktop_wallpaper".to_string(),
        completed_at_unix: unix_now(),
    })
}

#[cfg(target_os = "windows")]
fn wide_null_from_os_str(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
fn wide_null_from_str(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
fn shell_execute_file_verb(path: &Path, verb: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb_wide = wide_null_from_str(verb);
    let path_wide = wide_null_from_os_str(path.as_os_str());

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb_wide.as_ptr(),
            path_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    let result_code = result as isize;
    if result_code <= 32 {
        return Err(format!(
            "Windows shell action '{}' failed for {}. ShellExecuteW returned {}.",
            verb,
            path.display(),
            result_code
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_desktop_wallpaper(path: &Path) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW,
        SPI_SETDESKWALLPAPER,
        SPIF_SENDCHANGE,
        SPIF_UPDATEINIFILE,
    };

    let mut path_wide = wide_null_from_os_str(path.as_os_str());

    let result = unsafe {
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            path_wide.as_mut_ptr().cast(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };

    if result == 0 {
        return Err(format!("Windows refused to set desktop wallpaper: {}", path.display()));
    }

    Ok(())
}
