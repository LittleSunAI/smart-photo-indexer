use crate::*;
use super::{contracts::Draft,geometry,store};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
#[derive(Clone,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct LocalOperation {pub id:String,pub kind:String,pub x:u32,pub y:u32,pub width:u32,pub height:u32,#[serde(default)]pub continue_outpaint:bool}
fn budget(source:(u32,u32),target:(u32,u32))->Result<(),String>{
    let pixels=|s:(u32,u32)|u64::from(s.0)*u64::from(s.1);
    if source.0==0||source.1==0||target.0==0||target.1==0||pixels(target)>32_000_000||pixels(source)*8+pixels(target)*8+32*1024*1024>384*1024*1024{return Err("Операция превышает предел 32 Мп или бюджет памяти 384 МиБ. Исходник не изменён.".into());}Ok(())
}
pub fn project_asset(app:&tauri::AppHandle,project:&str,id:&str)->Result<Value,String>{
    let db=store::connection(app)?;let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_assets WHERE project_id=?1 AND asset_id=?2)",params![project,id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if !exists{return Err("Кадр не принадлежит проекту.".into());}store::asset_value(&db,id)
}
/// Immutable content-addressed file + preview. A failed DB commit never exposes a ready frame.
pub fn save_image(app:&tauri::AppHandle,image:&image::DynamicImage,kind:&str)->Result<Value,String>{
    let mut encoded=std::io::Cursor::new(Vec::new());image.write_to(&mut encoded,image::ImageFormat::Png).map_err(|e|e.to_string())?;
    save_encoded(app,image,kind,encoded.into_inner(),"png","image/png")
}
pub(super) fn save_raw_image(app:&tauri::AppHandle,image:&image::DynamicImage,bytes:Vec<u8>)->Result<Value,String>{
    let (extension,content_type)=raw_format(&bytes)?;
    save_encoded(app,image,"image",bytes,extension,content_type)
}
pub(super) fn raw_format(bytes:&[u8])->Result<(&'static str,&'static str),String>{
    let format=image::guess_format(&bytes).map_err(|e|e.to_string())?;
    let (extension,content_type)=match format{image::ImageFormat::Jpeg=>("jpg","image/jpeg"),image::ImageFormat::WebP=>("webp","image/webp"),image::ImageFormat::Gif=>("gif","image/gif"),image::ImageFormat::Tiff=>("tiff","image/tiff"),image::ImageFormat::Bmp=>("bmp","image/bmp"),_=>("png","image/png")};
    Ok((extension,content_type))
}
fn save_encoded(app:&tauri::AppHandle,image:&image::DynamicImage,kind:&str,bytes:Vec<u8>,extension:&str,content_type:&str)->Result<Value,String>{
    let hash=format!("{:x}",Sha256::digest(&bytes));
    let db=store::connection(app)?;let existing:Option<String>=db.query_row("SELECT id FROM editor_assets WHERE sha256=?1 AND kind=?2 LIMIT 1",params![hash,kind],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if let Some(existing)=existing{
        let asset=store::asset_value(&db,&existing)?;let relative=asset["relativePath"].as_str().ok_or("Неверный asset path")?;let path=store::owned_path(app,relative)?;
        if !path.exists(){let collected:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_gc_targets WHERE kind='asset' AND owner_id=?1 AND relative_path=?2 AND expected_sha256=?3 AND state IN ('planned','collected'))",params![existing,relative,hash],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !collected{return Err("Файл кадра отсутствует без записи штатной очистки. Автоматическая замена запрещена.".into());}
            file_safety::atomic_save(&path,|out,_|std::io::Write::write_all(out,&bytes).map_err(|e|e.to_string()))?;
            db.execute("UPDATE editor_gc_targets SET state='restored',updated_at_unix=?2 WHERE kind='asset' AND owner_id=?1",params![existing,unix_now()]).map_err(|e|e.to_string())?;
        }
        if compute_file_sha256(&path)?!=hash{return Err("Существующий owned-файл повреждён; замена запрещена.".into());}
        let preview=store::owned_path(app,asset["previewPath"].as_str().ok_or("Нет preview")?)?;
        if !preview.exists(){let thumb=image.resize(1600,1250,image::imageops::FilterType::Triangle);file_safety::atomic_save(&preview,|out,_|thumb.write_to(out,image::ImageFormat::Png).map_err(|e|e.to_string()))?;}
        return Ok(asset);
    }
    drop(db);
    let folder=if kind=="mask"{"masks"}else{"assets"};let relative=format!("{folder}/{hash}.{extension}");let preview=format!("previews/{hash}.png");let id=format!("editor-{kind}-{hash}");
    let path=store::owned_path(app,&relative)?;let preview_path=store::owned_path(app,&preview)?;
    let db=store::connection(app).map_err(|e|format!("Подготовка базы кадра: {e}"))?;let op=format!("image-{hash}");
    db.execute("INSERT OR IGNORE INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES(?1,?2,'planned',?3,?4)",params![op,relative,json!({"sha256":hash,"preview":preview}).to_string(),unix_now()]).map_err(|e|e.to_string())?;
    drop(db);
    if !path.exists(){file_safety::atomic_save(&path,|out,_|{std::io::Write::write_all(out,&bytes).map_err(|e|e.to_string())})?;}
    if compute_file_sha256(&path)?!=hash{return Err("Owned-файл повреждён; результат не зарегистрирован.".into());}
    drop(bytes);if !preview_path.exists(){let thumb=image.resize(1600,1250,image::imageops::FilterType::Triangle);file_safety::atomic_save(&preview_path,|out,_|thumb.write_to(out,image::ImageFormat::Png).map_err(|e|e.to_string()))?;}
    let length=fs::metadata(&path).map_err(|e|e.to_string())?.len();
    let db=store::connection(app).map_err(|e|format!("База регистрации кадра: {e}"))?;
    db.execute("INSERT OR IGNORE INTO editor_assets(id,kind,relative_path,preview_path,sha256,width,height,bytes,content_type) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,kind,relative,preview,hash,image.width(),image.height(),length,content_type]).map_err(|e|format!("Регистрация кадра: {e}"))?;
    db.execute("UPDATE editor_file_operations SET state='fileSaved' WHERE id=?1",[op]).map_err(|e|e.to_string())?;store::asset_value(&db,&id)
}
pub fn apply(app:&tauri::AppHandle,project:&str,revision:u64,draft:Draft,op:LocalOperation)->Result<Value,String>{
    draft.validate()?;store::require_mutable(&store::connection(app)?,project)?;
    if op.id.len()<8||op.id.len()>100||!op.id.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-')||!["crop","resize","flip-horizontal","flip-vertical"].contains(&op.kind.as_str()){return Err("Неверная локальная операция.".into());}
    if op.continue_outpaint&&op.kind!="crop"{return Err("Продолжение расширения требует локальную обрезку.".into());}
    let step_id=format!("{project}-{}",op.id);
    let db=store::connection(app)?;
    if db.query_row("SELECT EXISTS(SELECT 1 FROM editor_steps WHERE id=?1 AND project_id=?2)",params![step_id,project],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?{return store::project_snapshot(app,project);}
    let current:u64=db.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
    if current!=revision{return Err("Черновик изменился до начала операции.".into());}
    let asset=project_asset(app,project,&draft.asset_id)?;let raw=(asset["width"].as_u64().unwrap_or(0) as u32,asset["height"].as_u64().unwrap_or(0) as u32);
    let size=if draft.working_turn%2==0{raw}else{(raw.1,raw.0)};
    let target=if ["crop","resize"].contains(&op.kind.as_str()){(op.width,op.height)}else{size};budget(raw,target)?;
    if op.kind=="crop"&&(op.x.checked_add(op.width).is_none_or(|n|n>size.0)||op.y.checked_add(op.height).is_none_or(|n|n>size.1)){return Err("Обрезка выходит за пределы кадра.".into());}
    if op.continue_outpaint{super::outpaint::continue_crop(&draft,size,(op.x,op.y,op.width,op.height),&draft.asset_id)?;}
    let path=store::owned_path(app,asset["relativePath"].as_str().ok_or("Неверный owned asset")?)?;
    if compute_file_sha256(&path)?!=asset["sha256"].as_str().unwrap_or(""){return Err("Исходный кадр проекта повреждён.".into());}
    let image=geometry::orient_image(file_safety::decode_bounded(&path)?,draft.working_turn,draft.mirrored);
    let result=transform_image(image,&op);let saved=save_image(app,&result,"image")?;drop(result);
    let asset_id=saved["id"].as_str().ok_or("Неверный result")?;
    let next=if op.continue_outpaint{super::outpaint::continue_crop(&draft,size,(op.x,op.y,op.width,op.height),asset_id)?}else{draft.new_frame(asset_id)};next.validate()?;
    let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
    let now_revision:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
    if now_revision!=revision{return Err("Черновик изменился во время операции. Готовый файл не заменил текущий кадр.".into());}
    let parent:String=tx.query_row("SELECT s.id FROM editor_steps s JOIN editor_projects p ON p.active_step_id=s.id WHERE p.id=?1 AND s.asset_id=?2",params![project,draft.asset_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    store::stash_step_draft(&tx,project,&draft)?;
    let title=match op.kind.as_str(){"crop"=>"Обрезка","resize"=>"Изменение размера","flip-horizontal"=>"Отражение по горизонтали",_=>"Отражение по вертикали"};
    tx.execute("INSERT INTO editor_steps(id,project_id,asset_id,operation,title,parameters,annotations,created_at_unix) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![step_id,project,asset_id,if op.kind.starts_with("flip"){"flip"}else{op.kind.as_str()},title,serde_json::to_string(&op).map_err(|e|e.to_string())?,serde_json::to_string(&next.annotations).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO editor_edges(step_id,parent_id) VALUES(?1,?2)",params![step_id,parent]).map_err(|e|e.to_string())?;
    for id in std::iter::once(asset_id.to_string()).chain(next.annotations.iter().filter_map(|a|a["maskAssetId"].as_str().map(String::from))){tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'result')",params![project,id]).map_err(|e|e.to_string())?;}
    tx.execute("UPDATE editor_projects SET active_step_id=?2,draft=?3,revision=revision+1,updated_at_unix=?4 WHERE id=?1",params![project,step_id,serde_json::to_string(&next).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;store::stash_step_draft(&tx,project,&next)?;tx.commit().map_err(|e|e.to_string())?;
    store::project_snapshot(app,project)
}
fn transform_image(image:image::DynamicImage,op:&LocalOperation)->image::DynamicImage{match op.kind.as_str(){"crop"=>image.crop_imm(op.x,op.y,op.width,op.height),"resize"=>image.resize_exact(op.width,op.height,image::imageops::FilterType::Lanczos3),"flip-horizontal"=>image.fliph(),_=>image.flipv()}}
#[cfg(test)]mod tests{use super::*;#[test]fn budget_rejects_before_allocation(){assert!(budget((3000,2000),(10000,10000)).is_err());assert!(budget((3000,2000),(1500,1000)).is_ok());assert!(budget((3000,2000),(0,1000)).is_err());}}
