//! Native-only transports. Credentials and encoded frames never pass through a WebView.
use crate::*;
use serde_json::{json,Value};
use std::sync::OnceLock;
use std::time::{Duration,Instant};
use zeroize::Zeroizing;

pub fn analysis_contract()->&'static Value{
    static CONTRACT:OnceLock<Value>=OnceLock::new();CONTRACT.get_or_init(||serde_json::from_str(include_str!("../../src/shared/ai/analysis-contract.json")).expect("Compiled analysis contract must be valid"))
}
pub fn model_catalog()->&'static Value{
    static CATALOG:OnceLock<Value>=OnceLock::new();CATALOG.get_or_init(||serde_json::from_str(include_str!("../../src/shared/ai/model-catalog.json")).expect("Compiled model catalog must be valid"))
}
pub fn quick_tags_contract()->&'static Value{
    static CONTRACT:OnceLock<Value>=OnceLock::new();CONTRACT.get_or_init(||serde_json::from_str(include_str!("../../src/shared/ai/quick-tags-contract.json")).expect("Compiled quick tag contract must be valid"))
}
pub fn fast_summary_contract()->&'static Value{
    static CONTRACT:OnceLock<Value>=OnceLock::new();CONTRACT.get_or_init(||serde_json::from_str(include_str!("../../src/shared/ai/fast-summary-contract.json")).expect("Compiled fast summary contract must be valid"))
}
pub fn selected_model(provider:&str,model:&str)->Result<Value,String>{
    if provider=="fal" && std::env::var("SMART_PHOTO_CLOUD_EVAL").as_deref()==Ok("1") && std::env::var_os("SMART_PHOTO_DATA_DIR").is_some(){
        let candidates:Value=serde_json::from_str(include_str!("../../src/shared/ai/cloud-evaluation-candidates.json")).map_err(|e|e.to_string())?;
        if let Some(candidate)=candidates.as_array().and_then(|items|items.iter().find(|item|item["id"]==model)){return Ok(candidate.clone());}
    }
    if provider=="local" && std::env::var("SMART_PHOTO_LOCAL_EVAL").as_deref()==Ok("1") && std::env::var_os("SMART_PHOTO_DATA_DIR").is_some(){
        let candidates:Value=serde_json::from_str(include_str!("../../src/shared/ai/local-evaluation-candidates.json")).map_err(|e|e.to_string())?;
        if let Some(candidate)=candidates.as_array().and_then(|items|items.iter().find(|item|item["id"]==model)){return Ok(candidate.clone());}
    }
    if matches!(provider,"fal"|"openrouter"){if let Some(row)=model_catalog()["legacyCloud"].as_array().and_then(|rows|rows.iter().find(|row|row["id"]==model)){return Ok(row.clone());}}
    model_catalog()[provider].as_array().and_then(|models|models.iter().find(|item|item["id"].as_str()==Some(model))).cloned().ok_or("Выбранной модели нет в поддерживаемом списке этого сервиса.".into())
}
pub fn local_endpoint(endpoint:Option<&str>)->Result<reqwest::Url,String>{
    let endpoint=endpoint.unwrap_or("http://127.0.0.1:11434");let url=reqwest::Url::parse(endpoint).map_err(|_|"Invalid local endpoint.".to_string())?;
    if url.scheme()!="http" || ![Some("127.0.0.1"),Some("localhost"),Some("[::1]")].contains(&url.host_str()) || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some(){return Err("Local inference requires a loopback HTTP endpoint.".into());}
    Ok(url)
}
pub(crate) fn client(timeout:Duration)->Result<reqwest::Client,String>{
    build_client(timeout,false)
}
fn local_client(timeout:Duration)->Result<reqwest::Client,String>{build_client(timeout,true)}
fn build_client(timeout:Duration,loopback:bool)->Result<reqwest::Client,String>{
    let builder=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).connect_timeout(Duration::from_secs(15)).timeout(timeout);
    (if loopback{builder.no_proxy()}else{builder}).build().map_err(|_|"Could not initialize provider transport.".into())
}
fn inference_timeout(provider:&str)->Duration{Duration::from_secs(if provider=="local"{900}else{90})}
pub(crate) fn inference_network_message(provider:&str,error:&reqwest::Error)->String{
    if provider!="local"{return "Provider network request failed.".into();}
    if error.is_connect(){"Не удалось соединиться с локальным сервисом Ollama. Проверьте, что он запущен.".into()}
    else if error.is_timeout(){"Локальная модель не завершила запрос за 15 минут. Можно запустить обработку заново.".into()}
    else{"Соединение с локальной моделью прервано. Можно запустить обработку заново.".into()}
}
pub async fn bounded_json(response:reqwest::Response,limit:usize)->Result<Value,String>{
    read_json(response,limit,None).await
}
async fn read_json(mut response:reqwest::Response,limit:usize,provider:Option<&str>)->Result<Value,String>{
    if response.content_length().is_some_and(|size|size>limit as u64){return Err("Provider response exceeded its size limit.".into());}
    let mut bytes=Vec::new();
    while let Some(chunk)=response.chunk().await.map_err(|error|provider.map(|p|inference_network_message(p,&error)).unwrap_or_else(||"Provider response could not be read.".to_string()))?{
        if bytes.len()+chunk.len()>limit{return Err("Provider response exceeded its size limit.".into());}bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_|"Provider returned malformed JSON.".into())
}
fn http_error(code:reqwest::StatusCode)->String{
    match code.as_u16(){401|403=>"Ключ недействителен или не имеет доступа к выбранной модели.".into(),402=>"Баланс выбранного сервиса исчерпан.".into(),429=>"Сервис ограничил частоту запросов; обработка ждёт повторного допуска.".into(),400|422=>"Сервис не принимает параметры выбранной модели; неизменный запрос не повторяется.".into(),_=>format!("Provider returned HTTP {}.",code.as_u16())}
}

