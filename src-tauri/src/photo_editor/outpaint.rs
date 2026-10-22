//! The optional working-coordinate rectangle is the single editable outpaint geometry.
use super::contracts::Draft;
use serde::{Deserialize,Serialize};
use serde_json::json;

#[derive(Clone,Copy,Debug,PartialEq,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {pub version:u8,pub x:i64,pub y:i64,pub width:i64,pub height:i64}
impl Rect {
 pub fn read(d:&Draft,size:(u32,u32))->Result<Self,String>{
  let r=if let Some(value)=d.form_values.get("__outpaintFrame"){
   serde_json::from_value::<Self>(value.clone()).map_err(|_|"Проверьте рамку расширения.")?
  }else{
   let pad=512i64.saturating_sub(i64::from(size.1)).max(0);let mut sides=[128,128,pad/2,pad-pad/2];
   for(i,key)in["__outpaintLeft","__outpaintRight","__outpaintTop","__outpaintBottom"].iter().enumerate(){if let Some(v)=d.form_values.get(*key){sides[i]=v.as_i64().filter(|v|(0..=1024).contains(v)).ok_or("Поля расширения должны быть целыми числами от 0 до 1024.")?;}}
   Self{version:1,x:-sides[0],y:-sides[2],width:i64::from(size.0)+sides[0]+sides[1],height:i64::from(size.1)+sides[2]+sides[3]}
  };
  if r.version!=1||r.width<1||r.height<1||[r.x,r.y,r.width,r.height].iter().any(|v|v.unsigned_abs()>32_000_000){return Err("Недопустимая рамка расширения.".into());}Ok(r)
 }
 pub fn crop(self,size:(u32,u32))->Result<(u32,u32,u32,u32),String>{
  let x=self.x.max(0);let y=self.y.max(0);let w=(i64::from(size.0).min(self.x+self.width)-x).max(0);let h=(i64::from(size.1).min(self.y+self.height)-y).max(0);
  if w==0||h==0{return Err("Рамка должна пересекать исходную фотографию.".into());}Ok((x as u32,y as u32,w as u32,h as u32))
 }
 pub fn margins(self,size:(u32,u32))->[i64;4]{[-self.x,self.x+self.width-i64::from(size.0),-self.y,self.y+self.height-i64::from(size.1)]}
 pub fn expansion(self,size:(u32,u32))->Result<[u64;4],String>{
  if self.crop(size)?!=(0,0,size.0,size.1){return Err("Сначала примените предварительную обрезку исходника.".into());}
  let sides=self.margins(size);
  if !(512..=2048).contains(&self.width)||!(512..=2048).contains(&self.height)||sides.iter().any(|v|!(0..=1024).contains(v))||sides.iter().sum::<i64>()==0{return Err("Расширение требует итоговый размер от 512 до 2048 px и непустые поля.".into());}
  Ok(sides.map(|v|v as u64))
 }
}

pub fn continue_crop(d:&Draft,size:(u32,u32),crop:(u32,u32,u32,u32),asset:&str)->Result<Draft,String>{
 if d.operation!="outpaint"{return Err("Продолжение расширения доступно только для инструмента расширения.".into());}
 let r=Rect::read(d,size)?;let expected=r.crop(size)?;
 if crop!=expected||crop==(0,0,size.0,size.1)||r.width>2048||r.height>2048||r.margins(size).iter().any(|v|*v>1024){return Err("Предварительная обрезка не соответствует рамке расширения.".into());}
 let mut next=d.new_frame(asset);
 let r=Rect{x:r.x-i64::from(crop.0),y:r.y-i64::from(crop.1),..r};
 next.form_values.insert("__outpaintFrame".into(),json!(r));
 if let Some(flag)=d.form_values.get("__outpaintAutoCrop"){next.form_values.insert("__outpaintAutoCrop".into(),flag.clone());}
 next.prompt=super::instructions::plain(d)?;
 next.viewport.zoom=d.viewport.zoom;
 next.viewport.pan_x=d.viewport.pan_x+d.viewport.zoom*(f64::from(crop.0)+f64::from(crop.2)/2.-f64::from(size.0)/2.);
 next.viewport.pan_y=d.viewport.pan_y+d.viewport.zoom*(f64::from(crop.1)+f64::from(crop.3)/2.-f64::from(size.1)/2.);
 // Outpaint routes do not accept general references. Parent step retains all attachments/markup.
 next.validate()?;Ok(next)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn legacy_and_rectangle_are_the_same_contract(){let mut d=Draft::initial("a");d.operation="outpaint".into();let r=Rect::read(&d,(640,480)).unwrap();assert_eq!(r,Rect{version:1,x:-128,y:-16,width:896,height:512});assert_eq!(r.expansion((640,480)).unwrap(),[128,128,16,16]);d.form_values.insert("__outpaintFrame".into(),json!(r));assert_eq!(Rect::read(&d,(640,480)).unwrap(),r);}
 #[test]fn mixed_crop_continues_with_target_instruction_viewport_and_clean_coordinates(){
  let mut d=Draft::initial("old");d.operation="outpaint".into();d.prompt="Continue scene".into();d.viewport.zoom=0.5;d.viewport.pan_x=17.;d.viewport.pan_y=-7.;d.working_turn=1;d.mirrored=true;d.form_values.insert("__outpaintAutoCrop".into(),json!(true));d.form_values.insert("__outpaintFrame".into(),json!({"version":1,"x":-200,"y":100,"width":2000,"height":2000}));
  let r=Rect::read(&d,(1600,2400)).unwrap();assert!(r.expansion((1600,2400)).is_err());let next=continue_crop(&d,(1600,2400),(0,100,1600,2000),"new").unwrap();assert_eq!(next.prompt,d.prompt);assert_eq!(next.asset_id,"new");assert_eq!(next.working_turn,0);assert!(!next.mirrored);assert_eq!(next.viewport.pan_x,17.);assert_eq!(next.viewport.pan_y,-57.);assert_eq!(Rect::read(&next,(1600,2000)).unwrap().expansion((1600,2000)).unwrap(),[200,200,0,0]);assert_eq!(next.form_values["__outpaintAutoCrop"],json!(true));assert!(next.annotations.is_empty());assert!(next.selected_mask_ids.is_empty());assert_eq!(d.asset_id,"old");
 }
 #[test]fn pure_crop_is_local_and_invalid_continuations_are_rejected(){let mut d=Draft::initial("a");d.operation="outpaint".into();d.form_values.insert("__outpaintFrame".into(),json!({"version":1,"x":100,"y":50,"width":600,"height":400}));let next=continue_crop(&d,(800,600),(100,50,600,400),"b").unwrap();assert!(Rect::read(&next,(600,400)).unwrap().expansion((600,400)).is_err());assert!(continue_crop(&d,(800,600),(0,0,600,400),"b").is_err());d.operation="recolor".into();assert!(continue_crop(&d,(800,600),(100,50,600,400),"b").is_err());}
 #[test]fn malformed_overflow_and_empty_frames_cannot_reach_image_operations(){let mut d=Draft::initial("a");for value in [json!({"version":1,"x":i64::MAX,"y":0,"width":800,"height":800}),json!({"version":1,"x":0,"y":0,"width":0,"height":800}),json!({"version":2,"x":0,"y":0,"width":800,"height":800})]{d.form_values.insert("__outpaintFrame".into(),value);assert!(Rect::read(&d,(800,600)).is_err());}let r=Rect{version:1,x:900,y:0,width:800,height:800};assert!(r.crop((800,600)).is_err());}
}
