use crate::*;
use serde_json::{json,Value};

pub const SCHEMA:&str=r#"
    ALTER TABLE images ADD COLUMN file_metadata TEXT;
    ALTER TABLE images ADD COLUMN date_taken_local TEXT;
    ALTER TABLE images ADD COLUMN date_taken_offset TEXT;
    ALTER TABLE images ADD COLUMN date_taken_unix INTEGER;
    ALTER TABLE image_paths ADD COLUMN size_bytes INTEGER;
    ALTER TABLE image_paths ADD COLUMN modified_at_unix INTEGER;
    CREATE INDEX idx_images_date_taken ON images(date_taken_local);
"#;

#[derive(Clone,Debug,Default,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ImageMetadata{
    pub date_taken_raw:Option<String>,pub date_taken_local:Option<String>,pub date_taken_offset:Option<String>,pub date_taken_unix:Option<i64>,
    pub orientation:Option<u32>,pub width:Option<u32>,pub height:Option<u32>,pub camera_make:Option<String>,pub camera_model:Option<String>,
    pub modified_at_unix:Option<u64>,pub size_bytes:Option<u64>,pub exif_state:String,
    pub color_profile:String,pub decode_support:String,pub frame_policy:String,
}
fn text(exif:&exif::Exif,tag:exif::Tag)->Option<String>{
    match &exif.get_field(tag,exif::In::PRIMARY)?.value{
        exif::Value::Ascii(values)=>std::str::from_utf8(values.first()?).ok().map(|s|s.trim_matches('\0').trim().chars().take(500).collect()).filter(|s:&String|!s.is_empty()),
        _=>None
    }
}
fn parse_date(raw:&str,offset:Option<&str>)->(Option<String>,Option<String>,Option<i64>){
    let date=match chrono::NaiveDateTime::parse_from_str(raw,"%Y:%m:%d %H:%M:%S"){Ok(date)=>date,Err(_)=>return(None,None,None)};
    let local=Some(date.format("%Y-%m-%dT%H:%M:%S").to_string());
    if let Some(offset)=offset{
        if let Ok(value)=chrono::DateTime::parse_from_str(&format!("{raw} {offset}"),"%Y:%m:%d %H:%M:%S %:z"){
            return(local,Some(offset.to_string()),Some(value.timestamp()));
        }
    }
    (local,None,None)
}
pub fn read(path:&Path)->ImageMetadata{
    let mut metadata=ImageMetadata{exif_state:"unavailable".into(),color_profile:"unknown".into(),decode_support:"unknown".into(),frame_policy:"wholeImage".into(),..Default::default()};
    let file=match fs::File::open(path){Ok(file)=>file,Err(_)=>return metadata};
    if let Ok(info)=file.metadata(){metadata.modified_at_unix=modified_at_unix(&info);metadata.size_bytes=Some(info.len());}
    // Metadata inspection never decodes pixels and never reads an unbounded archive file.
    let mut bytes=Vec::new();if file.take(2*1024*1024).read_to_end(&mut bytes).is_err(){return metadata;}
    let extension=path.extension().map(|value|value.to_string_lossy().to_lowercase()).unwrap_or_default();
    metadata.frame_policy=match extension.as_str(){"gif"=>"firstFrame","tif"|"tiff"=>"firstPage",_=>"wholeImage"}.into();
    if ["heic","heif","avif"].contains(&extension.as_str()){metadata.decode_support="unsupported".into();}
    if let Ok(reader)=image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format(){
        if let Ok((width,height))=reader.into_dimensions(){metadata.width=Some(width);metadata.height=Some(height);if metadata.decode_support!="unsupported"{metadata.decode_support="native".into();}}
    }
    if let Ok(mut reader)=image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format(){
        let mut limits=image::Limits::default();limits.max_alloc=Some(2*1024*1024);reader.limits(limits);
        if let Ok(mut decoder)=reader.into_decoder(){use image::ImageDecoder;metadata.color_profile=match decoder.icc_profile(){Ok(None)=>"absent",Ok(Some(profile)) if profile.len()>2*1024*1024=>"outsideLimit",Ok(Some(profile)) if profile.get(16..20)==Some(b"RGB ")=>"rgbToSrgb",Ok(Some(_))=>"unsupportedOrInvalid",Err(_)=>"unreadable"}.into();}
    }
    if metadata.decode_support=="unknown"{metadata.decode_support="invalidOrUnknown".into();}
    match exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&bytes)){
        Ok(exif)=>{
            metadata.exif_state="read".into();metadata.date_taken_raw=text(&exif,exif::Tag::DateTimeOriginal);
            let offset=text(&exif,exif::Tag::OffsetTimeOriginal);
            if let Some(raw)=metadata.date_taken_raw.as_ref(){(metadata.date_taken_local,metadata.date_taken_offset,metadata.date_taken_unix)=parse_date(raw,offset.as_deref());}
            metadata.orientation=exif.get_field(exif::Tag::Orientation,exif::In::PRIMARY).and_then(|f|f.value.get_uint(0)).filter(|v|(1..=8).contains(v));
            metadata.camera_make=text(&exif,exif::Tag::Make);metadata.camera_model=text(&exif,exif::Tag::Model);
            if metadata.orientation.is_some_and(|orientation|orientation>=5){std::mem::swap(&mut metadata.width,&mut metadata.height);}
        },
        Err(exif::Error::NotFound(_))=>metadata.exif_state="absent".into(),
        Err(_)=>metadata.exif_state="unreadableOrOutsideLimit".into(),
    }
    metadata
}
pub fn save(tx:&Connection,image_id:&str,path:&str,metadata:&ImageMetadata)->Result<(),String>{
    let value=serde_json::to_string(metadata).map_err(|e|e.to_string())?;
    tx.execute("UPDATE images SET file_metadata=?1,date_taken_local=?2,date_taken_offset=?3,date_taken_unix=?4,width=COALESCE(?5,width),height=COALESCE(?6,height) WHERE id=?7",params![value,metadata.date_taken_local,metadata.date_taken_offset,metadata.date_taken_unix,metadata.width,metadata.height,image_id]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE image_paths SET size_bytes=?1,modified_at_unix=?2 WHERE path=?3 AND image_id=?4",params![metadata.size_bytes.map(u64_to_i64_saturating),metadata.modified_at_unix.map(u64_to_i64_saturating),path,image_id]).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn stored(db:&Connection,image_id:&str)->Result<Option<Value>,String>{
    let text:Option<String>=db.query_row("SELECT file_metadata FROM images WHERE id=?1",[image_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    text.map(|text|serde_json::from_str(&text).map_err(|e|e.to_string())).transpose()
}
#[tauri::command]
pub async fn inspect_image_metadata(path:String)->Result<Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let path=Path::new(&path);if !path.is_absolute() || !path.is_file(){return Err("Фотография недоступна.".into());}
        Ok(json!(read(path)))
    }).await.map_err(|e|e.to_string())?
}
#[cfg(test)] mod tests{
    use super::*;
    #[test] fn local_capture_date_never_acquires_an_invented_timezone(){
        let (local,offset,unix)=parse_date("2024:02:29 12:34:56",None);assert_eq!(local.as_deref(),Some("2024-02-29T12:34:56"));assert_eq!(offset,None);assert_eq!(unix,None);
        let (_,offset,unix)=parse_date("2024:02:29 12:34:56",Some("+03:00"));assert_eq!(offset.as_deref(),Some("+03:00"));assert_eq!(unix,Some(1709199296));
        assert_eq!(parse_date("2023:02:29 12:34:56",None),(None,None,None));
        assert_eq!(parse_date("2024:02:29 12:34:56",Some("bad")).1,None);
    }
}