#[derive(Clone)]
pub(super) struct ProviderReply { pub raw:Value,pub cost_micros:Option<i64>,pub usage:Value }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) enum FailureGroup { Credential, Model }
#[derive(Clone,Debug)]
pub(super) struct ProviderFailure { pub message:String,pub retryable:bool,pub uncertain:bool,pub retry_after:u64,pub group:Option<FailureGroup> }
impl ProviderFailure {
    // A local disconnect/cancel has no paid remote receipt to reconcile. Keep
    // the diagnostic, but make it terminal so an explicit new command can run.
    pub fn for_provider(mut self,provider:&str)->Self{
        if provider=="local"&&self.uncertain{self.uncertain=false;self.retryable=false;}
        self
    }
    pub fn definitive(message:impl Into<String>)->Self{Self{message:message.into(),retryable:false,uncertain:false,retry_after:1,group:None}}
    pub fn model_rejected(message:impl Into<String>)->Self{let mut failure=Self::definitive(message);let lower=failure.message.to_lowercase();if lower.contains("zdr violation") || lower.contains("guardrail restrictions") || lower.contains("no endpoints found") || lower.contains("model is not available") || lower.contains("model not found"){failure.group=Some(FailureGroup::Model);}failure}
    pub fn unknown(message:impl Into<String>)->Self{Self{message:message.into(),retryable:false,uncertain:true,retry_after:1,group:None}}
}
fn failure_group(code:u16)->Option<FailureGroup>{match code{401|402|403=>Some(FailureGroup::Credential),404=>Some(FailureGroup::Model),_=>None}}
fn classified_http(response:&reqwest::Response)->ProviderFailure{
    let code=response.status();
    let retry_after=response.headers().get("retry-after").and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok().or_else(||httpdate::parse_http_date(v).ok().map(|date|date.duration_since(SystemTime::now()).unwrap_or_default().as_secs().saturating_add(1)))).unwrap_or(1);
    ProviderFailure{message:http_error(code),retryable:code.as_u16()==429,uncertain:code.is_server_error(),retry_after,group:failure_group(code.as_u16())}
}
pub(super) fn reply_from_raw(provider:&str,raw:Value)->ProviderReply{
    let usage=if provider=="local"{json!({"promptTokens":raw["prompt_eval_count"],"completionTokens":raw["eval_count"],"durationNs":raw["total_duration"],"loadDurationNs":raw["load_duration"],"promptEvalDurationNs":raw["prompt_eval_duration"],"evalDurationNs":raw["eval_duration"]})}else{raw["usage"].clone()};
    let cost=if provider=="local" || provider=="controlled-test"{Some(0)}else{usage["cost"].as_f64().filter(|v|v.is_finite() && *v>=0.0 && *v<10000.0).map(|v|(v*1_000_000.0).ceil() as i64)};
    ProviderReply{raw,cost_micros:cost,usage}
}
pub(super) fn decode_reply(provider:&str,raw:&Value,contract:Option<&Value>)->Result<Value,String>{
    let content=match provider {
        "controlled-test"=>return processing::validate_result(raw.clone()),
        "fal"=>raw["output"].as_str().ok_or("Fal returned no output text.")?,
        "openrouter"=>{
            if raw["choices"][0]["finish_reason"]!="stop"{return Err("The provider did not finish the description normally; the partial output was preserved, not applied.".into());}
            raw["choices"][0]["message"]["content"].as_str().ok_or("OpenRouter returned no output text.")?
        },
        "local"=>{
            if raw["done"]!=true || raw["done_reason"]!="stop"{return Err("Local output was incomplete; increase the output/context limit explicitly.".into());}
            raw["message"]["content"].as_str().ok_or("Ollama returned no output text.")?
        },
        _=>return Err("Unknown response provider.".into()),
    };
    let content=if contract.is_some_and(|contract|contract["allowMarkdownWrapper"]==true){unwrap_json_fence(content)}else{content};
    let value=serde_json::from_str(content).map_err(|_|"The model did not return a complete JSON description. Its raw output was preserved.".to_string())?;
    let value=processing::validate_result(value)?;
    // Recovery validates against the job's frozen policy.
    if matches!(contract.and_then(|c|c["version"].as_str()),Some("archive-detailed-v2"|"archive-detailed-v3"|"archive-quick-tags-v1"|"archive-fast-summary-v1")){
        for field in ["description","scene"]{
            let text=value[field].as_str().ok_or("Text field is missing.")?;
            let letters=text.chars().filter(|c|c.is_alphabetic()).count();
            let russian=text.chars().filter(|c|matches!(c,'А'..='я'|'Ё'|'ё')).count();
            if letters>40 && russian*100<letters*80{return Err("Ответ не соответствует требованию русского языка. Исходный ответ сохранён; непригодный текст не записан в активные метаданные.".into());}
        }
        for field in ["tags","objects"]{
            for text in value[field].as_array().ok_or("List field is missing.")?.iter().filter_map(Value::as_str){
                if text.chars().any(|c|matches!(c,'А'..='я'|'Ё'|'ё')) && text.chars().any(|c|('\u{3400}'..='\u{9fff}').contains(&c)){return Err("Модель смешала языки внутри поискового тега или названия объекта. Исходный ответ сохранён для проверки.".into());}
            }
        }
    }
    if contract.and_then(|c|c["version"].as_str())==Some("archive-quick-tags-v1") && value["tags"].as_array().is_none_or(|tags|tags.is_empty()){
        return Err("Модель не вернула поисковых тегов. Пустая разметка не сохранена.".into());
    }
    Ok(value)
}

fn unwrap_json_fence(content:&str)->&str{
    let trimmed=content.trim();
    if let Some((opening,body))=trimmed.split_once('\n'){
        if opening.trim()=="```json" || opening.trim()=="```"{
            if let Some(body)=body.trim_end().strip_suffix("```"){return body.trim();}
        }
    }
    content
}

