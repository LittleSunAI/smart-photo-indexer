//! Optional pixel corrections never discard a usable provider image.
use image::{DynamicImage, GenericImageView};
use serde_json::{json,Value};

pub struct FinishedOutput {
    pub image: DynamicImage,
    pub warnings: Vec<String>,
    pub post_process: &'static str,
    pub correction: Option<Value>,
}
pub fn preserves_gray(source: &DynamicImage) -> bool {
    source.pixels().all(|(_, _, p)| p[0].abs_diff(p[1]) <= 2 && p[1].abs_diff(p[2]) <= 2)
}
pub fn output(image: DynamicImage, source: Option<DynamicImage>, operation: &str, expansion: &Value, preserve_color: bool) -> FinishedOutput {
    let mut result = FinishedOutput { image, warnings: Vec::new(), post_process: "none", correction: None };
    if operation == "outpaint" {
        let requested = source.as_ref().and_then(|s| {
            let side = |name: &str| expansion[name].as_u64().and_then(|v| u32::try_from(v).ok());
            let (l, r, t, b) = (side("left")?, side("right")?, side("top")?, side("bottom")?);
            Some((s.width().checked_add(l)?.checked_add(r)?, s.height().checked_add(t)?.checked_add(b)?, l, t))
        });
        if let Some((w, h, left, top)) = requested {
            if result.image.dimensions() == (w, h) {
                let mut canvas = result.image.to_rgba8();
                if image::GenericImage::copy_from(&mut canvas, &source.unwrap().to_rgba8(), left, top).is_ok() {
                    result.image = DynamicImage::ImageRgba8(canvas);
                    result.post_process = "preserve-source-pixels-v1";
                }
            } else {
                result.warnings.push(format!("Запрошено {w} × {h}, получено {} × {}. Показан ответ модели без растяжения; исходная область могла измениться.", result.image.width(), result.image.height()));
            }
        } else {
            result.warnings.push("Показан ответ модели. Сохранение исходных пикселей не подтверждено.".into());
        }
    } else if operation == "restore" && preserve_color {
        match source {
            Some(source) if preserves_gray(&source) => {
                result.image = result.image.grayscale();
                result.post_process = "preserve-color-choice-v1";
            }
            None => result.warnings.push("Показан ответ модели; цветность исходника не удалось проверить.".into()),
            _ => {},
        }
    }
    if operation == "remove-background" && !result.image.pixels().any(|(_, _, p)| p[3] < 255) {
        result.warnings.push("Модель вернула непрозрачное изображение. Ответ сохранён, но прозрачный фон не получен.".into());
    }
    result
}
/// Frozen local output target. A mismatched crop must not enter the exact-source overlay branch.
pub fn output_with_target(image:DynamicImage,source:Option<DynamicImage>,operation:&str,expansion:&Value,preserve_color:bool,target:&Value)->FinishedOutput{
    let requested=target["width"].as_u64().zip(target["height"].as_u64()).filter(|(w,h)|*w>0&&*h>0&&*w<=2048&&*h<=2048);
    if operation!="outpaint"||target["autoCrop"]!=true||requested.is_none(){return output(image,source,operation,expansion,preserve_color);}
    let(w,h)=requested.unwrap();let(w,h)=(w as u32,h as u32);let received=image.dimensions();
    let cw=received.0.min(w);let ch=received.1.min(h);let x=(received.0-cw)/2;let y=(received.1-ch)/2;
    let applied=(cw,ch)!=received;
    let correction=json!({"version":1,"processingVersion":"auto-crop-center-v1","applied":applied,"requested":{"width":w,"height":h},"received":{"width":received.0,"height":received.1},"actual":{"width":cw,"height":ch},"cropRect":if applied{json!({"x":x,"y":y,"width":cw,"height":ch})}else{Value::Null}});
    if !applied{let mut result=output(image,source,operation,expansion,preserve_color);result.correction=Some(correction);return result;}
    let mut warnings=vec![format!("Автообрезка по центру: получено {} × {}, сохранено {cw} × {ch}. Полный ответ модели сохранён в проекте; исходные пиксели не подтверждены.",received.0,received.1)];
    if(cw,ch)!=(w,h){warnings.push(format!("Запрошено {w} × {h}; фактический размер {cw} × {ch}. Меньшая сторона оставлена без увеличения."));}
    FinishedOutput{image:image.crop_imm(x,y,cw,ch),warnings,post_process:"auto-crop-center-v1",correction:Some(correction)}
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn exact_outpaint_preserves_source_pixels_and_alpha() {
        let source = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(512, 512, |x,y| image::Rgba([x as u8,y as u8,90,(x+y) as u8])));
        let result = output(DynamicImage::new_rgba8(608,576), Some(source.clone()), "outpaint", &json!({"left":32,"right":64,"top":16,"bottom":48}), true);
        assert!(result.warnings.is_empty());
        assert_eq!(result.post_process,"preserve-source-pixels-v1");
        for (x,y,p) in source.pixels() { assert_eq!(result.image.get_pixel(x+32,y+16),p); }
    }
    #[test]
    fn size_and_aspect_mismatch_keep_every_provider_pixel() {
        for (w,h) in [(1776,704),(1648,704),(1216,1152),(1024,512)] {
            let image=DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(w,h,image::Rgba([5,7,9,117])));
            let result=output(image.clone(),Some(DynamicImage::new_rgb8(1088,704)),"outpaint",&json!({"left":700,"right":0,"top":0,"bottom":0}),true);
            assert_eq!(result.image.as_bytes(),image.as_bytes());
            assert_eq!(result.image.dimensions(),(w,h));
            assert_eq!(result.post_process,"none");
            assert_eq!(result.warnings.len(),1);
        }
    }
    #[test]
    fn opaque_background_and_unavailable_source_are_warnings() {
        let image=DynamicImage::new_rgb8(64,64);
        assert_eq!(output(image.clone(),None,"remove-background",&Value::Null,true).warnings.len(),1);
        let result=output(image.clone(),None,"outpaint",&Value::Null,true);
        assert_eq!(result.image.as_bytes(),image.as_bytes());
        assert_eq!(result.post_process,"none");
        assert!(!result.warnings.is_empty());
    }
    #[test]
    fn restoration_respects_color_choice_and_alpha() {
        let source=DynamicImage::ImageRgb8(image::RgbImage::from_pixel(32,32,image::Rgb([90,90,90])));
        let image=DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(32,32,image::Rgba([200,80,15,117])));
        let result=output(image.clone(),Some(source.clone()),"restore",&Value::Null,true);
        let p=result.image.get_pixel(0,0);assert_eq!(p[0],p[1]);assert_eq!(p[1],p[2]);assert_eq!(p[3],117);
        assert_eq!(output(image.clone(),Some(source),"restore",&Value::Null,false).image.as_bytes(),image.as_bytes());
    }
    #[test]
    fn auto_crop_each_axis_and_odd_center_preserves_provider_pixels(){
      for(w,h,cw,ch,x,y)in[(2200,2100,2000,2000,100,50),(2200,1800,2000,1800,100,0),(1800,2200,1800,2000,0,100),(2000,2100,2000,2000,0,50),(1800,1800,1800,1800,0,0),(2000,2000,2000,2000,0,0),(2201,2101,2000,2000,100,50)]{
        let image=DynamicImage::ImageRgba8(image::RgbaImage::from_fn(w,h,|x,y|image::Rgba([x as u8,y as u8,31,117])));
        let result=output_with_target(image.clone(),None,"outpaint",&Value::Null,true,&json!({"width":2000,"height":2000,"autoCrop":true}));assert_eq!(result.image.dimensions(),(cw,ch));
        for(px,py)in[(0,0),(cw-1,ch-1),(cw/2,ch/2)]{assert_eq!(result.image.get_pixel(px,py),image.get_pixel(px+x,py+y));}
        assert_eq!(result.correction.as_ref().unwrap()["applied"],json!((cw,ch)!=(w,h)));
        if(cw,ch)!=(w,h){assert_eq!(result.post_process,"auto-crop-center-v1");assert_eq!(result.correction.unwrap()["cropRect"]["x"],x);}
      }
    }
    #[test]
    fn crop_never_uses_exact_size_overlay_and_legacy_remains_off(){
      let image=DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(800,650,image::Rgba([19,29,39,117])));let source=DynamicImage::new_rgba8(512,512);let expansion=json!({"left":44,"right":44,"top":44,"bottom":44});
      let result=output_with_target(image.clone(),Some(source.clone()),"outpaint",&expansion,true,&json!({"width":600,"height":600,"autoCrop":true}));assert_eq!(result.image.get_pixel(50,50),image::Rgba([19,29,39,117]));assert_eq!(result.post_process,"auto-crop-center-v1");
      for target in [Value::Null,json!({"width":600,"height":600}),json!({"width":600,"height":600,"autoCrop":false}),json!({"width":0,"height":600,"autoCrop":true})]{let result=output_with_target(image.clone(),Some(source.clone()),"outpaint",&expansion,true,&target);assert_eq!(result.image.as_bytes(),image.as_bytes());assert!(result.correction.is_none());}
    }
}
