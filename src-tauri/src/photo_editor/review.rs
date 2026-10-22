//! Recommendations are reviewed versions; endpoint contracts are never edited by advice.
use crate::*;
use super::{catalog,store};
use serde_json::{json,Value};
/// JSON / WebView roundtrips encode integral floats as integers. That is not
/// a contract change; keys, nonnumeric values and unsafe large numbers stay strict.
fn equivalent(a:&Value,b:&Value)->bool{
 if a==b{return true;}
 match(a,b){
  (Value::Number(x),Value::Number(y)) if x.is_f64()!=y.is_f64()=>{match(x.as_f64(),y.as_f64()){(Some(x),Some(y))=>x.abs()<=9007199254740991.&&y.abs()<=9007199254740991.&&x==y,_=>false}},
  (Value::Array(x),Value::Array(y))=>x.len()==y.len()&&x.iter().zip(y).all(|(a,b)|equivalent(a,b)),
  (Value::Object(x),Value::Object(y))=>x.len()==y.len()&&x.iter().all(|(k,v)|y.get(k).is_some_and(|other|equivalent(v,other))),
  _=>false
 }
}
pub fn active(app:&tauri::AppHandle)->Result<Value,String>{
 let mut db=store::connection(app)?;
 let raw:Option<String>=db.query_row("SELECT catalog FROM editor_catalog_versions WHERE active=1 ORDER BY rowid DESC LIMIT 1",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
 let Some(raw)=raw else{return Ok(catalog::bundled().clone());};
 let old:Value=serde_json::from_str(&raw).map_err(|_|"Активный каталог повреждён")?;
 if validate(&old).is_ok(){return Ok(old);}
 let value=migrate_legacy(&old)?;
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 tx.execute("INSERT OR IGNORE INTO editor_catalog_versions(id,catalog,created_at_unix,active) VALUES(?1,?2,?3,0)",params![value["id"].as_str(),value.to_string(),unix_now()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_catalog_versions SET active=0",[]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE editor_catalog_versions SET active=1 WHERE id=?1",[value["id"].as_str().unwrap()]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(value)
}
fn migrate_legacy(old:&Value)->Result<Value,String>{
 let legacy:Value=serde_json::from_str(include_str!("../../../src/modules/photoEditor/catalog-legacy-v3.json")).map_err(|e|e.to_string())?;
 if old["version"]!=1{return Err("Неизвестная версия прежнего каталога".into());}
 let routes=old["routes"].as_array().ok_or("Нет прежних маршрутов")?;
 if routes.len()<8||routes.len()>legacy["routes"].as_array().unwrap().len(){return Err("Состав каталога не соответствует поддерживаемой миграции".into());}
 let mut seen=std::collections::HashSet::new();
 for r in routes{let id=r["id"].as_str().ok_or("Нет ID прежней модели")?;if !seen.insert(id){return Err("Повтор модели в прежнем каталоге".into());}let pinned=legacy["routes"].as_array().unwrap().iter().find(|p|p["id"]==id).ok_or("Неизвестный прежний маршрут")?;let mut previous=pinned.clone();if id.contains("nano-banana"){previous["operations"].as_array_mut().unwrap().retain(|v|v!="outpaint");}if !equivalent(r,pinned)&&!equivalent(r,&previous){return Err("Неизвестное изменение прежнего контракта; миграция остановлена".into());}}
 let mut result=catalog::bundled().clone();
 for (op,levels) in result["operationChoices"].as_object().unwrap().clone(){for (tier,ids) in levels.as_object().unwrap(){if let Some(selected)=old["recommendations"][&op][tier].as_str(){if ids.as_array().unwrap().iter().any(|cid|result["choices"].as_array().unwrap().iter().any(|c|c["id"]==*cid&&c["route"]==selected)){result["recommendations"][&op][tier]=json!(selected);}}}}
 let suffix=format!("{:x}",Sha256::digest(old.to_string().as_bytes()));result["id"]=json!(format!("editor-catalog-2026-10-12-v4-migration-{}",&suffix[..12]));validate(&result)?;Ok(result)
}
pub fn validate(candidate:&Value)->Result<(),String>{
 if candidate["version"]!=1||candidate["id"].as_str().is_none_or(|id|id.is_empty()||id.len()>160){return Err("Неизвестная версия каталога.".into());}
 let routes=candidate["routes"].as_array().ok_or("Нет маршрутов каталога")?;if routes.len()!=catalog::bundled()["routes"].as_array().unwrap().len(){return Err("Добавление маршрутов требует отдельного проверенного адаптера.".into());}
 let mut seen=std::collections::HashSet::new();for route in routes{let id=route["id"].as_str().ok_or("Нет ID модели")?;if !seen.insert(id){return Err("Модель повторяется.".into());}let pinned=catalog::route(id)?;
  // A recommendation update can change ranking and editorial notes only.
  // Prices, schemas, limits and media fields keep their audited contracts.
  if !equivalent(route,&pinned){return Err(format!("Контракт или тариф {id} изменился. Для применения сначала нужны адаптер и проверки; совет модели их не заменяет."));}
 }
 if !equivalent(&candidate["choices"],&catalog::bundled()["choices"])||!equivalent(&candidate["operationChoices"],&catalog::bundled()["operationChoices"]){return Err("Набор конфигураций изменён вне проверенного каталога.".into());}
 if let Some(profiles)=candidate.get("recommendations"){let profiles=profiles.as_object().ok_or("Неверные рекомендации")?;for(operation,values)in profiles{if !catalog::bundled()["routes"].as_array().unwrap().iter().any(|r|r["operations"].as_array().unwrap().iter().any(|op|op==operation)){return Err("Неизвестная операция рекомендаций.".into());}for profile in ["economy","balanced","quality"]{let id=values[profile].as_str().ok_or("Не заполнены три профиля")?;let route=catalog::route(id)?;if !route["operations"].as_array().unwrap().iter().any(|op|op==operation){return Err("Рекомендация несовместима с операцией.".into());}}}}
 Ok(())
}
pub fn preview(app:&tauri::AppHandle,candidate:&Value)->Result<Value,String>{validate(candidate)?;let current=active(app)?;Ok(json!({"from":current["id"],"to":candidate["id"],"before":current.get("recommendations"),"after":candidate.get("recommendations"),"contractsChanged":false,"pricesChanged":false}))}
pub fn apply(app:&tauri::AppHandle,candidate:&Value)->Result<Value,String>{validate(candidate)?;let current=active(app)?;if candidate["id"]==current["id"]&&candidate!=&current{return Err("Новая рекомендация должна иметь новый ID версии.".into());}let mut db=store::connection(app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 for version in [&current,candidate]{let id=version["id"].as_str().unwrap();let old:Option<String>=tx.query_row("SELECT catalog FROM editor_catalog_versions WHERE id=?1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;if old.is_some_and(|raw|serde_json::from_str::<Value>(&raw).ok().as_ref()!=Some(version)){return Err("ID уже связан с другой версией каталога.".into());}tx.execute("INSERT OR IGNORE INTO editor_catalog_versions(id,catalog,created_at_unix,active) VALUES(?1,?2,?3,0)",params![id,version.to_string(),unix_now()]).map_err(|e|e.to_string())?;}
 tx.execute("UPDATE editor_catalog_versions SET active=0",[]).map_err(|e|e.to_string())?;tx.execute("UPDATE editor_catalog_versions SET active=1 WHERE id=?1",[candidate["id"].as_str().unwrap()]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(candidate.clone())
}
pub fn versions(app:&tauri::AppHandle)->Result<Value,String>{let db=store::connection(app)?;let mut q=db.prepare("SELECT id,created_at_unix,active FROM editor_catalog_versions ORDER BY rowid DESC").map_err(|e|e.to_string())?;let rows=q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"createdAtUnix":r.get::<_,u64>(1)?,"active":r.get::<_,bool>(2)?}))).map_err(|e|e.to_string())?;Ok(json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?))}
pub fn rollback(app:&tauri::AppHandle,id:&str)->Result<Value,String>{let raw:String=store::connection(app)?.query_row("SELECT catalog FROM editor_catalog_versions WHERE id=?1",[id],|r|r.get(0)).map_err(|_|"Версия каталога не найдена")?;let value:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;let value=if validate(&value).is_ok(){value}else{migrate_legacy(&value)?};apply(app,&value)}
pub fn notes(app:&tauri::AppHandle,project:&str)->Result<Value,String>{let db=store::connection(app)?;let mut q=db.prepare("SELECT id,model_id,note,created_at_unix FROM editor_notes WHERE project_id=?1 ORDER BY rowid DESC").map_err(|e|e.to_string())?;let rows=q.query_map([project],|r|Ok(json!({"id":r.get::<_,String>(0)?,"modelId":r.get::<_,Option<String>>(1)?,"note":r.get::<_,String>(2)?,"createdAtUnix":r.get::<_,u64>(3)?}))).map_err(|e|e.to_string())?;Ok(json!(rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?))}
pub fn save_note(app:&tauri::AppHandle,project:&str,model:Option<&str>,text:&str)->Result<Value,String>{let text=text.trim();if text.is_empty()||text.chars().count()>4000{return Err("Заметка должна содержать от 1 до 4000 символов.".into());}if let Some(model)=model{catalog::route(model)?;}let id=stable_text_id("editor-note",&format!("{project}|{model:?}|{}",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));store::connection(app)?.execute("INSERT INTO editor_notes(id,project_id,model_id,note,created_at_unix) VALUES(?1,?2,?3,?4,?5)",params![id,project,model,text,unix_now()]).map_err(|e|e.to_string())?;notes(app,project)}
#[cfg(test)]mod tests{use super::*;#[test]fn advice_cannot_replace_provider_contract(){let mut c=catalog::bundled().clone();assert!(validate(&c).is_ok());c["routes"][0]["priceMicros"]=json!(1);assert!(validate(&c).is_err());}}
#[cfg(test)]mod numeric_tests{use super::*;
 #[test]fn webview_integer_float_roundtrip_does_not_change_contract(){assert!(equivalent(&json!({"minimum":0.0,"default":1.0}),&json!({"minimum":0,"default":1})));assert!(!equivalent(&json!({"default":1.5}),&json!({"default":1})));assert!(!equivalent(&json!({"default":1}),&json!({"default":1,"new":true})));}
 #[test]fn old_catalog_can_migrate_after_javascript_number_serialization(){fn ints(v:&mut Value){match v{Value::Number(n) if n.is_f64()&&n.as_f64().is_some_and(|x|x.fract()==0.&&x>=0.&&x<=9007199254740991.)=>*v=json!(n.as_f64().unwrap() as u64),Value::Array(v)=>for v in v{ints(v)},Value::Object(v)=>for v in v.values_mut(){ints(v)},_=>{}}}let mut old:Value=serde_json::from_str(include_str!("../../../src/modules/photoEditor/catalog-legacy-v3.json")).unwrap();ints(&mut old);assert!(migrate_legacy(&old).is_ok());}
}