#[derive(Clone)]
pub(super) struct PreparedFrame { base64:String,data_uri:String,_memory:std::sync::Arc<tokio::sync::OwnedSemaphorePermit> }
pub(super) async fn prepare_frame(w:&processing::Work)->Result<PreparedFrame,String>{
    static PREPARE:OnceLock<tokio::sync::Semaphore>=OnceLock::new();
    // One decoder (including one <=12 MiB encoded frame waiting for memory)
    // plus 96 MiB retained base64/request buffers. Reserve the actual size before
    // constructing strings so small frames can fill all requested network slots.
    static INPUT_MEMORY:OnceLock<std::sync::Arc<tokio::sync::Semaphore>>=OnceLock::new();
    let _permit=PREPARE.get_or_init(||tokio::sync::Semaphore::new(1)).acquire().await.map_err(|e|e.to_string())?;
    let worker=file_safety::image_worker().acquire_owned().await.map_err(|e|e.to_string())?;
    let w=w.clone();
    let (bytes,mime)=tauri::async_runtime::spawn_blocking(move||{
        if compute_file_sha256(Path::new(&w.path))?!=w.sha{return Err("Source changed before frame preparation.".into());}
        let mut image=file_safety::decode_bounded(Path::new(&w.path))?;
        if image.width().max(image.height())>w.config.image_max_side{image=image.resize(w.config.image_max_side,w.config.image_max_side,image::imageops::FilterType::Triangle);}
        let mut bytes=Vec::new();
        let mime=if image.color().has_alpha(){image.write_to(&mut std::io::Cursor::new(&mut bytes),image::ImageFormat::Png).map_err(|e|e.to_string())?;"image/png"}
            else{image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes,90).encode_image(&image).map_err(|e|e.to_string())?;"image/jpeg"};
        if bytes.len()>12*1024*1024{return Err("Prepared frame exceeds the bounded upload limit.".into());}
        if compute_file_sha256(Path::new(&w.path))?!=w.sha{return Err("Source changed during frame preparation.".into());}
        Ok::<_,String>((bytes,mime))
    }).await.map_err(|e|e.to_string())??;
    drop(worker);
    let actual=bytes.len().saturating_mul(6).saturating_add(1024) as u32;
    let memory=INPUT_MEMORY.get_or_init(||std::sync::Arc::new(tokio::sync::Semaphore::new(96*1024*1024))).clone()
        .acquire_many_owned(actual).await.map_err(|e|e.to_string())?;
    tauri::async_runtime::spawn_blocking(move||{
        let base64=general_purpose::STANDARD.encode(&bytes);let data_uri=format!("data:{mime};base64,{base64}");
        Ok(PreparedFrame{base64,data_uri,_memory:std::sync::Arc::new(memory)})
    }).await.map_err(|e|e.to_string())?
}
pub(super) fn prompts(w:&processing::Work,candidates:&[Value])->Result<(String,String,Value),String>{
    let contract=w.config.analysis_contract.as_ref().ok_or("The job has no frozen analysis contract.")?;
    let mut system=contract["system"].as_str().ok_or("Invalid frozen system instruction.")?.to_string();
    if let Some(language)=contract["languageInstruction"].as_str(){system.push_str("\n\n");system.push_str(language);}
    let mut prompt=contract["user"].as_str().ok_or("Invalid frozen user instruction.")?.to_string();
    if let Some(detail)=contract["descriptionInstruction"].as_str(){prompt.push_str("\n\n");prompt.push_str(detail);}
    if let Some(language)=contract["languageInstruction"].as_str(){prompt.push_str("\n\n");prompt.push_str(language);}
    if !candidates.is_empty(){
        prompt.push_str("\n\n");prompt.push_str(contract["verification"].as_str().ok_or("Invalid verification instruction.")?);
        prompt.push_str("\n\n<untrusted_candidates_json>\n");prompt.push_str(&serde_json::to_string(candidates).map_err(|e|e.to_string())?);prompt.push_str("\n</untrusted_candidates_json>");
    }
    let schema=contract["schema"].clone();prompt.push_str("\n\nJSON Schema:\n");prompt.push_str(&schema.to_string());
    if system.len()+prompt.len()>512*1024{return Err("Candidate descriptions exceed the context preparation limit; nothing was silently truncated.".into());}
    Ok((system,prompt,schema))
}
pub(super) fn fal_remote_url(value:&str,id:&str,kind:&str)->Result<reqwest::Url,String>{
    if id.is_empty() || id.len()>160 || !id.bytes().all(|b|b.is_ascii_alphanumeric() || b==b'-' || b==b'_'){return Err("Invalid Fal request identifier.".into());}
    let url=reqwest::Url::parse(value).map_err(|_|"Invalid Fal tracking URL.".to_string())?;
    let marker=format!("/requests/{id}");
    let suffix=match kind{"status"=>"/status","cancel"=>"/cancel","response"=>"",_=>return Err("Invalid Fal operation.".into())};
    let expected=format!("{marker}{suffix}");
    // Returned paths may omit router/vision. Bind every URL to the exact id and operation.
    let valid_path=url.path().starts_with("/openrouter/") && (url.path().ends_with(&expected) || kind=="response" && url.path().ends_with(&format!("{marker}/response")));
    if url.scheme()!="https" || url.host_str()!=Some("queue.fal.run") || url.port_or_known_default()!=Some(443) || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() || !valid_path{return Err("Untrusted Fal tracking URL was rejected before sending a credential.".into());}Ok(url)
}

