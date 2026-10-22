use super::{contracts::Draft,local};
use serde_json::{json,Value};
pub fn bundled()->&'static Value{static C:std::sync::OnceLock<Value>=std::sync::OnceLock::new();C.get_or_init(||serde_json::from_str(include_str!("../../../src/modules/photoEditor/catalog.json")).expect("Pinned editor catalog"))}
pub fn route(id:&str)->Result<Value,String>{bundled()["routes"].as_array().unwrap().iter().find(|r|r["id"]==id).cloned().ok_or("Модель не включена в проверенный каталог редактора.".into())}
pub fn parameter_fields(properties:&serde_json::Map<String,Value>,definitions:&Value,preset:Option<&serde_json::Map<String,Value>>,values:&serde_json::Map<String,Value>)->Result<serde_json::Map<String,Value>,String>{
 let mut fields=preset.cloned().unwrap_or_default();
 let modes=values.get("__parameterModes").and_then(Value::as_object);
 if let Some(modes)=modes{for(key,mode)in modes{if !properties.contains_key(key)||!["preset","explicit","provider-default"].contains(&mode.as_str().unwrap_or("")){return Err("Неверный режим параметра модели".into());}if mode=="provider-default"{fields.remove(key);}}}
 for(k,v)in values{if k.starts_with("__")||modes.and_then(|m|m.get(k)).is_some_and(|v|v=="provider-default"){continue;}if ["image_url","image_urls","mask_url","mask_image_url","canvas_size","original_image_size","original_image_location","prompt","video_url","pdf_url","audio_url","sync_mode","enable_safety_checker","expand_left","expand_right","expand_top","expand_bottom","auto_crop"].contains(&k.as_str())||!properties.contains_key(k){return Err(format!("Поле «{k}» не разрешено в этой форме."));}validate_value(&properties[k],v,definitions,0).map_err(|e|format!("{k}: {e}"))?;fields.insert(k.clone(),v.clone());}
 for(key,mode)in modes.into_iter().flat_map(|m|m.iter()){if mode=="provider-default"{if let Some(default)=properties[key].get("default"){validate_value(&properties[key],default,definitions,0)?;fields.insert(key.clone(),default.clone());}}}
 Ok(fields)
}
pub fn validate_value(schema:&Value,value:&Value,definitions:&Value,depth:u8)->Result<(),String>{
 if depth>12{return Err("Слишком глубокая структура параметров.".into());}
 if let Some(reference)=schema["$ref"].as_str(){let name=reference.strip_prefix("#/components/schemas/").ok_or("Неизвестная schema reference")?;return validate_value(&definitions[name],value,definitions,depth+1);}
 if let Some(variants)=schema["anyOf"].as_array(){if !variants.iter().any(|s|validate_value(s,value,definitions,depth+1).is_ok()){return Err("Значение не соответствует схеме поля.".into());}}
 if let Some(choices)=schema["enum"].as_array(){if !choices.contains(value){return Err("Значение отсутствует в разрешённом списке.".into());}}
 match schema["type"].as_str(){
  Some("null") if !value.is_null()=>return Err("Ожидалось пустое значение.".into()),
  Some("boolean") if !value.is_boolean()=>return Err("Ожидался переключатель.".into()),
  Some("string")=>{let s=value.as_str().ok_or("Ожидался текст")?;let length=s.chars().count() as u64;if schema["minLength"].as_u64().is_some_and(|n|length<n)||schema["maxLength"].as_u64().is_some_and(|n|length>n){return Err("Длина текста выходит за границы поля модели.".into());}if s.len()>32768&&!s.starts_with("data:image/"){return Err("Слишком длинный текст поля.".into());}},
  Some("number")|Some("integer")=>{let n=value.as_f64().ok_or("Ожидалось число")?;if !n.is_finite()||schema["type"]=="integer"&&n.fract()!=0.||schema["minimum"].as_f64().is_some_and(|m|n<m)||schema["maximum"].as_f64().is_some_and(|m|n>m){return Err("Число выходит за пределы поля.".into());}},
  Some("array")=>{let values=value.as_array().ok_or("Ожидался список")?;if values.len()>32||schema["maxItems"].as_u64().is_some_and(|n|values.len()>n as usize)||schema["minItems"].as_u64().is_some_and(|n|values.len()<n as usize){return Err("Неверная длина списка.".into());}if schema.get("items").is_some(){for v in values{validate_value(&schema["items"],v,definitions,depth+1)?;}}},
  Some("object")=>{let values=value.as_object().ok_or("Ожидался объект")?;if let Some(properties)=schema["properties"].as_object(){if values.keys().any(|k|!properties.contains_key(k)){return Err("Объект содержит неизвестные поля.".into());}for(k,v)in values{validate_value(&properties[k],v,definitions,depth+1)?;}if let Some(required)=schema["required"].as_array(){for field in required{if !values.contains_key(field.as_str().ok_or("Invalid required field")?){return Err("Не заполнено обязательное поле.".into());}}}}},
  _=>{}
 }Ok(())
}
pub fn compile_prompt(draft:&Draft)->String{
 let action=match draft.operation.as_str(){"replace-object"=>"Добавьте или замените указанный объект.","replace-background"=>"Замените фон, сохранив главный объект и его форму.","outpaint"=>"Создайте одну фотографию: заполните все прозрачные поля непрерывным естественным продолжением исходной сцены до каждого края полотна. Сохраните исходную область. Не оставляйте пустые полосы, рамки или сетку и не дублируйте фотографию.","relight"=>"Измените освещение, сохранив объекты и композицию.","recolor"=>"Измените указанные цвета, сохранив текстуру и форму.","compose"=>"Составьте композицию из основного и всех дополнительных изображений в указанном порядке. Используйте каждый вход; не добавляйте новых людей и не меняйте исходные лица. При запросе коллажа оставляйте сцены раздельными и не дублируйте входы.","restore"=>"Восстановите повреждённую фотографию: уберите дефекты, сохранив лица, предметы и характер снимка. Сохраните исходные цвета; чёрно-белые снимки оставьте чёрно-белыми, если пользователь явно не попросил раскрасить.",_=>"Измените изображение по инструкции, сохранив остальные важные детали."};
 let (global,tasks)=super::instructions::tasks(draft).unwrap_or_default();
 let mut result=format!("{action}\n{}",global.trim());
 for (i,(target,text)) in tasks.iter().enumerate(){result.push_str(&format!("\n{}. Метка [{}], {}: {}",i+1,target,super::instructions::region(draft,target),super::spatial::suffix(text,draft.annotations.iter().find(|a|a["id"].as_str()==Some(target)).and_then(|a|a["kind"].as_str()).unwrap_or(""))));}
 if draft.model_id.as_deref().and_then(|id|route(id).ok()).is_some_and(|r|r["maskEncoding"]=="transparent-edits")&&draft.annotations.iter().any(|a|a["kind"]=="mask"&&a["included"]==true&&draft.selected_mask_ids.iter().any(|id|a["id"].as_str()==Some(id))){
  result.push_str("\nThe supplied raster mask defines the editable region. Apply the requested change only inside that region; preserve the composition and all content outside the mask. Do not relocate the edit to another part of the image.");
 }
  result
}
pub fn reference_prompt(draft:&Draft,route:&str,prompt:&str,inputs:&[Value])->String{
 let label=if route.contains("seedream/"){"Figure"}else{"Image"};
 let mut text=prompt.to_string();
 if inputs.iter().any(|i|i["bindings"].as_array().is_some_and(|b|!b.is_empty()))||inputs.len()>1{
  text.push_str(&format!("\n{label} 1 — редактируемая сцена. Остальные входы — образцы предметов/деталей, не дополнительные результаты. Все метки относятся к {label} 1; координаты в процентах от левого верхнего угла. Выполните каждое включённое задание в его месте; сохраните остальную сцену, пропорции и фон. Не копируйте фон образцов и не создавайте коллаж без явной просьбы."));
  for(i,input)in inputs.iter().enumerate(){
   if i>0&&input["bindings"].as_array().is_none_or(|v|v.is_empty()){text.push_str(&format!("\n{label} {}: {}.",i+1,input["role"].as_str().unwrap_or("образец")));}
   for binding in input["bindings"].as_array().into_iter().flatten(){
    let id=binding["annotationId"].as_str().unwrap_or("");
    let name=serde_json::to_string(binding["displayName"].as_str().unwrap_or("")).unwrap_or_default();
    text.push_str(&format!("\n{label} {} ({name}) — образец для метки [{}], {} на {label} 1. Роль: {}. Задание этой метки: {}",i+1,id,super::instructions::region(draft,id),binding["roleDescription"].as_str().filter(|s|!s.is_empty()).unwrap_or("внешний вид/предметы образца"),binding["instruction"].as_str().unwrap_or("")));
   }
  }
 }
 text
}
pub fn plan(app:&tauri::AppHandle,project:&str,draft:&Draft)->Result<Value,String>{
 plan_inner(app,project,draft,false)
}
/// Read-only model filtering may estimate before the user writes an instruction
/// or paints a mask. Submission always uses the strict plan above.
pub fn model_probe(app:&tauri::AppHandle,project:&str,draft:&Draft)->Result<Value,String>{
 plan_inner(app,project,draft,true)
}
fn plan_inner(app:&tauri::AppHandle,project:&str,draft:&Draft,probe:bool)->Result<Value,String>{
 draft.validate()?;if draft.form_values.get("__rawJSONError")==Some(&json!(true))||draft.form_values.get("__invalidFields").and_then(Value::as_object).is_some_and(|m|!m.is_empty()){return Err("Исправьте неверный JSON или числовое поле; предыдущие параметры не будут отправлены скрытно.".into());}let selected=draft.model_id.as_deref().ok_or("Выберите модель редактирования.")?;let current=super::review::active(app)?;let mut r=current["routes"].as_array().ok_or("Каталог повреждён")?.iter().find(|r|r["id"]==selected).cloned().ok_or("Модель отсутствует в активном каталоге")?;
 if let Some(reference)=draft.form_values.get("__pricingCredential").and_then(Value::as_str){if let Some(rate)=super::pricing::cached(app,reference,selected)?{if rate["currency"]=="USD"&&rate["unit"]==r["priceUnit"]{r["priceMicros"]=json!((rate["unitPrice"].as_f64().ok_or("Тариф повреждён")?*1e6).ceil() as u64);r["pricingCheckedAtUnix"]=rate["checkedAtUnix"].clone();r["pricingAuthenticated"]=json!(true);}}}
 if !r["operations"].as_array().unwrap().iter().any(|o|o==&draft.operation){return Err("Модель несовместима с выбранной операцией.".into());}
 if !probe&&draft.form_values.get("__maskStrokes").and_then(Value::as_array).is_some_and(|v|!v.is_empty()){return Err("Сначала примените или отмените маску.".into());}
  let asset=local::project_asset(app,project,&draft.asset_id)?;if asset["kind"]!="image"{return Err("Главный источник должен быть изображением.".into());}
  for id in std::iter::once(draft.asset_id.as_str()){let a=local::project_asset(app,project,id)?;let pixels=a["width"].as_u64().ok_or("Нет ширины")?.saturating_mul(a["height"].as_u64().ok_or("Нет высоты")?);if pixels.saturating_mul(24).saturating_add(48*1024*1024)>384*1024*1024{return Err("Подготовка этого кадра превысит предел памяти. Сначала явно уменьшите рабочую копию инструментом «Изменить размер».".into());}}
 let properties=r["inputSchema"]["properties"].as_object().ok_or("Неверная схема модели")?;
 let chosen=current["choices"].as_array().and_then(|rows|rows.iter().find(|c|draft.form_values.get("__choiceId")==Some(&c["id"])&&c["route"]==selected&&c["tier"]==draft.profile)).or_else(||current["choices"].as_array().and_then(|rows|rows.iter().find(|c|c["route"]==selected&&c["tier"]==draft.profile)));
 let mut fields=parameter_fields(properties,&r["definitions"],chosen.and_then(|c|c["preset"].as_object()),&draft.form_values)?;
 if properties.contains_key("prompt")&&!(selected=="topaz/upscale/image/generative"&&!fields.get("model").and_then(Value::as_str).unwrap_or("").starts_with("Redefine")){
 let text=if probe{super::instructions::plain(draft).unwrap_or_default()}else{super::instructions::plain(draft)?};
 if !probe&&["universal-edit","replace-object","replace-background","relight","recolor","compose"].contains(&draft.operation.as_str())&&text.trim().is_empty(){return Err("Опишите нужное изменение перед запуском.".into());}
 if selected=="fal-ai/bria/background/remove"{if !text.trim().is_empty(){fields.insert("prompt".into(),json!(text.trim()));}}
 else if selected=="fal-ai/bria/expand"{if !text.is_ascii(){return Err("BRIA Expand принимает инструкцию на английском. Выберите другой редактор для русской инструкции.".into());}if !text.trim().is_empty(){fields.insert("prompt".into(),json!(text.trim()));}}
 else if selected=="fal-ai/ideogram/v3/replace-background"{fields.insert("prompt".into(),json!(text.trim()));}
 else if selected=="fal-ai/flux-pro/v1/fill"{fields.insert("prompt".into(),json!(format!("Empty surface matching the surrounding background, texture, lighting and perspective. {}",text.trim())));}
 else{fields.insert("prompt".into(),json!(compile_prompt(draft)));}
 }
 for (key,schema) in properties{if fields.contains_key(key)||["image_url","image_urls","mask_url","mask_image_url","canvas_size","original_image_size","original_image_location","prompt","video_url","audio_url","pdf_url","sync_mode","enable_safety_checker","model","image_size","expand_left","expand_right","expand_top","expand_bottom","auto_crop"].contains(&key.as_str()){continue;}if let Some(value)=schema.get("default").filter(|v|!v.is_null()){fields.insert(key.clone(),value.clone());}}
 if selected=="fal-ai/flux-2-pro/outpaint"&&!draft.form_values.contains_key("mode"){fields.insert("mode".into(),json!(if draft.profile=="economy"{"fast"}else{"high"}));}
 if selected=="fal-ai/seedvr/upscale/image"{if fields.get("upscale_mode")==Some(&json!("target")){fields.remove("upscale_factor");}else{fields.remove("target_resolution");}}

 if properties.contains_key("sync_mode"){fields.insert("sync_mode".into(),json!(false));}if properties.contains_key("enable_safety_checker"){fields.insert("enable_safety_checker".into(),json!(true));}
 if selected=="pixelcut/background-removal"{if fields.get("output_format").is_some_and(|v|v!=&json!("rgba")){return Err("Для кадра с прозрачностью выберите rgba; ZIP и отдельная alpha-карта не являются готовой фотографией.".into());}fields.insert("output_format".into(),json!("rgba"));}
 if selected=="fal-ai/object-removal/mask"&&!fields.contains_key("model"){fields.insert("model".into(),json!(if draft.profile=="economy"{"low_quality"}else if draft.profile=="balanced"{"medium_quality"}else{"best_quality"}));}
 let n=fields.get("num_images").and_then(Value::as_u64).unwrap_or(1);if !(1..=4).contains(&n){return Err("Разрешено от 1 до 4 вариантов.".into());}
 if super::instructions::local(&draft.operation)||draft.operation=="compose"{let capacity=super::references::capacity(app,project,draft,None)?.len();if capacity>r["maxInputs"].as_u64().unwrap_or(1) as usize{return Err("Прикреплённые дополнительные картинки превышают лимит этой модели. Выберите совместимую модель или исключите лишние дополнительные картинки.".into());}}
 let mut input_draft=draft.clone();if r["maxInputs"]==1{input_draft.references.clear();input_draft.annotation_references.clear();}let inputs=super::references::inputs(app,project,&input_draft,None)?;for input in &inputs{let pixels=input["width"].as_u64().ok_or("Нет ширины входа")?.saturating_mul(input["height"].as_u64().ok_or("Нет высоты входа")?);if pixels.saturating_mul(24).saturating_add(48*1024*1024)>384*1024*1024{return Err("Дополнительная картинка превышает бюджет подготовки. Используйте уменьшенную копию; сохранённый проект не изменён.".into());}}
 if inputs.len()>r["maxInputs"].as_u64().unwrap_or(1) as usize{return Err("Превышено число изображений для выбранной модели; ничего не обрезано автоматически.".into());}
 if let Some(prompt)=fields.get("prompt").and_then(Value::as_str).map(String::from){fields.insert("prompt".into(),json!(reference_prompt(draft,selected,&prompt,&inputs)));}
 if !probe&&draft.operation=="compose"&&inputs.len()<2{return Err("Для композиции добавьте хотя бы один дополнительный источник.".into());}
 let masks:Vec<Value>=draft.annotations.iter().filter(|_|!r["maskField"].is_null()).filter(|a|a["kind"]=="mask"&&a["included"]==true&&draft.selected_mask_ids.iter().any(|id|a["id"].as_str()==Some(id))).cloned().collect();
 let derived_mask=super::spatial::descriptor(draft,&r,&asset,&masks,super::spatial::enabled())?;
 if !probe&&(r["maskPolicy"]=="required"||draft.operation=="remove-object")&&masks.is_empty(){return Err("Эта модель требует применённую и включённую маску.".into());}
 let(w,h)=if draft.working_turn%2==0{(asset["width"].as_u64().unwrap(),asset["height"].as_u64().unwrap())}else{(asset["height"].as_u64().unwrap(),asset["width"].as_u64().unwrap())};
 let mut size=fields.get("image_size").cloned().unwrap_or_else(||json!({"width":w,"height":h}));
 if draft.form_values.get("__outputSizeMode")==Some(&json!("source"))&&draft.operation!="outpaint"&&draft.operation!="upscale" {
  if !properties.contains_key("image_size"){return Err("Эта модель не принимает точный размер в пикселях. Выберите её разрешение явно или другую модель с собственным размером.".into());}
  size=json!({"width":w,"height":h});fields.insert("image_size".into(),size.clone());
 }
 if ["fal-ai/flux-2/flash/edit","fal-ai/flux-2/klein/4b/edit"].contains(&selected)&&!fields.contains_key("image_size"){size=super::adapter_parameters::aligned_edit_size(w,h)?;fields.insert("image_size".into(),size.clone());}
 if selected=="fal-ai/iclight-v2"&&!fields.contains_key("image_size"){fields.insert("image_size".into(),size.clone());}
  if selected=="xai/grok-imagine-image/edit"{fields.entry("aspect_ratio").or_insert(json!("auto"));}
 if inputs.len()>1&&["universal-edit","replace-object","replace-background","relight","recolor","restore"].contains(&draft.operation.as_str())&&!draft.form_values.contains_key("aspect_ratio"){if let Some(ratio)=base_aspect_ratio(properties.get("aspect_ratio").unwrap_or(&Value::Null),w,h){fields.insert("aspect_ratio".into(),json!(ratio));}}

 let expansion=if draft.operation=="outpaint"{let frame=super::outpaint::Rect::read(draft,(w as u32,h as u32))?;let[left,right,top,bottom]=frame.expansion((w as u32,h as u32))?;size=json!({"width":w+left+right,"height":h+top+bottom});if selected=="fal-ai/bria/expand"{fields.remove("aspect_ratio");fields.insert("canvas_size".into(),json!([w+left+right,h+top+bottom]));fields.insert("original_image_size".into(),json!([w,h]));fields.insert("original_image_location".into(),json!([left,top]));}else if properties.contains_key("image_size"){fields.insert("image_size".into(),size.clone());}else if properties.contains_key("aspect_ratio"){if fields.get("aspect_ratio").is_some_and(|v|v!=&json!("auto")){return Err("Для расширения выберите автоматическое соотношение сторон: оно берётся из расширенного полотна.".into());}fields.insert("aspect_ratio".into(),json!("auto"));}else if selected=="fal-ai/flux-2-pro/outpaint"{for(name,value)in [("expand_left",left),("expand_right",right),("expand_top",top),("expand_bottom",bottom)]{fields.insert(name.into(),json!(value));}fields.insert("auto_crop".into(),json!(false));fields.entry("output_format").or_insert(json!("png"));}else{return Err("Эта модель не поддерживает размер расширенного полотна.".into());}json!({"left":left,"right":right,"top":top,"bottom":bottom})}else{Value::Null};
 if selected=="fal-ai/flux-2/flash/edit"{validate_flash_size(&size)?;}
 super::adapter_parameters::validate(&r,&mut fields,w,h,&draft.operation,&size)?;
 for(key,value)in &fields{validate_value(properties.get(key).ok_or_else(||format!("Недопустимое поле адаптера: {key}"))?,value,&r["definitions"],0).map_err(|e|format!("{key}: {e}"))?;}
  let reserve=super::estimates::reserve(&r,&fields,&inputs,w,h,&size)?;
 let mut warnings=Vec::new();if selected=="fal-ai/seedvr/upscale/image"&&fields.get("upscale_mode")==Some(&json!("target")){warnings.push("Для целевого разрешения показан верхний резерв до 32 Мп, а не точное списание.");}if !draft.selected_mask_ids.is_empty()&&r["maskField"].is_null(){warnings.push("Эта модель не принимает растровую маску. Для локальной правки используйте точку или область с инструкцией; точная граница не гарантирована.");}if selected=="fal-ai/flux-2-pro/outpaint"{warnings.push("Модель автоматически продолжает сцену и не принимает текстовую инструкцию. Параметры задают размер полей и качество.");}if !expansion.is_null()&&!properties.contains_key("image_size")&&selected!="fal-ai/flux-2-pro/outpaint"{warnings.push("Модель выбирает разрешение результата сама; соотношение сторон берётся из расширенного полотна.");}
 let mut plan=json!({"version":1,"spatialCompilerVersion":super::spatial::VERSION,"derivedMask":derived_mask,"catalogId":current["id"],"choiceId":chosen.map(|c|c["id"].clone()),"estimateIsApproximate":true,"route":r,"operation":draft.operation,"fields":fields,"inputs":inputs,"masks":masks,"workingOrientation":{"rotation":draft.working_turn,"mirrored":draft.mirrored},"expansion":expansion,"outputTarget":if draft.operation=="outpaint"{json!({"width":size["width"],"height":size["height"],"autoCrop":draft.form_values.get("__outpaintAutoCrop")==Some(&json!(true))})}else{Value::Null},"reservedMicros":reserve,"outputCount":n,"warning":warnings.join(" ")});
 plan["planHash"]=json!(super::spatial::plan_hash(&plan));Ok(plan)
}
fn base_aspect_ratio(schema:&Value,w:u64,h:u64)->Option<String>{
 let choices=schema.get("enum").and_then(Value::as_array).or_else(||schema["anyOf"].as_array()?.iter().find_map(|s|s["enum"].as_array()))?;
 let source_ratio=w as f64/h as f64;choices.iter().filter_map(|v|{let label=v.as_str()?;let(a,b)=label.split_once(':')?;let ratio=a.parse::<f64>().ok()?/b.parse::<f64>().ok()?;Some((label,(ratio/source_ratio).ln().abs()))}).min_by(|a,b|a.1.total_cmp(&b.1)).map(|(label,_)|label.into())
}
fn validate_flash_size(size:&Value)->Result<(),String>{if size.is_object(){for key in ["width","height"]{let value=size[key].as_u64().ok_or("Размер результата должен быть целым числом.")?;if !(512..=2048).contains(&value){return Err(format!("FLUX Flash не может выдать {}×{}: каждая сторона должна быть от 512 до 2048 px. Выберите поддерживаемый размер явно или другую модель.",size["width"],size["height"]));}}}Ok(())}
#[cfg(test)]mod tests{use super::*;#[test]fn pinned_schemas_reject_unknown_enum_and_wrong_type(){let r=route("fal-ai/flux-2/flash/edit").unwrap();assert!(validate_value(&r["inputSchema"]["properties"]["num_images"],&json!(5),&r["definitions"],0).is_err());assert!(validate_value(&r["inputSchema"]["properties"]["output_format"],&json!("zip"),&r["definitions"],0).is_err());assert!(validate_value(&r["inputSchema"]["properties"]["enable_prompt_expansion"],&json!("true"),&r["definitions"],0).is_err());}}

