use crate::*;
use super::{contracts::Draft,geometry,local,store};
use serde::Deserialize;
use serde_json::{json,Value};
#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]pub struct Stroke {pub points:Vec<Position>,pub radius:f64,pub erase:bool}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]pub struct Position {pub x:f64,pub y:f64}
pub fn save(app:&tauri::AppHandle,project:&str,revision:u64,mut draft:Draft,annotation_id:String,strokes:Vec<Stroke>)->Result<Value,String>{
 draft.validate()?;store::require_mutable(&store::connection(app)?,project)?;if annotation_id.is_empty()||annotation_id.len()>160||strokes.len()>512||strokes.iter().map(|s|s.points.len()).sum::<usize>()>20000{return Err("Маска превышает допустимые пределы.".into());}
 let asset=local::project_asset(app,project,&draft.asset_id)?;let w=asset["width"].as_u64().ok_or("Нет ширины кадра")? as u32;let h=asset["height"].as_u64().ok_or("Нет высоты кадра")? as u32;
 if u64::from(w)*u64::from(h)>32_000_000{return Err("Маска превышает предел памяти.".into());}
 let existing=draft.annotations.iter().find(|a|a["id"].as_str()==Some(&annotation_id));
 let mut mask=if let Some(a)=existing{if a["kind"]!="mask"{return Err("Выберите растровую маску.".into());}let source=local::project_asset(app,project,a["maskAssetId"].as_str().ok_or("Неверная маска")?)?;let path=store::owned_path(app,source["relativePath"].as_str().ok_or("Неверный mask path")?)?;let image=file_safety::decode_bounded(&path)?.into_luma8();if image.dimensions()!=(w,h){return Err("Маска имеет другую координатную базу.".into());}image}else{image::GrayImage::new(w,h)};
 for stroke in &strokes{
  if !stroke.radius.is_finite()||stroke.radius<0.5||stroke.radius>512.||stroke.points.len()<1{return Err("Неверный размер кисти.".into());}
  for p in &stroke.points{geometry::point(&json!({"x":p.x,"y":p.y}))?;}
  let color=if stroke.erase{0}else{255};
  for pair in stroke.points.windows(2){let a=&pair[0];let b=&pair[1];let dx=(b.x-a.x)*w as f64;let dy=(b.y-a.y)*h as f64;let length=dx.hypot(dy);let count=(length/(stroke.radius*0.45).max(0.5)).ceil() as u32;for n in 0..=count{let t=if count==0{0.}else{n as f64/count as f64};disc(&mut mask,(a.x+(b.x-a.x)*t)*w as f64,(a.y+(b.y-a.y)*t)*h as f64,stroke.radius,color);}}
  if stroke.points.len()==1{let p=&stroke.points[0];disc(&mut mask,p.x*w as f64,p.y*h as f64,stroke.radius,color);}
 }
 // A zero raster is a valid application: remove its annotation, retaining assets for history.
 // Decode/storage errors above remain errors and cannot be mistaken for an empty mask.
 let saved=if mask.pixels().any(|p|p[0]!=0){Some(local::save_image(app,&image::DynamicImage::ImageLuma8(mask),"mask")?)}else{None};
 apply_mask_annotation(&mut draft,&annotation_id,saved.as_ref());
 let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;store::require_mutable(&tx,project)?;
 let current:u64=tx.query_row("SELECT revision FROM editor_projects WHERE id=?1",[project],|r|r.get(0)).map_err(|e|e.to_string())?;if current!=revision{return Err("Черновик изменился при подготовке маски. Повторите применение.".into());}
 if let Some(saved)=saved{tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'mask')",params![project,saved["id"].as_str()]).map_err(|e|e.to_string())?;}
 draft.form_values.remove("__maskStrokes");draft.form_values.remove("__maskAnnotationId");draft.validate()?;
 tx.execute("UPDATE editor_projects SET draft=?2,revision=revision+1,updated_at_unix=?3 WHERE id=?1",params![project,serde_json::to_string(&draft).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;store::stash_step_draft(&tx,project,&draft)?;tx.commit().map_err(|e|e.to_string())?;store::project_snapshot(app,project)
}
fn apply_mask_annotation(draft:&mut Draft,id:&str,saved:Option<&Value>){
 let existing=draft.annotations.iter().find(|a|a["id"].as_str()==Some(id)).cloned();
 if saved.is_some()||existing.is_some(){draft.push_annotation_undo();}
 draft.annotations.retain(|a|a["id"].as_str()!=Some(id));
 if let Some(saved)=saved{
  let version=existing.as_ref().and_then(|a|a["version"].as_u64()).unwrap_or(0)+1;
  let mut annotation=existing.unwrap_or_else(||json!({"id":id,"kind":"mask","assetId":draft.asset_id,"instruction":"","visible":true,"included":true}));
  annotation["maskAssetId"]=saved["id"].clone();annotation["version"]=json!(version);draft.annotations.push(annotation);
  draft.selected_annotation_id=Some(id.to_string());if !draft.selected_mask_ids.iter().any(|v|v==id){draft.selected_mask_ids.push(id.to_string());}
 }else{
  if draft.selected_annotation_id.as_deref()==Some(id){draft.selected_annotation_id=None;}
  draft.selected_mask_ids.retain(|v|v!=id);draft.annotation_references.remove(id);
 }
}
fn disc(image:&mut image::GrayImage,x:f64,y:f64,r:f64,color:u8){let left=(x-r).floor().max(0.) as u32;let top=(y-r).floor().max(0.) as u32;let right=(x+r).ceil().min(image.width() as f64) as u32;let bottom=(y+r).ceil().min(image.height() as f64) as u32;for v in top..bottom{for u in left..right{if (u as f64+0.5-x).powi(2)+(v as f64+0.5-y).powi(2)<=r*r{image.put_pixel(u,v,image::Luma([color]));}}}}
#[cfg(test)]mod tests{use super::*;#[test]fn brush_and_eraser_modify_only_mask(){let mut m=image::GrayImage::new(12,8);disc(&mut m,6.,4.,3.,255);assert_eq!(m.get_pixel(6,4)[0],255);assert_eq!(m.get_pixel(0,0)[0],0);disc(&mut m,6.,4.,1.,0);assert_eq!(m.get_pixel(6,4)[0],0);assert_eq!(m.get_pixel(4,4)[0],255);}}

#[cfg(test)]mod application_tests{
 use super::*;
 #[test]fn empty_application_removes_only_its_mask_and_preserves_undo_asset(){
  let mut d=Draft::initial("image");
  apply_mask_annotation(&mut d,"mask-a",Some(&json!({"id":"pixels-a"})));
  apply_mask_annotation(&mut d,"mask-b",Some(&json!({"id":"pixels-b"})));
  let other=d.annotations[1].clone();d.selected_annotation_id=Some("mask-a".into());
  apply_mask_annotation(&mut d,"mask-a",None);
  assert_eq!(d.annotations,vec![other]);assert_eq!(d.selected_annotation_id,None);
  assert_eq!(d.selected_mask_ids,vec!["mask-b"]);
  assert!(d.edit_history.undo.last().unwrap().annotations.iter().any(|a|a["maskAssetId"]=="pixels-a"));
  d.validate().unwrap();
 }
 #[test]fn never_applied_empty_mask_creates_no_annotation_or_phantom_undo(){
  let mut d=Draft::initial("image");apply_mask_annotation(&mut d,"empty",None);
  assert!(d.annotations.is_empty());assert!(d.selected_mask_ids.is_empty());assert!(d.edit_history.undo.is_empty());d.validate().unwrap();
 }
 #[test]fn one_nonzero_pixel_is_not_empty_and_partial_erase_is_preserved(){
  let mut raster=image::GrayImage::new(8,8);raster.put_pixel(7,7,image::Luma([1]));
  assert!(raster.pixels().any(|p|p[0]!=0));disc(&mut raster,2.,2.,1.,0);assert!(raster.pixels().any(|p|p[0]!=0));
  disc(&mut raster,7.5,7.5,1.,0);assert!(!raster.pixels().any(|p|p[0]!=0));
 }
}
