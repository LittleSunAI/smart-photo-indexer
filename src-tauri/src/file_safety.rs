use super::*;

pub(super) const MAX_IMAGE_PIXELS: u64 = 32_000_000;
pub(super) static IMAGE_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub(super) fn image_worker()->std::sync::Arc<tokio::sync::Semaphore>{
    static WORKER:std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>>=std::sync::OnceLock::new();
    WORKER.get_or_init(||std::sync::Arc::new(tokio::sync::Semaphore::new(1))).clone()
}

pub(super) fn decode_bounded(path: &Path) -> Result<image::DynamicImage, String> {
    let extension=path.extension().map(|value|value.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ["heic","heif","avif"].contains(&extension.as_str()) {return Err(format!("Формат {extension} не декодируется этой сборкой. Сохраните отдельную копию в JPEG или PNG; оригинал не изменён."));}
    let dimensions = image::ImageReader::open(path).map_err(|error| error.to_string())?
        .with_guessed_format().map_err(|error| error.to_string())?
        .into_dimensions().map_err(|error| format!("Cannot read image dimensions: {error}"))?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > MAX_IMAGE_PIXELS {
        return Err("Image is too large for the current memory limit.".to_string());
    }
    let mut reader = image::ImageReader::open(path).map_err(|error| error.to_string())?
        .with_guessed_format().map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    reader.limits(limits);
    use image::ImageDecoder;
    let mut decoder=reader.into_decoder().map_err(|error|format!("Cannot create bounded image decoder: {error}"))?;
    let orientation=decoder.orientation().map_err(|error|format!("Cannot read image orientation: {error}"))?;
    let profile=decoder.icc_profile().map_err(|error|format!("Не удалось прочитать цветовой профиль: {error}"))?;
    if profile.as_ref().is_some_and(|bytes|bytes.len()>2*1024*1024){return Err("Цветовой профиль превышает предел 2 МБ; изображение не обработано.".into());}
    let mut image=image::DynamicImage::from_decoder(decoder).map_err(|error|format!("Cannot decode image within memory limits: {error}"))?;
    if let Some(profile)=profile{convert_rgb_profile(&mut image,&profile)?;}
    image.apply_orientation(orientation);
    Ok(image)
}

fn convert_rgb_profile(image:&mut image::DynamicImage,bytes:&[u8])->Result<(),String>{
    use moxcms::{ColorProfile,DataColorSpace,Layout,TransformOptions};
    let profile=ColorProfile::new_from_slice(bytes).map_err(|e|format!("Неверный цветовой профиль: {e}"))?;
    if profile.color_space!=DataColorSpace::Rgb{return Err("Этот цветовой профиль не RGB; требуется отдельная проверенная конвертация. Оригинал не изменён.".into());}
    let srgb=ColorProfile::new_srgb();
    match image{
        image::DynamicImage::ImageRgb8(buffer)=>{let transform=profile.create_in_place_transform_8bit(Layout::Rgb,&srgb,TransformOptions::default()).map_err(|e|e.to_string())?;transform.transform(buffer.as_mut()).map_err(|e|e.to_string())?;},
        image::DynamicImage::ImageRgba8(buffer)=>{let transform=profile.create_in_place_transform_8bit(Layout::Rgba,&srgb,TransformOptions::default()).map_err(|e|e.to_string())?;transform.transform(buffer.as_mut()).map_err(|e|e.to_string())?;},
        image::DynamicImage::ImageRgb16(buffer)=>{let transform=profile.create_in_place_transform_16bit(Layout::Rgb,&srgb,TransformOptions::default()).map_err(|e|e.to_string())?;transform.transform(buffer.as_mut()).map_err(|e|e.to_string())?;},
        image::DynamicImage::ImageRgba16(buffer)=>{let transform=profile.create_in_place_transform_16bit(Layout::Rgba,&srgb,TransformOptions::default()).map_err(|e|e.to_string())?;transform.transform(buffer.as_mut()).map_err(|e|e.to_string())?;},
        _=>return Err("Тип пикселей не соответствует RGB-профилю; изображение не преобразовано.".into()),
    }Ok(())
}

pub(super) fn move_no_replace(source: &Path, target: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        // Deliberately omit MOVEFILE_REPLACE_EXISTING and cross-volume copy fallback.
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(format!("Cannot move without replacing a file: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        fs::hard_link(source, target).map_err(|error| format!("Cannot publish without replacing a file: {error}"))?;
        fs::remove_file(source).map_err(|error| format!("File published but temporary/source cleanup failed: {error}"))
    }
}

pub(super) fn atomic_save(
    target: &Path,
    write: impl FnOnce(&mut fs::File, &Path) -> Result<(), String>,
) -> Result<(), String> {
    if !target.is_absolute() { return Err("Choose an absolute output path.".to_string()); }
    if fs::symlink_metadata(target).is_ok() {
        return Err("The output already exists. Choose a new filename; existing files are protected.".to_string());
    }
    let parent = target.parent().ok_or("Output directory is unavailable.")?;
    fs::create_dir_all(parent).map_err(|error| format!("Cannot create output directory: {error}"))?;
    let parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
    let destination = parent.join(target.file_name().ok_or("Output filename is unavailable.")?);
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_nanos();
    let temporary = parent.join(format!(".smart-photo-{}-{nonce}.partial", std::process::id()));
    let mut output = fs::OpenOptions::new().write(true).read(true).create_new(true).open(&temporary)
        .map_err(|error| format!("Cannot create temporary output: {error}"))?;
    let result = (|| {
        write(&mut output, &temporary)?;
        output.sync_all().map_err(|error| format!("Cannot flush image to disk: {error}"))?;
        Ok(())
    })();
    drop(output);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = move_no_replace(&temporary, &destination) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}
