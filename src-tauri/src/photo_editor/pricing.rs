//! Authenticated GET-only tariff cache, scoped to profile and credential reference.
use crate::*;
use super::{catalog,store};
use serde_json::{json,Value};
pub fn cached(app:&tauri::AppHandle,reference:&str,route:&str)->Result<Option<Value>,String>{
 let db=store::connection(app)?;
 let raw:Option<String>=db.query_row("SELECT value FROM editor_pricing_cache WHERE credential_ref=?1 AND route=?2",params![reference,route],|r|r.get(0)).optional().map_err(|e|e.to_string())?;raw.map(|v|serde_json::from_str(&v).map_err(|e|e.to_string())).transpose()
}
pub async fn refresh(app:&tauri::AppHandle,reference:&str)->Result<Value,String>{
 let routes=catalog::bundled()["routes"].as_array().ok_or("Нет каталога")?;let now=unix_now();let mut old=Vec::new();
 for route in routes{if let Some(v)=cached(app,reference,route["id"].as_str().unwrap())?{old.push(v);}}
 if old.len()==routes.len()&&old.iter().all(|v|v["checkedAtUnix"].as_u64().is_some_and(|t|now.saturating_sub(t)<3600)){return Ok(json!(old));}
 let key=credentials::unlock(app,"fal",reference)?;
 let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(30)).build().map_err(|_|"Не удалось проверить тарифы")?;
 let ids=routes.iter().filter_map(|r|r["id"].as_str()).collect::<Vec<_>>().join(",");
 let response=client.get("https://api.fal.ai/v1/models/pricing").query(&[("endpoint_id",ids)]).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Тарифы временно недоступны; последняя оценка сохранена")?;
 if !response.status().is_success(){return Err("Не удалось обновить тарифы; запуск использует последнюю опубликованную оценку".into());}
 let result=provider_transport::bounded_json(response,1024*1024).await?;let mut records=Vec::new();
 for rate in result["prices"].as_array().ok_or("Нет тарифов в ответе")?{
  let id=rate["endpoint_id"].as_str().ok_or("Нет ID тарифа")?;if !routes.iter().any(|r|r["id"]==id){continue;}
  let price=rate["unit_price"].as_f64().filter(|v|v.is_finite()&&*v>=0.&&*v<=1000.).ok_or("Неверная цена")?;
  if rate["currency"]!="USD"{continue;}
  let value=json!({"route":id,"unitPrice":price,"unit":rate["unit"],"currency":"USD","checkedAtUnix":now});
  records.push(value);
 }
 // One short writer transaction on the blocking pool; never stall the async
 // HTTP runtime while a mask or draft is being committed in another thread.
 let app=app.clone();let reference=reference.to_string();
 tauri::async_runtime::spawn_blocking(move||{
  let mut db=store::connection(&app)?;let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|format!("Обновление тарифов: {e}"))?;
  for value in &records{tx.execute("INSERT INTO editor_pricing_cache(credential_ref,route,value,checked_at) VALUES(?1,?2,?3,?4) ON CONFLICT(credential_ref,route) DO UPDATE SET value=excluded.value,checked_at=excluded.checked_at",params![reference,value["route"].as_str(),value.to_string(),now]).map_err(|e|format!("Запись тарифа: {e}"))?;}
  tx.commit().map_err(|e|e.to_string())?;Ok::<Value,String>(json!(records))
 }).await.map_err(|e|e.to_string())?
}
