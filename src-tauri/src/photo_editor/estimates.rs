//! Route-specific estimates. Token/compute figures are explicitly not invoices or hard ceilings.
use serde_json::{Map,Value};
pub fn size(value:&Value,w:u64,h:u64)->(u64,u64){
 if value.is_object(){return(value["width"].as_u64().unwrap_or(w),value["height"].as_u64().unwrap_or(h));}
 let ratio=w as f64/h.max(1) as f64;
 match value.as_str().unwrap_or("auto"){
  "square"=>(512,512),"square_hd"=>(1024,1024),"portrait_4_3"=>(768,1024),"landscape_4_3"=>(1024,768),"portrait_16_9"=>(576,1024),"landscape_16_9"=>(1024,576),
  "auto_1K"|"auto_2K"|"auto_3K"|"auto_4K"=>{let pixels=match value.as_str(){Some("auto_2K")=>4_194_304.,Some("auto_3K")=>9_437_184.,Some("auto_4K")=>16_777_216.,_=>1_048_576.};((pixels*ratio).sqrt().round() as u64,(pixels/ratio).sqrt().round() as u64)},_=> (w,h)
 }
}
pub fn reserve(route:&Value,fields:&Map<String,Value>,inputs:&[Value],w:u64,h:u64,size_value:&Value)->Result<u64,String>{
 let n=fields.get("num_images").and_then(Value::as_u64).unwrap_or(1);
 let(wout,hout)=size(size_value,w,h);let factor=fields.get("upscale_factor").and_then(Value::as_f64).unwrap_or(2.);
 let enlarged=(w as f64*h as f64*factor*factor).ceil() as u64;
 let base=route["priceMicros"].as_u64().ok_or("Тариф неизвестен")?;
 let id=route["id"].as_str().unwrap_or("");let output=wout.saturating_mul(hout);
 let input_mp=inputs.iter().map(|i|(i["width"].as_u64().unwrap_or(w)*i["height"].as_u64().unwrap_or(h)).div_ceil(1_000_000)).sum::<u64>();
 let total=match route["pricePolicy"].as_str().unwrap_or("fixed"){
  "input-output-mp"=>{if id.contains("seedvr") {base*(input_mp+if fields.get("upscale_mode").and_then(Value::as_str)==Some("target"){32}else{enlarged.div_ceil(1_000_000)})}else{base*((if id.contains("/flash/"){inputs.len() as u64}else{input_mp})+output.div_ceil(1_000_000)*n)}},
  "output-image-plus-input-image"=>base*n+2000*inputs.len() as u64,
  "banana-resolution-and-options"=>{let resolution=fields.get("resolution").and_then(Value::as_str).unwrap_or("1K");let multiplier=match resolution{"0.5K"=>0.75,"2K" if id.contains("nano-banana-2/")=>1.5,"4K"=>2.,_=>1.};(base as f64*multiplier*n as f64).ceil() as u64+if fields.get("enable_web_search")==Some(&Value::Bool(true)){15000}else{0}+if fields.get("thinking_level").and_then(Value::as_str)==Some("high"){2000}else{0}},
  "area-tier-plus-additional-input"=>(if output<=1536*1536{base}else{base*2}+4500*(inputs.len() as u64).saturating_sub(1))*n,
  "token"=>{let quality=fields.get("quality").and_then(Value::as_str).unwrap_or("high");let unit=match quality{"low"=>14200,"medium"=>21500,"high"=>61000,"xhigh"=>125000,"max"=>250000,_=>61000};let area=output.max(1_048_576) as f64/1_048_576.;(unit as f64*area*n as f64*base as f64/1_000_000.).ceil() as u64+12288*(inputs.len() as u64).saturating_sub(1)+fields.get("prompt").and_then(Value::as_str).map(|s|s.chars().count() as u64*5).unwrap_or(0)},
  "outpaint-input-output-mp"=>{let fast=fields.get("mode").and_then(Value::as_str)==Some("fast");let first=if fast{10000}else{30000};first+15000*((w*h).div_ceil(1_000_000).saturating_sub(1)+output.div_ceil(1_000_000).saturating_sub(1))},
  "output-mp-ceil"=>base*output.div_ceil(1_000_000)*n,
  "rendering-speed-output-image"=>base*match fields.get("rendering_speed").and_then(Value::as_str).unwrap_or("TURBO"){"BALANCED"=>2,"QUALITY"=>3,_=>1}*n,
  "output-24mp-block"=>base*8*(if id.contains("upscale"){enlarged}else{w*h}).div_ceil(24_000_000),
  "output-8mp-block"=>base*8*enlarged.div_ceil(8_000_000),
  "compute-second"=>base*60,
  "mp-unresolved"=>base*if id.contains("clarity"){enlarged.div_ceil(1_000_000)}else if id=="fal-ai/iclight-v2"{output.max(w*h).div_ceil(1_000_000)}else{(w*h).div_ceil(1_000_000)},
  _=>if id=="fal-ai/bria/background/remove"&&fields.get("prompt").and_then(Value::as_str).is_some_and(|s|!s.trim().is_empty()){30000*n}else if id=="fal-ai/object-removal/mask"{match fields.get("model").and_then(Value::as_str).unwrap_or("low_quality"){"medium_quality"=>12000,"high_quality"=>18000,"best_quality"=>24000,_=>6000}}else{base*n}
 };
 if fields.contains_key("image_size")&&output>32_000_000||fields.contains_key("upscale_factor")&&enlarged>32_000_000{return Err("Выход превышает предел подготовки 32 Мп; уменьшите коэффициент или рабочий кадр.".into());}
 Ok(total)
}
#[cfg(test)]mod tests{
 use super::*;use serde_json::json;
 #[test]fn different_units_and_modes_are_not_one_fixed_image_price(){let input=vec![json!({"width":1024,"height":1024})];let route=json!({"id":"topaz/upscale/image/precision","priceMicros":10000,"pricePolicy":"output-24mp-block"});let fields=json!({"upscale_factor":2}).as_object().unwrap().clone();assert_eq!(reserve(&route,&fields,&input,1024,1024,&Value::Null).unwrap(),80000);let route=json!({"id":"fal-ai/ideogram/v3/replace-background","priceMicros":30000,"pricePolicy":"rendering-speed-output-image"});let fields=json!({"rendering_speed":"QUALITY"}).as_object().unwrap().clone();assert_eq!(reserve(&route,&fields,&input,1024,1024,&Value::Null).unwrap(),90000);}
}
