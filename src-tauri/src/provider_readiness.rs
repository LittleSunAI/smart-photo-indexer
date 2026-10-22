//! Credential authentication is shared across models; model capabilities remain separate.
use crate::*;
use serde_json::{json,Value};
use std::sync::OnceLock;

pub async fn authentication_gate()->tokio::sync::MutexGuard<'static,()>{
 static GATE:OnceLock<tokio::sync::Mutex<()>>=OnceLock::new();
 GATE.get_or_init(||tokio::sync::Mutex::new(())).lock().await
}

pub fn authenticated(db:&Connection,provider:&str,reference:&str)->Result<bool,String>{
 let mut statement=db.prepare("SELECT result FROM provider_readiness WHERE provider=?1 AND credential_ref=?2").map_err(|e|e.to_string())?;
 let rows=statement.query_map(params![provider,reference],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
 for row in rows{let value:Value=serde_json::from_str(&row.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;if value["authenticated"]==true{return Ok(true);}}
 Ok(false)
}
pub fn remember(app:&tauri::AppHandle,provider:&str,reference:&str)->Result<(),String>{
 let db=open_index_connection(app)?;
 db.execute("INSERT INTO provider_readiness(provider,credential_ref,model,result,checked_at_unix) VALUES(?1,?2,'__credential__',?3,?4) ON CONFLICT(provider,credential_ref,model) DO UPDATE SET result=excluded.result,checked_at_unix=excluded.checked_at_unix",params![provider,reference,json!({"provider":provider,"authenticated":true}).to_string(),unix_now()]).map_err(|e|e.to_string())?;
 Ok(())
}
pub async fn catalog()->Result<Value,String>{
 crate::provider_availability::require("openrouter")?;
 static CATALOG:OnceLock<tokio::sync::Mutex<Option<Value>>>=OnceLock::new();
 let mut saved=CATALOG.get_or_init(||tokio::sync::Mutex::new(None)).lock().await;
 if let Some(value)=saved.as_ref(){return Ok(value.clone());}
 let response=provider_transport::client(std::time::Duration::from_secs(30))?.get("https://openrouter.ai/api/v1/models").send().await.map_err(|_|"Каталог моделей сейчас недоступен. Сохранённый ключ не потерян.".to_string())?;
 if !response.status().is_success(){return Err("Не удалось получить каталог моделей. Сохранённый ключ не потерян.".into());}
 let value=provider_transport::bounded_json(response,16*1024*1024).await?;*saved=Some(value.clone());Ok(value)
}

/// Share one automatic check across buttons/windows. A failure is not retried on remount.
pub async fn ensure(app:tauri::AppHandle,provider:String,model:String,credential_ref:Option<String>)->Result<Value,String>{
 if let Some(status)=crate::provider_availability::status(&provider){return Ok(status);}
 type Checks=std::collections::HashMap<String,Result<Value,String>>;
 static CHECKS:OnceLock<tokio::sync::Mutex<Checks>>=OnceLock::new();
 let Some(reference)=credential_ref.clone() else{return Ok(json!({"ready":false,"message":"Добавьте ключ выбранного сервиса."}));};
 provider_transport::selected_model(&provider,&model)?;
 let cached=provider_transport::get_cached_provider_status(app.clone(),provider.clone(),model.clone(),credential_ref.clone()).await?;
 if cached["ready"]==true{return Ok(cached);}
 let key=format!("{provider}|{reference}|{model}");
 let authentication_key=format!("{provider}|{reference}|__credential__");
 let mut checks=CHECKS.get_or_init(||tokio::sync::Mutex::new(std::collections::HashMap::new())).lock().await;
 let owner=app.clone();let service=provider.clone();let id=reference.clone();
 let already_authenticated=tauri::async_runtime::spawn_blocking(move||authenticated(&open_index_connection(&owner)?,&service,&id)).await.map_err(|e|e.to_string())??;
 if !already_authenticated{if let Some(Err(error))=checks.get(&authentication_key){return Err(error.clone());}}
 if let Some(Err(error))=checks.get(&key){return Err(error.clone());}
 let result=provider_transport::get_ai_provider_status(app.clone(),provider.clone(),model,None,credential_ref).await;
 if let Err(error)=&result{let after=tauri::async_runtime::spawn_blocking(move||authenticated(&open_index_connection(&app)?,&provider,&reference)).await.map_err(|e|e.to_string())??;if !after{checks.insert(authentication_key,Err(error.clone()));}}
 checks.insert(key,result.clone());result
}

#[cfg(test)]
mod tests{
 use super::*;
 #[test]
 fn model_confirmation_proves_only_its_credential_and_provider(){
  let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE provider_readiness(provider TEXT,credential_ref TEXT,model TEXT,result TEXT,checked_at_unix INTEGER);").unwrap();
  assert!(!authenticated(&db,"fal","saved-key").unwrap());
  db.execute("INSERT INTO provider_readiness VALUES('fal','saved-key','one-model','{\"ready\":true}',1)",[]).unwrap();
  assert!(!authenticated(&db,"fal","saved-key").unwrap());
  db.execute("INSERT INTO provider_readiness VALUES('fal','saved-key','another-model','{\"authenticated\":true,\"ready\":true}',2)",[]).unwrap();
  assert!(authenticated(&db,"fal","saved-key").unwrap());
  assert!(!authenticated(&db,"fal","replacement-key").unwrap());
  assert!(!authenticated(&db,"openrouter","saved-key").unwrap());
  db.execute("DELETE FROM provider_readiness WHERE provider='fal' AND credential_ref='saved-key'",[]).unwrap();
  assert!(!authenticated(&db,"fal","saved-key").unwrap());
 }
}
