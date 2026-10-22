// Copy this snippet into src-tauri/src/lib.rs.
// Then add choose_save_image_path and save_resized_image to tauri::generate_handler![...].

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChooseSaveImagePathInputDto {
    source_path: String,
    default_file_name: String,
    format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChooseSaveImagePathResultDto {
    selected_path: Option<String>,
    opened_at_unix: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveResizedImageInputDto {
    source_path: String,
    target_path: String,
    width: u32,
    height: u32,
    quality: u8,
    format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResizedImageResultDto {
    target_path: String,
    file_name: String,
    width: u32,
    height: u32,
    size_bytes: u64,
    format: String,
    saved_at_unix: u64,
}

#[tauri::command]
async fn choose_save_image_path(input: ChooseSaveImagePathInputDto) -> Result<ChooseSaveImagePathResultDto, String> {
    let mut dialog = rfd::AsyncFileDialog::new();

    let source = PathBuf::from(input.source_path.trim());
    if let Some(parent) = source.parent() {
        if parent.exists() && parent.is_dir() {
            dialog = dialog.set_directory(parent);
        }
    }

    let format = input.format.trim().to_lowercase();
    let extension = match format.as_str() {
        "jpeg" | "jpg" => "jpg",
        "png" => "png",
        "tiff" | "tif" => "tiff",
        "bmp" => "bmp",
        _ => "jpg",
    };

    let default_name = input.default_file_name.trim();
    let file_name = if default_name.is_empty() {
        format!("resized-image.{extension}")
    } else {
        default_name.to_string()
    };

    dialog = dialog.set_file_name(file_name);

    dialog = match extension {
        "jpg" => dialog.add_filter("JPEG image", &["jpg", "jpeg"]),
        "png" => dialog.add_filter("PNG image", &["png"]),
        "tiff" => dialog.add_filter("TIFF image", &["tif", "tiff"]),
        "bmp" => dialog.add_filter("BMP image", &["bmp"]),
        _ => dialog,
    };

    Ok(ChooseSaveImagePathResultDto {
        selected_path: dialog.save_file().await.map(|file| path_to_string(file.path())),
        opened_at_unix: unix_now(),
    })
}

#[tauri::command]
async fn save_resized_image(input: SaveResizedImageInputDto) -> Result<SaveResizedImageResultDto, String> {
    tauri::async_runtime::spawn_blocking(move || save_resized_image_internal(input))
        .await.map_err(|error| format!("Resize worker failed: {error}"))?
}

fn save_resized_image_internal(input: SaveResizedImageInputDto) -> Result<SaveResizedImageResultDto, String> {
    use std::io::Write;
    let _write_guard = file_safety::IMAGE_WRITE_LOCK.lock().map_err(|_| "Image writer lock failed.".to_string())?;
    let source_path = validate_existing_image_file(&input.source_path)?;
    let target_path = PathBuf::from(input.target_path.trim());
    if input.target_path.trim().is_empty() { return Err("Target image path is empty.".to_string()); }
    let width = input.width;
    let height = input.height;
    if width == 0 || height == 0 || width > 32768 || height > 32768
        || u64::from(width) * u64::from(height) > file_safety::MAX_IMAGE_PIXELS {
        return Err("Output dimensions exceed the memory limit.".to_string());
    }
    let format = input.format.trim().to_lowercase();
    let output_format = match format.as_str() {
        "jpeg" | "jpg" => image::ImageFormat::Jpeg,
        "png" => image::ImageFormat::Png,
        "tiff" | "tif" => image::ImageFormat::Tiff,
        "bmp" => image::ImageFormat::Bmp,
        _ => return Err(format!("Unsupported resize output format: {}", input.format)),
    };
    if fs::symlink_metadata(&target_path).is_ok() {
        return Err("The output already exists. Choose a new filename; existing files are protected.".to_string());
    }
    let expected_source = compute_file_sha256(&source_path)?;
    let image = file_safety::decode_bounded(&source_path)?;
    let resized = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    drop(image);
    file_safety::atomic_save(&target_path, |output, temporary| {
        {
            let mut writer = std::io::BufWriter::new(output);
            if output_format == image::ImageFormat::Jpeg {
                let rgb_image = image::DynamicImage::ImageRgb8(resized.to_rgb8());
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, input.quality.clamp(1, 100))
                    .encode_image(&rgb_image).map_err(|error| format!("Cannot encode JPEG: {error}"))?;
            } else {
                resized.write_to(&mut writer, output_format).map_err(|error| format!("Cannot encode image: {error}"))?;
            }
            writer.flush().map_err(|error| format!("Cannot flush encoded image: {error}"))?;
        }
        let dimensions = image::ImageReader::open(temporary).map_err(|error| error.to_string())?
            .with_guessed_format().map_err(|error| error.to_string())?
            .into_dimensions().map_err(|error| error.to_string())?;
        if dimensions != (width, height) { return Err("Saved image dimensions did not match.".to_string()); }
        if compute_file_sha256(&source_path)? != expected_source {
            return Err("Source changed while resizing. The output was not published.".to_string());
        }
        Ok(())
    })?;
    let metadata = fs::metadata(&target_path).map_err(|error| format!("Cannot read saved image metadata: {error}"))?;
    Ok(SaveResizedImageResultDto {
        target_path: path_to_string(&target_path),
        file_name: viewer_file_name(&target_path, "resized-image"),
        width, height, size_bytes: metadata.len(), format, saved_at_unix: unix_now(),
    })
}