async fn unlocked(app:&tauri::AppHandle,w:&processing::Work)->Result<Zeroizing<String>,ProviderFailure>{
    let reference=w.credential_ref.clone().ok_or_else(||ProviderFailure::definitive("Add a credential for this provider."))?;
    let app=app.clone();let provider=w.config.provider_mode.clone();
    tauri::async_runtime::spawn_blocking(move||credentials::unlock(&app,&provider,&reference)).await.map_err(|e|ProviderFailure::definitive(e.to_string()))?.map_err(ProviderFailure::definitive)
}
pub(super) async fn infer(app:&tauri::AppHandle,w:&processing::Work,attempt:&str,candidates:&[Value],frame:Option<&PreparedFrame>)->Result<ProviderReply,ProviderFailure>{
    let provider=w.config.provider_mode.as_str();
    crate::provider_availability::require(provider).map_err(ProviderFailure::definitive)?;
    let client=if provider=="local"{local_client(inference_timeout(provider))}else{client(inference_timeout(provider))}.map_err(ProviderFailure::definitive)?;
    let key=if provider=="local"{None}else{Some(unlocked(app,w).await?)};
    if provider=="fal"{
        if let Some(remote)=processing::load_remote(app,attempt).await.map_err(ProviderFailure::definitive)?{return poll_fal(&client,key.as_ref().unwrap(),&remote).await;}
    }
    let frame=frame.ok_or_else(||ProviderFailure::definitive("Prepared frame is unavailable."))?;
    let (system,prompt,schema)=prompts(w,candidates).map_err(ProviderFailure::definitive)?;
    let request=match provider {
        "local"=>{
            let endpoint=local_endpoint(w.config.endpoint.as_deref()).map_err(ProviderFailure::definitive)?;
            let context=((system.len()+prompt.len()) as u32 + w.config.max_tokens + 4096).next_power_of_two().clamp(8192,32768);
            if system.len()+prompt.len()>context.saturating_sub(w.config.max_tokens+4096) as usize{return Err(ProviderFailure::definitive("Local candidate context is too large; use fewer passes or explicitly larger context."));}
            let threads=std::thread::available_parallelism().map(|v|v.get().saturating_sub(2).clamp(1,4)).unwrap_or(1);
            client.post(endpoint.join("/api/chat").map_err(|e|ProviderFailure::definitive(e.to_string()))?).json(&json!({"model":w.config.model,"messages":[{"role":"system","content":system},{"role":"user","content":prompt,"images":[frame.base64]}],"format":schema,"stream":false,"think":w.config.local_reasoning,"keep_alive":"2m","options":{"num_predict":w.config.max_tokens,"num_ctx":context,"num_thread":threads,"num_gpu":0,"temperature":0.15}}))
        },
        "openrouter"=>client.post("https://openrouter.ai/api/v1/chat/completions").bearer_auth(key.as_ref().unwrap().as_str()).json(&json!({"model":w.config.model,"messages":[{"role":"system","content":system},{"role":"user","content":[{"type":"text","text":prompt},{"type":"image_url","image_url":{"url":frame.data_uri}}]}],"stream":false,"max_tokens":w.config.max_tokens,"response_format":{"type":"json_schema","json_schema":{"name":"photo_analysis","strict":true,"schema":schema}},"provider":{"require_parameters":true,"allow_fallbacks":false}})),
        "fal"=>client.post("https://queue.fal.run/openrouter/router/vision").header("Authorization",format!("Key {}",key.as_ref().unwrap().as_str())).header("X-Fal-No-Retry","1").json(&json!({"model":w.config.model,"image_urls":[frame.data_uri],"system_prompt":system,"prompt":prompt,"max_tokens":w.config.max_tokens,"reasoning":w.config.fal_reasoning,"temperature":0.15,"enable_web_search":false})),
        _=>return Err(ProviderFailure::definitive("Unknown inference provider.")),
    };
    let response=processing::send_with_admission(app,w,attempt,request).await.map_err(ProviderFailure::unknown)?;
    if matches!(response.status().as_u16(),401|403)&&provider!="local"{if let Some(reference)=&w.credential_ref{let _=invalidate_readiness(app,provider,reference);}}
    if !response.status().is_success(){
        if provider=="local"{
            let code=response.status();
            let detail=bounded_json(response,65536).await.ok().and_then(|raw|raw["error"].as_str().map(|text|text.chars().filter(|c|!c.is_control()).take(1000).collect::<String>()));
            return Err(ProviderFailure::definitive(format!("Local runtime HTTP {}: {}",code.as_u16(),detail.unwrap_or_else(||"The model could not complete this request.".into()))));
        }
        return Err(classified_http(&response));
    }
    let raw=read_json(response,1024*1024,Some(provider)).await.map_err(ProviderFailure::unknown)?;
    if provider!="fal"{return Ok(reply_from_raw(provider,raw));}
    let remote=processing::RemoteRequest::from_submit(&raw).map_err(ProviderFailure::unknown)?;
    processing::persist_remote(app,attempt,&remote).await.map_err(ProviderFailure::unknown)?;
    // Persist first, then poll. Closing a window or process never loses the remote id.
    if processing::is_cancelled(app,w).await.unwrap_or(true){let _=cancel_fal(&client,key.as_ref().unwrap(),&remote).await;return Err(ProviderFailure::unknown("Cancelled after Fal submit; remote outcome requires reconciliation."));}
    poll_fal(&client,key.as_ref().unwrap(),&remote).await
}
async fn poll_fal(client:&reqwest::Client,key:&str,remote:&processing::RemoteRequest)->Result<ProviderReply,ProviderFailure>{
    let status=fal_remote_url(&remote.status_url,&remote.id,"status").map_err(ProviderFailure::definitive)?;
    let result=fal_remote_url(&remote.response_url,&remote.id,"response").map_err(ProviderFailure::definitive)?;
    let deadline=Instant::now()+Duration::from_secs(900);let mut delay=1;
    loop {
        if Instant::now()>deadline{return Err(ProviderFailure::unknown("Fal request remains unresolved after the polling deadline. Resume polls the same request id."));}
        match client.get(status.clone()).header("Authorization",format!("Key {key}")).send().await {
            Ok(response) if response.status().is_success()=>{
                let state=bounded_json(response,1024*1024).await.map_err(ProviderFailure::unknown)?;
                match state["status"].as_str(){
                    Some("COMPLETED")=>{
                        match client.get(result.clone()).header("Authorization",format!("Key {key}")).send().await {
                            Ok(response) if response.status().is_success()=>return Ok(reply_from_raw("fal",bounded_json(response,1024*1024).await.map_err(ProviderFailure::unknown)?)),
                            Ok(response) if matches!(response.status().as_u16(),400|422)=>{
                                let code=response.status();
                                let detail=bounded_json(response,65536).await.ok().and_then(|raw|raw["detail"].as_str().map(|text|text.chars().filter(|c|!c.is_control()).take(1000).collect::<String>()));
                                return Err(ProviderFailure::model_rejected(format!("Fal completed request rejected (HTTP {}): {}",code.as_u16(),detail.unwrap_or_else(||"Unsupported model parameters.".into()))));
                            },
                            Ok(response) if !response.status().is_server_error() && response.status().as_u16()!=429=>return Err({let mut failure=ProviderFailure::unknown(http_error(response.status()));failure.group=failure_group(response.status().as_u16());failure}),
                            Ok(response)=>{delay=classified_http(&response).retry_after.max(1);},
                            Err(_)=>{delay=delay.saturating_mul(2).min(30);}
                        }
                    },
                    Some("IN_QUEUE"|"IN_PROGRESS")=>delay=1,
                    _=>return Err(ProviderFailure::unknown("Fal returned an unrecognized queue state; no new inference was submitted.")),
                }
            },
            Ok(response) if response.status().is_server_error() || response.status().as_u16()==429=>delay=classified_http(&response).retry_after.max(1),
            Ok(response)=>return Err({let mut failure=ProviderFailure::unknown(http_error(response.status()));failure.group=failure_group(response.status().as_u16());failure}),
            Err(_)=>delay=delay.saturating_mul(2).min(30),
        }
        tokio::time::sleep(Duration::from_secs(delay)).await;
    }
}
pub(super) async fn reconcile_attempt(app:&tauri::AppHandle,w:&processing::Work,attempt:&str)->Result<Option<Result<ProviderReply,ProviderFailure>>,String>{
    if w.config.provider_mode!="fal"{return Ok(None);}
    let Some(remote)=processing::load_remote(app,attempt).await? else{return Ok(None);};
    let key=unlocked(app,w).await.map_err(|e|e.message)?;
    let client=client(Duration::from_secs(30))?;
    let status=fal_remote_url(&remote.status_url,&remote.id,"status")?;
    let response=client.get(status).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Не удалось сверить состояние запроса Fal.".to_string())?;
    if !response.status().is_success(){return Err(http_error(response.status()));}
    let state=bounded_json(response,65536).await?;
    if state["status"]!="COMPLETED"{return Ok(None);}
    if let Some(message)=state["error"].as_str().filter(|s|!s.trim().is_empty()){processing::settle_terminal_remote_failure(app,attempt,&format!("Fal подтвердил завершение с ошибкой: {message}. Расход неизвестен; резерв сохранён.")).await?;return Ok(None);}
    let result=fal_remote_url(&remote.response_url,&remote.id,"response")?;
    let response=client.get(result).header("Authorization",format!("Key {}",key.as_str())).send().await.map_err(|_|"Не удалось получить сохранённый ответ Fal.".to_string())?;
    if response.status().is_success(){return Ok(Some(Ok(reply_from_raw("fal",bounded_json(response,1024*1024).await?))));}
    if matches!(response.status().as_u16(),400|422){
        let code=response.status();let detail=bounded_json(response,65536).await.ok().and_then(|raw|raw["detail"].as_str().map(|text|text.chars().filter(|c|!c.is_control()).take(1000).collect::<String>()));
        processing::settle_terminal_remote_failure(app,attempt,&format!("Fal завершил запрос с HTTP {}: {}. Фактический расход неизвестен, резерв сохранён.",code.as_u16(),detail.unwrap_or_default())).await?;return Ok(None);
    }
    Err(http_error(response.status()))
}

