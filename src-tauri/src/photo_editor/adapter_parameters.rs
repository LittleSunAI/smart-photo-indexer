//! Semantic constraints which the public field schemas do not express.
use serde_json::{json,Map,Value};
/// FLUX may round custom sizes. Use a 32-pixel grid while keeping source aspect.
pub fn aligned_edit_size(w:u64,h:u64)->Result<Value,String>{
 let ratio=w as f64/h.max(1) as f64;
 let scale=(1024.0/w.max(h) as f64).min(1.).max(512.0/w.min(h).max(1) as f64);
 let target=w as f64*h as f64*scale*scale;
 let mut best:Option<(f64,u64,u64)>=None;
 for width in (512u64..=2048).step_by(32){let height=((width as f64/ratio/32.).round() as u64)*32;if !(512..=2048).contains(&height){continue;}let error=((width as f64/height as f64)/ratio).ln().abs();if error>0.003{continue;}let score=error*10000.+(width as f64*height as f64/target).ln().abs();if best.is_none_or(|previous|score<previous.0){best=Some((score,width,height));}}
 best.map(|(_,width,height)|json!({"width":width,"height":height})).ok_or_else(||"Пропорции кадра не помещаются в размеры FLUX без искажения. Выберите другую модель или задайте размер явно.".into())
}
pub fn validate(route:&Value,fields:&mut Map<String,Value>,w:u64,h:u64,operation:&str,size:&Value)->Result<(),String>{
 let id=route["id"].as_str().unwrap_or("");
 if id.contains("seedream/"){
  let(sw,sh)=super::estimates::size(size,w,h);let pixels=sw.saturating_mul(sh);
  let(min,max)=if id.contains("/lite/"){(2560*1440,4096*4096)}else{(1024*1024,2048*2048)};
  if (size.is_object()&&!(min..=max).contains(&pixels)||!size.is_object()&&((pixels as f64)<min as f64*0.998||(pixels as f64)>max as f64*1.002))||!(1./16. ..=16.).contains(&(sw as f64/sh.max(1) as f64)){return Err("Размер Seedream вне допустимой площади или пропорций. Выберите auto либо подходящий свой размер; скрытое масштабирование не выполняется.".into());}
  if fields.contains_key("max_images"){fields.insert("max_images".into(),json!(1));}
 }
 if id=="fal-ai/image-apps-v2/photo-restoration"{
  let ratio=w as f64/h.max(1) as f64;let target=[("1:1",1.),("16:9",16./9.),("9:16",9./16.),("4:3",4./3.),("3:4",3./4.)].into_iter().find(|(_,r)|(ratio/r-1.).abs()<0.003).ok_or("Эта модель восстановления не сохраняет пропорции данного кадра. Выберите другой редактор; фото не обрезано.")?;
  fields.insert("aspect_ratio".into(),json!({"ratio":target.0}));
 }
 if id.starts_with("openai/gpt-image-2.5/"){
  let format=fields.get("output_format").and_then(Value::as_str).unwrap_or("png").to_string();
  if format=="png"{fields.remove("output_compression");}
  if format=="jpeg"&&fields.get("background")==Some(&json!("transparent")){return Err("Для прозрачного фона выберите PNG или WebP.".into());}
 }
 if id.starts_with("topaz/upscale/"){
  let model=fields.get("model").and_then(Value::as_str).unwrap_or("").to_string();
  if model!="Text Refine"{fields.remove("strength");}if model=="CGI"{fields.remove("fix_compression");}
  if fields.get("face_enhancement")!=Some(&json!(true)){fields.remove("face_enhancement_strength");fields.remove("face_enhancement_creativity");}
  if id.ends_with("generative")&&!model.starts_with("Wonder"){return Err("Этот профиль проверяет Wonder. Другие внутренние генеративные модели требуют своей настройки и тарифа.".into());}
 }
 if id=="topaz/restore/image"&&fields.get("model")!=Some(&json!("Dust-Scratch V2")){return Err("В этом профиле используется Dust-Scratch V2. Recover имеет другой тариф и пока не включён.".into());}
 if id=="fal-ai/iclight-v2"{fields.remove("background_threshold");if fields.get("enable_hr_fix")!=Some(&json!(true)){fields.remove("highres_denoise");}}
 if id=="fal-ai/birefnet/v2"&&fields.get("model")!=Some(&json!("Dynamic"))&&fields.get("operating_resolution")==Some(&json!("2304x2304")){return Err("Разрешение 2304 доступно только модели Dynamic.".into());}
 if id=="fal-ai/ideogram/v3/replace-background"&&fields.contains_key("style_codes"){fields.remove("style");fields.remove("style_preset");}
 if id=="fal-ai/flux-2/flash/edit"&&size.is_object(){let(sw,sh)=super::estimates::size(size,w,h);if !(512..=2048).contains(&sw)||!(512..=2048).contains(&sh){return Err(format!("FLUX Flash не может выдать {sw}×{sh}: каждая сторона должна быть от 512 до 2048 px. Выберите поддерживаемый размер явно или другую модель."));}}
 if let Some(schema)=route["inputSchema"]["properties"].get("image_size"){if let Some(size)=fields.get("image_size"){super::catalog::validate_value(schema,size,&route["definitions"],0)?;}}
 if operation=="outpaint"&&id=="fal-ai/bria/expand"{let area=(w*h) as f64/(size["width"].as_u64().unwrap_or(w)*size["height"].as_u64().unwrap_or(h)) as f64;if area<0.15{return Err("Для BRIA Expand исходник должен занимать не менее 15% полотна. Уменьшите поля.".into());}}
 Ok(())
}
#[cfg(test)]mod tests{use super::*;
 #[test]fn flux_sizes_keep_common_aspects_on_the_provider_grid(){for(w,h)in[(640,480),(480,640),(1920,1080),(1024,1024)]{let size=aligned_edit_size(w,h).unwrap();let x=size["width"].as_u64().unwrap();let y=size["height"].as_u64().unwrap();assert_eq!(x%32,0);assert_eq!(y%32,0);assert!(((x as f64/y as f64)/(w as f64/h as f64)-1.).abs()<0.003);}assert_eq!(aligned_edit_size(640,480).unwrap(),json!({"width":768,"height":576}));}
 #[test]fn unsupported_restoration_aspect_is_rejected_without_crop(){let r=json!({"id":"fal-ai/image-apps-v2/photo-restoration"});let mut f=Map::new();assert!(validate(&r,&mut f,1000,300,"restore",&Value::Null).is_err());validate(&r,&mut f,640,480,"restore",&Value::Null).unwrap();assert_eq!(f["aspect_ratio"],json!({"ratio":"4:3"}));}
 #[test]fn png_never_sends_inapplicable_compression(){let r=json!({"id":"openai/gpt-image-2.5/flare/edit"});let mut f=json!({"output_format":"png","output_compression":80}).as_object().unwrap().clone();validate(&r,&mut f,512,512,"universal-edit",&Value::Null).unwrap();assert!(!f.contains_key("output_compression"));}
}