#[cfg(test)]mod source_aspect_tests{use super::*;#[test]fn reference_frames_do_not_choose_reference_aspect_ratio(){for model in ["fal-ai/nano-banana-2/edit","fal-ai/nano-banana-pro/edit"]{let r=route(model).unwrap();let schema=&r["inputSchema"]["properties"]["aspect_ratio"];assert_eq!(base_aspect_ratio(schema,640,480),Some("4:3".into()));assert_eq!(base_aspect_ratio(schema,480,640),Some("3:4".into()));assert_eq!(base_aspect_ratio(schema,512,512),Some("1:1".into()));}}}

#[cfg(test)]mod parameter_mode_tests{
 use super::*;
 #[test]fn defaults_unset_and_nullable_values_survive_presets(){
  let props=json!({"style":{"type":"string","enum":["REALISTIC"]},"image_size":{"type":"string","default":"auto_2K","enum":["auto_1K","auto_2K"]},"nullable":{"anyOf":[{"type":"integer"},{"type":"null"}]}});
  let preset=json!({"style":"REALISTIC","image_size":"auto_1K","nullable":3});
  let values=json!({"__parameterModes":{"style":"provider-default","image_size":"provider-default","nullable":"explicit"},"nullable":null});
  let fields=parameter_fields(props.as_object().unwrap(),&Value::Null,preset.as_object(),values.as_object().unwrap()).unwrap();
  assert!(!fields.contains_key("style"));assert_eq!(fields["image_size"],"auto_2K");assert!(fields["nullable"].is_null());
 }
 #[test]fn all_catalog_choices_keep_valid_parameter_modes(){
  for choice in bundled()["choices"].as_array().unwrap(){
   let r=route(choice["route"].as_str().unwrap()).unwrap();let props=r["inputSchema"]["properties"].as_object().unwrap();
   let fields=parameter_fields(props,&r["definitions"],choice["preset"].as_object(),&serde_json::Map::new()).unwrap();
   for(k,v)in fields{validate_value(&props[&k],&v,&r["definitions"],0).unwrap();}
  }
 }
}