async fn cancel_fal(client:&reqwest::Client,key:&str,remote:&processing::RemoteRequest)->Result<u16,String>{
    let url=fal_remote_url(&remote.cancel_url,&remote.id,"cancel")?;
    client.put(url).header("Authorization",format!("Key {key}")).send().await.map(|r|r.status().as_u16()).map_err(|_|"Fal cancellation could not be confirmed.".into())
}
pub(super) async fn cancel_attempt(app:&tauri::AppHandle,w:&processing::Work,attempt:&str)->Result<(),String>{
    if w.config.provider_mode!="fal"{return Ok(());}
    if let Some(remote)=processing::load_remote(app,attempt).await?{
        let key=unlocked(app,w).await.map_err(|e|e.message)?;
        let code=cancel_fal(&client(Duration::from_secs(15))?,&key,&remote).await?;
        processing::record_cancel(app,attempt,code).await?;
    }Ok(())
}

#[tauri::command]
pub async fn get_ai_provider_status(app:tauri::AppHandle,provider:String,model:String,endpoint:Option<String>,credential_ref:Option<String>)->Result<Value,String>{
    if let Some(status)=crate::provider_availability::status(&provider){return Ok(status);}
    let result=match fetch_ai_provider_status(app.clone(),provider.clone(),model.clone(),endpoint,credential_ref.clone()).await{Ok(value)=>value,Err(error)=>{if error.contains("отклонил ключ")||error.contains("нет доступа к проверке")||error.contains("Ключ недействителен"){if let Some(reference)=&credential_ref{invalidate_readiness(&app,&provider,reference)?;}}return Err(error);}};
    if provider!="local"&&result["ready"]==true&&result["authenticated"]==true{if let Some(reference)=credential_ref{let owner=app.clone();let value=result.clone();let name=model.clone();tauri::async_runtime::spawn_blocking(move||{let mut db=open_index_connection(&owner)?;run_index_migrations(&mut db)?;db.execute("INSERT INTO provider_readiness(provider,credential_ref,model,result,checked_at_unix) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(provider,credential_ref,model) DO UPDATE SET result=excluded.result,checked_at_unix=excluded.checked_at_unix",params![provider,reference,name,value.to_string(),unix_now()]).map_err(|e|e.to_string())?;Ok::<_,String>(())}).await.map_err(|e|e.to_string())??;}}
    use tauri::Emitter;let _=app.emit("smart-photo-indexer:provider-readiness-changed",json!({}));
    Ok(result)
}
pub(crate) fn invalidate_readiness(app:&tauri::AppHandle,provider:&str,reference:&str)->Result<(),String>{use tauri::Emitter;let db=open_index_connection(app)?;db.execute("DELETE FROM provider_readiness WHERE provider=?1 AND credential_ref=?2",params![provider,reference]).map_err(|e|e.to_string())?;app.emit("smart-photo-indexer:provider-readiness-invalidated",json!({"provider":provider,"credentialRef":reference})).map_err(|e|e.to_string())?;Ok(())}
#[tauri::command]
pub async fn get_cached_provider_status(app:tauri::AppHandle,provider:String,model:String,credential_ref:Option<String>)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||{
    if let Some(status)=crate::provider_availability::status(&provider){return Ok(status);}
    let Some(reference)=credential_ref else{return Ok(Value::Null);};credentials::unlock(&app,&provider,&reference)?;let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let raw:Option<String>=db.query_row("SELECT result FROM provider_readiness WHERE provider=?1 AND credential_ref=?2 AND model=?3",params![provider,reference,model],|r|r.get(0)).optional().map_err(|e|e.to_string())?;raw.map(|raw|serde_json::from_str(&raw).map_err(|e|e.to_string())).unwrap_or(Ok(Value::Null))
}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn ensure_ai_provider_ready(app:tauri::AppHandle,provider:String,model:String,endpoint:Option<String>,credential_ref:Option<String>)->Result<Value,String>{
    if let Some(status)=crate::provider_availability::status(&provider){return Ok(status);}
    if provider!="local"{return provider_readiness::ensure(app,provider,model,credential_ref).await;}
    get_ai_provider_status(app,provider,model,endpoint,credential_ref).await
}
async fn fetch_ai_provider_status(app:tauri::AppHandle,provider:String,model:String,endpoint:Option<String>,credential_ref:Option<String>)->Result<Value,String>{
    let selected=selected_model(&provider,&model)?;
    let client=if provider=="local"{local_client(Duration::from_secs(30))}else{client(Duration::from_secs(30))}?;
    if provider=="local"{
        let endpoint=local_endpoint(endpoint.as_deref())?;
        let response=client.get(endpoint.join("/api/tags").map_err(|e|e.to_string())?).send().await.map_err(|_|"Ollama не запущен. Запустите локальный runtime явно.".to_string())?;
        if !response.status().is_success(){return Err(http_error(response.status()));}
        let tags=bounded_json(response,1024*1024).await?;
        let installed=tags["models"].as_array().and_then(|models|models.iter().find(|item|item["name"]==model || item["model"]==model));
        let Some(installed)=installed else{return Ok(json!({"provider":provider,"model":model,"ready":false,"modelInstalled":false,"downloadBytes":selected["downloadBytes"],"message":"Выбранная модель не скачана. Скачивание запускается отдельной кнопкой."}));};
        if installed["size"].as_u64().is_none_or(|size|size>selected["downloadLimitBytes"].as_u64().unwrap_or(5_000_000_000)) || installed["digest"]!=selected["digest"]{return Err("Installed model differs from the verified packet. Inspect or download the selected version explicitly.".into());}
        let response=client.post(endpoint.join("/api/show").map_err(|e|e.to_string())?).json(&json!({"model":model})).send().await.map_err(|_|"Ollama model information is unavailable.".to_string())?;
        if !response.status().is_success(){return Err(http_error(response.status()));}
        let show=bounded_json(response,4*1024*1024).await?;
        let vision=show["capabilities"].as_array().is_some_and(|caps|caps.iter().any(|c|c=="vision"));
        return Ok(json!({"provider":provider,"model":model,"ready":vision,"modelInstalled":true,"vision":vision,"downloadBytes":selected["downloadBytes"],"installedBytes":installed["size"],"message":if vision{"Локальная vision-модель готова."}else{"Runtime не подтверждает распознавание изображений этой моделью."}}));
    }
    if provider!="openrouter" && provider!="fal"{return Err("Unknown cloud provider.".into());}
    let Some(reference)=credential_ref else{return Ok(json!({"provider":provider,"model":model,"ready":false,"message":"Добавьте ключ выбранного сервиса."}));};
    let unlock_app=app.clone();let unlock_provider=provider.clone();let unlock_reference=reference.clone();
    let credential:Zeroizing<String>=tauri::async_runtime::spawn_blocking(move||credentials::unlock(&unlock_app,&unlock_provider,&unlock_reference)).await.map_err(|e|e.to_string())??;
    let authentication_guard=provider_readiness::authentication_gate().await;
    let authenticated={let owner=app.clone();let service=provider.clone();let id=reference.clone();tauri::async_runtime::spawn_blocking(move||provider_readiness::authenticated(&open_index_connection(&owner)?,&service,&id)).await.map_err(|e|e.to_string())??};
    if !authenticated{
    let authentication=if provider=="fal"{
        client.get("https://api.fal.ai/v1/models?limit=1").header("Authorization",format!("Key {}",credential.as_str())).send().await
    }else{
        client.get("https://openrouter.ai/api/v1/key").bearer_auth(credential.as_str()).send().await
    }.map_err(|_|"Не удалось проверить ключ: сервис недоступен. Ключ сохранён, попробуйте позже.".to_string())?;
    if !authentication.status().is_success(){return Err(match authentication.status().as_u16(){
        401=>"Сервис отклонил ключ. Проверьте его или сохраните новый.",403=>"У ключа нет доступа к проверке сервиса.",
        402=>"Недостаточно средств или исчерпан лимит ключа.",429=>"Сервис ограничил частоту проверок. Повторите позже.",
        _=>"Сервис не подтвердил доступ по этому ключу. Повторите проверку позже.",
    }.into());}
    let _=bounded_json(authentication,1024*1024).await?;
    let owner=app.clone();let service=provider.clone();let id=reference.clone();tauri::async_runtime::spawn_blocking(move||provider_readiness::remember(&owner,&service,&id)).await.map_err(|e|e.to_string())??;
    }
    drop(authentication_guard);
    let catalog=provider_readiness::catalog().await?;
    let advertised=catalog["data"].as_array().and_then(|models|models.iter().find(|item|item["id"]==model)).ok_or("The selected model is no longer in the public catalog.".to_string())?;
    let vision=advertised["architecture"]["input_modalities"].as_array().is_some_and(|items|items.iter().any(|item|item=="image"));
    Ok(json!({"provider":provider,"model":model,"ready":vision,"authenticated":true,"vision":vision,"contextLength":advertised["context_length"],"maxOutputTokens":advertised["top_provider"]["max_completion_tokens"],"supportedParameters":advertised["supported_parameters"],"pricing":advertised["pricing"],"balanceConfirmed":false,"message":if vision{"Ключ принят сервисом; выбранная модель поддерживает изображения. Распознавание не запускалось."}else{"Ключ принят, но каталог не подтверждает поддержку изображений моделью."}}))
}

