//! Versioned spatial compilation. Descriptors are immutable plan data, never fake assets.
use super::{contracts::Draft,geometry,instructions};
use serde_json::{json,Value};

pub const VERSION:u32=1;
pub fn enabled()->bool{std::env::var("SMART_PHOTO_EDITOR_RECT_MASK").as_deref()!=Ok("0")}
pub fn suffix(text:&str,kind:&str)->String{
 let phrase=match kind{"point"=>"в эту точку","rect"=>"в этой области",_=>return text.trim().into()};
 let lower=text.to_lowercase();
 if (kind=="point"&&["в эту точку","в этой точке"].iter().any(|s|lower.contains(s)))||(kind=="rect"&&lower.contains("в этой области")){return text.trim().into();}
 let text=text.trim();let end=text.trim_end_matches(|c:char|['.','!','?','…',',',';',':'].contains(&c));
 format!("{} {}{}",end.trim_end(),phrase,&text[end.len()..])
}
pub fn descriptor(d:&Draft,route:&Value,asset:&Value,effective_masks:&[Value],flag:bool)->Result<Value,String>{
 if !flag||!instructions::local(&d.operation)||!effective_masks.is_empty()||route["maskEncoding"]!="transparent-edits"||!["openai/gpt-image-2.5/flare/edit","openai/gpt-image-2.5/sunburst/edit"].contains(&route["id"].as_str().unwrap_or("")){return Ok(Value::Null);}
 let(_,tasks)=instructions::tasks(d)?;if tasks.len()!=1{return Ok(Value::Null);}
 let Some(a)=d.annotations.iter().find(|a|a["id"].as_str()==Some(tasks[0].0.as_str())&&a["kind"]=="rect"&&a["included"]==true)else{return Ok(Value::Null)};
 let(x,y)=geometry::point(&a["start"])?;let(u,v)=geometry::point(&a["end"])?;
 let w=asset["width"].as_u64().filter(|v|*v>0).ok_or("Нет ширины кадра")?;let h=asset["height"].as_u64().filter(|v|*v>0).ok_or("Нет высоты кадра")?;
 let bounds=[(x.min(u)*w as f64).floor() as u64,(y.min(v)*h as f64).floor() as u64,(x.max(u)*w as f64).ceil() as u64,(y.max(v)*h as f64).ceil() as u64];
 if bounds[0]>=bounds[2]||bounds[1]>=bounds[3]{return Err("Область не может быть пустой.".into());}
 Ok(json!({"version":VERSION,"policy":"single-local-rect","sourceAssetId":d.asset_id,"sourceSha256":asset["sha256"],"sourceSize":{"width":w,"height":h},"orientation":{"rotation":d.working_turn,"mirrored":d.mirrored},"annotationId":a["id"],"rect":{"left":x.min(u),"top":y.min(v),"right":x.max(u),"bottom":y.max(v)},"pixelBounds":bounds}))
}
pub fn raster(desc:&Value,asset:&Value,d:&Draft)->Result<image::GrayImage,String>{
 if desc["version"]!=VERSION||desc["sourceAssetId"]!=d.asset_id||desc["sourceSha256"]!=asset["sha256"]||desc["sourceSize"]["width"]!=asset["width"]||desc["sourceSize"]["height"]!=asset["height"]||desc["orientation"]!=json!({"rotation":d.working_turn,"mirrored":d.mirrored}){return Err("Сохранённая область не соответствует исходному кадру.".into());}
 let w=asset["width"].as_u64().ok_or("Нет ширины кадра")?;let h=asset["height"].as_u64().ok_or("Нет высоты кадра")?;
 if w==0||h==0||w.saturating_mul(h)>32_000_000{return Err("Маска превышает предел памяти.".into());}
 let b=desc["pixelBounds"].as_array().filter(|v|v.len()==4).ok_or("Неверные границы области")?;
 let b=b.iter().map(|v|v.as_u64().ok_or("Неверные границы области")).collect::<Result<Vec<_>,_>>()?;
 if b[0]>=b[2]||b[1]>=b[3]||b[2]>w||b[3]>h{return Err("Неверные границы области".into());}
 Ok(image::GrayImage::from_fn(w as u32,h as u32,|x,y|image::Luma([if (b[0]..b[2]).contains(&u64::from(x))&&(b[1]..b[3]).contains(&u64::from(y)){255}else{0}])))
}
pub fn plan_hash(plan:&Value)->String{
 use sha2::{Digest,Sha256};let mut identity=plan.clone();if let Some(o)=identity.as_object_mut(){o.remove("planHash");o.remove("warning");}
 format!("{:x}",Sha256::digest(identity.to_string().as_bytes()))
}

