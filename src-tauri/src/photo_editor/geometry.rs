//! Pixel and normalized geometry share the same canonical, EXIF-oriented basis.
use image::DynamicImage;
use serde_json::{json,Value};
pub fn turn_point(x:f64,y:f64,turn:u8)->(f64,f64){match turn%4{1=>(1.-y,x),2=>(1.-x,1.-y),3=>(y,1.-x),_=>(x,y)}}
pub fn turn_image(image:DynamicImage,turn:u8)->DynamicImage{match turn%4{1=>image.rotate90(),2=>image.rotate180(),3=>image.rotate270(),_=>image}}
pub fn orient_point(x:f64,y:f64,turn:u8,mirrored:bool)->(f64,f64){turn_point(if mirrored{1.-x}else{x},y,turn)}
pub fn orient_image(image:DynamicImage,turn:u8,mirrored:bool)->DynamicImage{turn_image(if mirrored{image.fliph()}else{image},turn)}
pub fn point(value:&Value)->Result<(f64,f64),String>{
    let x=value["x"].as_f64().ok_or("Неверная координата X")?;let y=value["y"].as_f64().ok_or("Неверная координата Y")?;
    if !x.is_finite()||!y.is_finite()||!(0.0..=1.0).contains(&x)||!(0.0..=1.0).contains(&y){return Err("Координаты должны находиться внутри исходного кадра.".into());}Ok((x,y))
}
pub fn validate_annotations(d:&super::contracts::Draft)->Result<(),String>{
    let mut ids=std::collections::HashSet::new();
    for a in &d.annotations{
        let object=a.as_object().ok_or("Неверный объект разметки")?;
        let kind=a["kind"].as_str().ok_or("Не указан тип разметки")?;
        let fields: &[&str]=match kind{"point"=>&["point"],"rect"=>&["start","end"],"mask"=>&["maskAssetId","version"],_=>return Err("Неизвестный тип разметки.".into())};
        if object.keys().any(|key|!["id","assetId","instruction","visible","included","kind"].contains(&key.as_str())&&!fields.contains(&key.as_str())){return Err("Неизвестное поле разметки.".into());}
        let id=a["id"].as_str().filter(|id|!id.is_empty()&&id.len()<=160).ok_or("Неверный ID разметки")?;
        if !ids.insert(id)||a["assetId"].as_str()!=Some(d.asset_id.as_str())||a["instruction"].as_str().is_none_or(|s|s.len()>16384)||!a["visible"].is_boolean()||!a["included"].is_boolean(){return Err("Неверная база или поля разметки.".into());}
        if kind=="point"{point(&a["point"])?;}
        if kind=="rect"{let p=point(&a["start"])?;let q=point(&a["end"])?;if p.0==q.0||p.1==q.1{return Err("Область не может быть пустой.".into());}}
        if kind=="mask"&&(a["maskAssetId"].as_str().is_none_or(|s|s.is_empty()||s.len()>160)||a["version"].as_u64().is_none_or(|n|n==0)){return Err("Неверная версия маски.".into());}
    }
    if d.selected_annotation_id.as_ref().is_some_and(|id|!ids.contains(id.as_str())){return Err("Выбранная разметка отсутствует.".into());}
    for id in &d.selected_mask_ids{if !d.annotations.iter().any(|a|a["id"].as_str()==Some(id)&&a["kind"]=="mask"){return Err("Выбранная маска отсутствует.".into());}}
    Ok(())
}
/// Rectangle is transformed through all corners; cropping clips it and drops empty regions.
pub fn transform_annotations(values:&[Value],asset:&str,mut f:impl FnMut(f64,f64)->(f64,f64))->Vec<Value>{
    let mut output=Vec::new();for original in values{let mut a=original.clone();a["assetId"]=json!(asset);
        match a["kind"].as_str(){Some("point")=>{if let Ok((x,y))=point(&a["point"]){let (x,y)=f(x,y);if !(0.0..=1.0).contains(&x)||!(0.0..=1.0).contains(&y){continue;}a["point"]=json!({"x":x,"y":y});}},Some("rect")=>{if let (Ok((x,y)),Ok((u,v)))=(point(&a["start"]),point(&a["end"])){let corners=[f(x,y),f(x,v),f(u,y),f(u,v)];let left=corners.iter().map(|p|p.0).fold(f64::INFINITY,f64::min).max(0.);let right=corners.iter().map(|p|p.0).fold(f64::NEG_INFINITY,f64::max).min(1.);let top=corners.iter().map(|p|p.1).fold(f64::INFINITY,f64::min).max(0.);let bottom=corners.iter().map(|p|p.1).fold(f64::NEG_INFINITY,f64::max).min(1.);if left>=right||top>=bottom{continue;}a["start"]=json!({"x":left,"y":top});a["end"]=json!({"x":right,"y":bottom});}},_=>{}}
        output.push(a);
    }output
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn eight_orientations_match_pixel_centres_and_alpha(){
        let source=image::RgbaImage::from_fn(7,3,|x,y|image::Rgba([x as u8,y as u8,(x*7+y) as u8,if x==1{0}else{255}]));
        for mirrored in [false,true]{for rotation in 0..4{let image=orient_image(DynamicImage::ImageRgba8(source.clone()),rotation,mirrored).to_rgba8();
            for(x,y,p)in source.enumerate_pixels(){let(u,v)=orient_point((x as f64+0.5)/7.,(y as f64+0.5)/3.,rotation,mirrored);assert_eq!(image.get_pixel((u*image.width() as f64).floor() as u32,(v*image.height() as f64).floor() as u32),p);let inverse=if mirrored{rotation}else{(4-rotation)%4};let(a,b)=orient_point(u,v,inverse,mirrored);assert!((a-(x as f64+0.5)/7.).abs()<1e-9&&(b-(y as f64+0.5)/3.).abs()<1e-9);}
        }}
    }
    #[test]fn rotations_match_pixels_and_inverse(){let image=image::RgbaImage::from_fn(3,2,|x,y|image::Rgba([x as u8,y as u8,0,if x==0{0}else{255}]));let turned=turn_image(DynamicImage::ImageRgba8(image.clone()),1).to_rgba8();assert_eq!(turned.dimensions(),(2,3));for(x,y,p)in image.enumerate_pixels(){assert_eq!(turned.get_pixel(1-y,x),p);}for t in 0..4{let p=turn_point(0.2,0.7,t);let q=turn_point(p.0,p.1,(4-t)%4);assert!((q.0-0.2).abs()<1e-9&&(q.1-0.7).abs()<1e-9);}}
    #[test]fn all_corners_and_clipping(){let d=super::super::contracts::Draft::initial("a");let rect=json!({"id":"r","kind":"rect","assetId":"a","instruction":"","visible":true,"included":true,"start":{"x":0.1,"y":0.2},"end":{"x":0.3,"y":0.6}});let result=transform_annotations(&[rect],"b",|x,y|turn_point(x,y,1));assert!((result[0]["start"]["x"].as_f64().unwrap()-0.4).abs()<1e-9);assert_eq!(result[0]["assetId"],"b");d.validate().unwrap();}
}