#[tauri::command]
pub async fn download_local_model(app:tauri::AppHandle,model:String,endpoint:Option<String>)->Result<Value,String>{
    static DOWNLOAD:OnceLock<tokio::sync::Semaphore>=OnceLock::new();
    let _permit=DOWNLOAD.get_or_init(||tokio::sync::Semaphore::new(1)).try_acquire().map_err(|_|"A selected model download is already in progress.".to_string())?;
    let selected=selected_model("local",&model)?;let endpoint=local_endpoint(endpoint.as_deref())?;
    let client=client(Duration::from_secs(3600))?;
    let (name,tag)=model.rsplit_once(':').ok_or("Invalid local packet id.")?;
    let response=client.get(format!("https://registry.ollama.ai/v2/library/{name}/manifests/{tag}")).send().await.map_err(|_|"Could not verify the model manifest.".to_string())?;
    if !response.status().is_success(){return Err(http_error(response.status()));}
    let bytes=response.bytes().await.map_err(|_|"Could not read the model manifest.".to_string())?;
    if bytes.len()>1024*1024{return Err("Model manifest exceeds its size limit.".into());}
    let digest=format!("{:x}",Sha256::digest(&bytes));let manifest:Value=serde_json::from_slice(&bytes).map_err(|_|"Invalid model manifest.".to_string())?;
    let config=manifest["config"]["size"].as_u64().ok_or("Manifest config size is missing.")?;
    let size=manifest["layers"].as_array().ok_or("Manifest layers are missing.")?.iter().try_fold(config,|total,layer|total.checked_add(layer["size"].as_u64()?).filter(|sum|*sum<=5_000_000_000)).ok_or("The complete model packet exceeds 5 GB or has invalid sizes.")?;
    if selected["digest"]!=digest || selected["downloadBytes"]!=size{return Err("The pinned model packet changed; no weights were requested.".into());}
    let mut response=local_client(Duration::from_secs(3600))?.post(endpoint.join("/api/pull").map_err(|e|e.to_string())?).json(&json!({"model":model,"stream":true})).send().await.map_err(|_|"Ollama is unavailable. Start it explicitly before downloading.".to_string())?;
    if !response.status().is_success(){return Err(http_error(response.status()));}
    let mut pending=Vec::new();let mut finished=false;
    while let Some(chunk)=response.chunk().await.map_err(|_|"Model download progress was interrupted; check installed status before retrying.".to_string())?{
        pending.extend_from_slice(&chunk);
        while let Some(newline)=pending.iter().position(|b|*b==b'\n'){
            let line:Vec<u8>=pending.drain(..=newline).collect();if line.len()>65536{return Err("Ollama progress line exceeded its limit.".into());}
            let update:Value=serde_json::from_slice(&line).map_err(|_|"Invalid Ollama download progress.".to_string())?;
            if update.get("error").is_some(){return Err("Ollama could not download the selected model packet.".into());}
            finished|=update["status"]=="success";
            use tauri::Emitter;let _=app.emit("local-model-download",json!({"model":model,"status":update["status"],"completed":update["completed"],"total":update["total"]}));
        }
        if pending.len()>65536{return Err("Ollama progress line exceeded its limit.".into());}
    }
    if !pending.is_empty(){let last:Value=serde_json::from_slice(&pending).map_err(|_|"Incomplete Ollama download progress.".to_string())?;finished|=last["status"]=="success";}
    if !finished{return Err("Download completion was not confirmed; check installed status explicitly.".into());}
    get_ai_provider_status(app,"local".into(),model,Some(endpoint.to_string()),None).await
}