#[cfg(test)]mod reference_prompt_tests {
 use super::*;
 #[test]fn selected_mask_limits_instruction_without_mutating_user_text(){
  let mut d=Draft::initial("image");d.operation="replace-object".into();d.model_id=Some("openai/gpt-image-2.5/sunburst/edit".into());d.prompt="Add a beetle".into();d.annotations=vec![json!({"id":"m","kind":"mask","included":true,"visible":false})];d.selected_mask_ids=vec!["m".into()];
  let prompt=compile_prompt(&d);assert!(prompt.contains("only inside that region"));assert!(prompt.contains("Add a beetle"));assert_eq!(d.prompt,"Add a beetle");
  d.selected_mask_ids.clear();assert!(!compile_prompt(&d).contains("supplied raster mask"));
  d.selected_mask_ids=vec!["m".into()];d.model_id=Some("fal-ai/flux-2/flash/edit".into());assert!(!compile_prompt(&d).contains("supplied raster mask"));
 }
 #[test]fn every_reference_of_every_region_is_numbered_and_owned(){
  let mut d=Draft::initial("main");d.operation="replace-object".into();
  d.annotations=vec![json!({"id":"p","kind":"point","point":{"x":0.2,"y":0.4},"included":true,"instruction":"человек"}),json!({"id":"r1","kind":"rect","start":{"x":0.1,"y":0.1},"end":{"x":0.4,"y":0.3},"included":true,"instruction":"машина и снеговик"}),json!({"id":"r2","kind":"rect","start":{"x":0.5,"y":0.5},"end":{"x":0.9,"y":0.9},"included":true,"instruction":"лошадь"})];
  let inputs=json!([{"bindings":[]},{"bindings":[{"annotationId":"p","displayName":"человек.png","instruction":"человек"}]},{"bindings":[{"annotationId":"r1","displayName":"машина.png","instruction":"машина и снеговик"},{"annotationId":"r2","displayName":"общий.png","instruction":"лошадь"}]},{"bindings":[{"annotationId":"r1","displayName":"снеговик.png","instruction":"машина и снеговик"}]}]);
  let prompt=reference_prompt(&d,"bytedance/seedream/v5/pro/edit",&compile_prompt(&d),inputs.as_array().unwrap());
  for text in ["[p]","[r1]","[r2]","Figure 2","Figure 3","Figure 4","человек.png","машина.png","снеговик.png","лошадь"]{assert!(prompt.contains(text),"Missing {text}");}
  assert!(!prompt.contains("Figure 5"));
 }
}