#[cfg(test)]mod tests{
 use super::*;
 fn fixture()->(Draft,Value,Value){let mut d=Draft::initial("a");d.operation="replace-object".into();d.annotations=vec![json!({"id":"r","assetId":"a","kind":"rect","instruction":"Добавь котёнка.","included":true,"visible":false,"start":{"x":0.4,"y":0.6},"end":{"x":0.1,"y":0.2}})];(d,json!({"id":"openai/gpt-image-2.5/flare/edit","maskEncoding":"transparent-edits"}),json!({"id":"a","sha256":"fixture-sha","width":17,"height":11}))}
 #[test]fn suffix_preserves_raw_and_punctuation_without_duplicate_verb(){for(raw,kind,want)in [("  Добавь мяч.  ","point","Добавь мяч в эту точку."),("Котёнок спит?!","rect","Котёнок спит в этой области?!"),("Добавь в этой точке мяч","point","Добавь в этой точке мяч"),("Добавь в этой области мяч.","rect","Добавь в этой области мяч.")]{assert_eq!(suffix(raw,kind),want);}}
 #[test]fn single_rect_only_explicit_masks_win_and_flag_is_read_only(){let(mut d,r,a)=fixture();let original=serde_json::to_value(&d).unwrap();assert!(descriptor(&d,&r,&a,&[],true).unwrap().is_object());assert!(descriptor(&d,&r,&a,&[],false).unwrap().is_null());assert!(descriptor(&d,&r,&a,&[json!({"maskAssetId":"brush"})],true).unwrap().is_null());assert_eq!(serde_json::to_value(&d).unwrap(),original);d.annotations.push(json!({"id":"p","kind":"point","included":true,"instruction":"мяч","point":{"x":0.8,"y":0.9}}));assert!(descriptor(&d,&r,&a,&[],true).unwrap().is_null());d.annotations[1]["included"]=json!(false);assert!(descriptor(&d,&r,&a,&[],true).unwrap().is_object());assert!(descriptor(&d,&json!({"id":"fal-ai/nano-banana-2/edit"}),&a,&[],true).unwrap().is_null());}
 #[test]fn reverse_rect_pixel_bounds_alpha_and_all_orientations(){let(mut d,r,a)=fixture();for turn in 0..4{for mirror in [false,true]{d.working_turn=turn;d.mirrored=mirror;let desc=descriptor(&d,&r,&a,&[],true).unwrap();assert_eq!(desc["pixelBounds"],json!([1,2,7,7]));let raster=raster(&desc,&a,&d).unwrap();assert_eq!(raster.pixels().filter(|p|p[0]>0).count(),30);let oriented=geometry::orient_image(image::DynamicImage::ImageLuma8(raster),turn,mirror).into_luma8();for x in 0..17{for y in 0..11{let(u,v)=geometry::orient_point((x as f64+0.5)/17.,(y as f64+0.5)/11.,turn,mirror);assert_eq!(oriented.get_pixel((u*oriented.width() as f64).floor() as u32,(v*oriented.height() as f64).floor() as u32)[0],if (1..7).contains(&x)&&(2..7).contains(&y){255}else{0});}}}}}
 #[test]fn plan_identity_covers_spatial_policy_orientation_source_and_price(){let(d,r,a)=fixture();let desc=descriptor(&d,&r,&a,&[],true).unwrap();let p=json!({"derivedMask":desc,"reservedMicros":20});for field in ["derivedMask","reservedMicros"]{let mut q=p.clone();q[field]=Value::Null;assert_ne!(plan_hash(&p),plan_hash(&q));}let mut corrupt=a.clone();corrupt["sha256"]=json!("changed");assert!(raster(&p["derivedMask"],&corrupt,&d).is_err());}
}