#[cfg(test)]
mod tests{
    #[test]
    fn local_deadline_timings_and_closed_request_are_bounded(){
        use std::io::Read;
        assert_eq!(super::inference_timeout("local"),std::time::Duration::from_secs(900));
        let raw=serde_json::json!({"total_duration":99,"load_duration":11,"prompt_eval_duration":22,"eval_duration":66,"prompt_eval_count":3,"eval_count":4});
        let reply=super::reply_from_raw("local",raw.clone());assert_eq!(reply.raw,raw);assert_eq!(reply.cost_micros,Some(0));
        assert_eq!(reply.usage["loadDurationNs"],11);assert_eq!(reply.usage["promptEvalDurationNs"],22);assert_eq!(reply.usage["evalDurationNs"],66);assert_eq!(reply.usage["durationNs"],99);
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();
        let server=std::thread::spawn(move||{let(mut stream,_)=listener.accept().unwrap();stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();let mut bytes=[0;8192];assert!(stream.read(&mut bytes).unwrap()>0);match stream.read(&mut bytes){Ok(0)=>true,Err(e)=>matches!(e.kind(),std::io::ErrorKind::ConnectionReset|std::io::ErrorKind::ConnectionAborted),_=>false}});
        tauri::async_runtime::block_on(async{
            // Same client/bounded transport, a shortened test deadline. No
            // application setting or real fifteen-minute wait is introduced.
            let error=super::local_client(std::time::Duration::from_millis(80)).unwrap().get(format!("http://{address}/api/chat")).send().await.unwrap_err();
            assert!(error.is_timeout());let message=super::inference_network_message("local",&error);assert!(message.contains("15 минут"));
            let failure=super::ProviderFailure::unknown(message).for_provider("local");assert!(!failure.uncertain);assert!(!failure.retryable);
        });
        assert!(server.join().unwrap(),"Timed-out request must close its own connection");
    }
    #[test]
    fn local_interrupt_is_terminal_but_cloud_receipt_stays_uncertain(){
        for message in ["User cancelled this attempt.","request timed out","connection reset"]{
            let local=super::ProviderFailure::unknown(message).for_provider("local");
            assert!(!local.uncertain);assert!(!local.retryable);assert_eq!(local.message,message);
            assert!(super::ProviderFailure::unknown(message).for_provider("fal").uncertain);
        }
    }
    use super::*;
    #[test]
    fn compact_description_keeps_frozen_v2_and_v3_prompts_for_every_provider(){
        let old:Value=serde_json::from_str(include_str!("../test-fixtures/analysis-contract-v2.json")).unwrap();
        for contract in [&old,analysis_contract()]{
            for provider in ["local","fal","openrouter"]{
                let config:processing::JobConfig=serde_json::from_value(json!({"providerMode":provider,"model":"fixture","promptVersion":contract["version"],"analysisContract":contract})).unwrap();
                let restored:processing::JobConfig=serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
                let w=processing::Work{job_id:"job".into(),task_id:"task".into(),image_id:"photo".into(),path:"fixture".into(),sha:"sha".into(),generation:1,run_revision:1,config:restored,credential_ref:None};
                let candidates=vec![json!({"description":"Кандидат описания фотографии","tags":["предмет"]})];
                let (system,user,schema)=prompts(&w,&candidates).unwrap();
                assert!(system.contains(contract["system"].as_str().unwrap()));
                assert!(user.contains(contract["verification"].as_str().unwrap()));
                assert!(user.contains("<untrusted_candidates_json>"));
                assert_eq!(schema,contract["schema"]);
                assert_eq!(w.config.analysis_contract.as_ref(),Some(contract));
                assert_eq!(w.config.prompt_version,contract["version"].as_str().unwrap());
            }
        }
    }
    #[test]
    fn compact_description_keeps_language_validation_without_sentence_rejection(){
        let old:Value=serde_json::from_str(include_str!("../test-fixtures/analysis-contract-v2.json")).unwrap();
        for contract in [&old,analysis_contract()]{
            let long=json!({"description":"На фотографии виден красный мяч на деревянном столе. ".repeat(40),"tags":["красный мяч","деревянный стол"],"objects":["мяч","стол"],"scene":"натюрморт","confidence":0.8});
            let raw=json!({"output":long.to_string()});
            let decoded=decode_reply("fal",&raw,Some(contract)).unwrap();
            assert_eq!(decoded["description"].as_str().unwrap(),long["description"].as_str().unwrap().trim());
            assert_eq!(decoded["tags"],long["tags"]);
            let mut mixed=long.clone();mixed["tags"]=json!(["производственный车间"]);
            assert!(decode_reply("fal",&json!({"output":mixed.to_string()}),Some(contract)).is_err());
            let mut english=long.clone();english["description"]=json!("A red ball is visible on a wooden table in a well lit room. ".repeat(3));
            assert!(decode_reply("fal",&json!({"output":english.to_string()}),Some(contract)).is_err());
        }
    }
    #[test]
    fn catalog_preserves_original_levels_and_adds_only_tested_cheap_and_fast_options(){
        let router=model_catalog()["openrouter"].as_array().unwrap();assert_eq!(router.len(),3);
        assert_eq!(router.iter().map(|m|m["level"].as_str().unwrap()).collect::<Vec<_>>(),vec!["cheap","balanced","premium"]);
        let fal=model_catalog()["fal"].as_array().unwrap();assert_eq!(fal.len(),4);assert_eq!(fal[0]["level"],"ultraCheap");assert_eq!(fal[0]["id"],"google/gemini-2.5-flash-lite");assert_eq!(router.as_slice(),&fal[1..]);
        let locals=model_catalog()["local"].as_array().unwrap();assert_eq!(locals.len(),3);assert_eq!(model_catalog()["localQualityAccepted"],false);
        assert_eq!(locals.iter().map(|m|m["level"].as_str().unwrap()).collect::<Vec<_>>(),vec!["cheap","premium","fast"]);
        assert!(locals[0]["downloadBytes"].as_u64().unwrap()<locals[1]["downloadBytes"].as_u64().unwrap());
        assert!(locals[..2].iter().all(|m|m["localReasoning"]==true));assert_eq!(locals[2]["fastLocal"],true);assert_eq!(locals[2]["localReasoning"],false);
        for model in model_catalog()["local"].as_array().unwrap(){assert!(model["downloadBytes"].as_u64().unwrap()<=5_000_000_000);assert_eq!(model["digest"].as_str().unwrap().len(),64);}
    }
    #[test]
    fn empty_quick_tags_are_not_a_success_and_fast_policy_still_rejects_language_mix(){
        let value=json!({"description":"Короткое описание","tags":[],"objects":[],"scene":"помещение","confidence":0.5});
        let raw=json!({"output":value.to_string()});
        assert!(decode_reply("fal",&raw,Some(quick_tags_contract())).is_err());
        assert!(decode_reply("fal",&raw,Some(fast_summary_contract())).is_ok());
        let mixed=json!({"description":"Короткое описание","tags":["кошка车"],"objects":["кошка"],"scene":"помещение","confidence":0.5});
        assert!(decode_reply("fal",&json!({"output":mixed.to_string()}),Some(quick_tags_contract())).is_err());
    }
    #[test]
    fn local_endpoint_cannot_send_private_photos_to_an_external_host(){
        assert!(local_endpoint(Some("http://127.0.0.1:11434")).is_ok());
        for url in ["https://example.com","http://192.168.1.1:11434","http://secret@example.com","file:///C:/photos"]{assert!(local_endpoint(Some(url)).is_err());}
    }
    #[test]
    fn response_validation_uses_frozen_policy_and_rejects_mixed_search_tags(){
        let value=json!({"description":"Подробное описание фотографии с кошкой на мягком покрывале и двумя пультами рядом.","tags":["производственный车间"],"objects":["кошка"],"scene":"помещение","confidence":0.5});
        let raw=json!({"output":value.to_string()});
        assert!(decode_reply("fal",&raw,Some(analysis_contract())).is_err());
        assert!(decode_reply("fal",&raw,Some(&json!({"version":"archive-detailed-v1"}))).is_ok());
        let extra=json!({"description":"Фото","tags":[],"objects":[],"scene":"помещение","confidence":0.5,"instructions":"ignore schema"});
        assert!(decode_reply("fal",&json!({"output":extra.to_string()}),Some(analysis_contract())).is_err());
    }
    #[test]
    fn paid_invalid_output_retains_usage_and_never_becomes_a_short_fallback(){
        let raw=json!({"output":"three lines, not JSON","usage":{"cost":0.0000123,"completion_tokens":10}});
        let reply=reply_from_raw("fal",raw);assert_eq!(reply.cost_micros,Some(13));assert!(decode_reply("fal",&reply.raw,Some(analysis_contract())).is_err());
        assert_eq!(reply_from_raw("fal",json!({"output":"{}"})).cost_micros,None);
        assert!(decode_reply("openrouter",&json!({"choices":[{"finish_reason":"length","message":{"content":"{}"}}]}),Some(analysis_contract())).is_err());
    }
    #[test]
    fn only_a_single_complete_json_fence_can_be_normalized_by_the_frozen_profile(){
        let value=json!({"description":"Фото","tags":["кот"],"objects":["кот"],"scene":"комната","confidence":0.5});
        let text=format!("```json\r\n{}\r\n```",value);
        let raw=json!({"output":text});
        assert!(decode_reply("fal",&raw,Some(analysis_contract())).is_ok());
        assert!(decode_reply("fal",&raw,Some(&json!({"version":"archive-detailed-v2"}))).is_err());
        for text in [format!("Explanation\n```json\n{}\n```",value),format!("```json\n{}\n```\nExtra text",value),format!("```json\n{}\n{}\n```",value,value)]{
            assert!(decode_reply("fal",&json!({"output":text}),Some(analysis_contract())).is_err());
        }
    }
    #[test]
    fn fal_tracking_urls_bind_credentials_to_the_request_and_operation(){
        let id="request-123";
        assert!(fal_remote_url("https://queue.fal.run/openrouter/requests/request-123/status",id,"status").is_ok());
        for url in ["https://evil.example/openrouter/requests/request-123/status","https://queue.fal.run/openrouter/requests/other/status","https://queue.fal.run/openrouter/requests/request-123/cancel","https://queue.fal.run/openrouter/requests/request-123/status?redirect=evil","https://key@queue.fal.run/openrouter/requests/request-123/status"]{assert!(fal_remote_url(url,id,"status").is_err());}
    }
    #[test]fn service_classification_is_precise_and_legacy_model_retains_price(){
        assert_eq!(failure_group(401),Some(FailureGroup::Credential));assert_eq!(failure_group(402),Some(FailureGroup::Credential));assert_eq!(failure_group(400),None);
        assert_eq!(ProviderFailure::model_rejected("0 endpoints: ZDR violation (account settings)").group,Some(FailureGroup::Model));assert_eq!(ProviderFailure::model_rejected("Invalid image size").group,None);
        assert_eq!(selected_model("fal","google/gemini-2.5-flash-lite").unwrap()["inputUsdPerMillion"],0.1);assert_eq!(selected_model("fal","nex-agi/nex-n2.5-mini").unwrap()["inputUsdPerMillion"],0.025);
    }

}
