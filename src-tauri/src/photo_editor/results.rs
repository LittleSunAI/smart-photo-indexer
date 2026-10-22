//! Provider files use the existing file journal before optional pixel finishing.
use crate::*;
use super::{contracts::Draft, finish, geometry, local, store};
use serde_json::{json, Value};

fn operation_id(job: &str, index: usize) -> String { format!("provider-output-{job}-{index}") }

pub(super) fn descriptors(output: &Value) -> Vec<Value> {
    output["images"].as_array().cloned().unwrap_or_else(|| {
        if output["image"].is_object() { vec![output["image"].clone()] } else { Vec::new() }
    })
}

pub(super) fn wanted(snapshot: &Value, output: &Value) -> usize {
    descriptors(output).len().min(snapshot["plan"]["outputCount"].as_u64().unwrap_or(1).clamp(1,4) as usize)
}

pub(super) fn persist(app: &tauri::AppHandle, job: &str, index: usize, descriptor: &Value, bytes: &[u8]) -> Result<(),String> {
    let sha = format!("{:x}",Sha256::digest(bytes));
    let (extension, _) = local::raw_format(bytes)?;
    let relative = format!("assets/{sha}.{extension}");
    let id = operation_id(job,index);
    let db = store::connection(app)?;
    let metadata = json!({"version":1,"kind":"providerOutput","jobId":job,"outputIndex":index,"descriptor":descriptor,"sha256":sha});
    db.execute("INSERT OR IGNORE INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES(?1,?2,'planned',?3,?4)",params![id,relative,metadata.to_string(),unix_now()]).map_err(|e|e.to_string())?;
    let saved:String=db.query_row("SELECT metadata FROM editor_file_operations WHERE id=?1",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let saved:Value=serde_json::from_str(&saved).map_err(|e|e.to_string())?;
    if saved["sha256"] != sha { return Err("Повторный ответ отличается от уже сохранённого output; прежние байты сохранены.".into()); }
    let path=store::owned_path(app,&relative)?;
    if !path.exists() { file_safety::atomic_save(&path,|out,_|std::io::Write::write_all(out,bytes).map_err(|e|e.to_string()))?; }
    if compute_file_sha256(&path)? != sha { return Err("Сохранённый output повреждён; замена запрещена.".into()); }
    db.execute("UPDATE editor_file_operations SET state=CASE WHEN state='committed' THEN state ELSE 'fileSaved' END,error=NULL WHERE id=?1",[id]).map_err(|e|e.to_string())?;
    Ok(())
}

pub(super) fn process(app:&tauri::AppHandle,job:&str,index:usize,snapshot:&Value)->Result<Value,String>{
    let db=store::connection(app)?;
    let (relative,raw):(String,String)=db.query_row("SELECT relative_path,metadata FROM editor_file_operations WHERE id=?1",[operation_id(job,index)],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    let metadata:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
    if metadata["jobId"]!=job || metadata["outputIndex"].as_u64()!=Some(index as u64){return Err("Output принадлежит другой задаче.".into());}
    let path=store::owned_path(app,&relative)?;
    if compute_file_sha256(&path)?!=metadata["sha256"].as_str().unwrap_or(""){return Err("Сохранённый output не прошёл SHA-проверку.".into());}
    if let Some(result)=metadata.get("readyResult") {
        let asset=store::asset_value(&db,result["id"].as_str().ok_or("Нет result ID")?)?;
        let path=store::owned_path(app,asset["relativePath"].as_str().ok_or("Нет owned path")?)?;
        if compute_file_sha256(&path)?!=asset["sha256"].as_str().unwrap_or(""){return Err("Результат повреждён.".into());}
        return Ok(result.clone());
    }
    drop(db);
    let image=file_safety::decode_bounded(&path)?;
    let raw_asset=local::save_raw_image(app,&image,fs::read(&path).map_err(|e|e.to_string())?)?;
    let operation=snapshot["draft"]["operation"].as_str().unwrap_or("");
    let source=if ["outpaint","restore"].contains(&operation){
        (||->Result<image::DynamicImage,String>{
            let draft:Draft=serde_json::from_value(snapshot["draft"].clone()).map_err(|e|e.to_string())?;
            let db=store::connection(app)?;
            let project:String=db.query_row("SELECT project_id FROM editor_jobs WHERE id=?1",[job],|r|r.get(0)).map_err(|e|e.to_string())?;
            let asset=local::project_asset(app,&project,&draft.asset_id)?;
            let path=store::owned_path(app,asset["relativePath"].as_str().ok_or("Нет исходника")?)?;
            if compute_file_sha256(&path)?!=asset["sha256"].as_str().unwrap_or(""){return Err("Исходник изменился".into());}
            Ok(geometry::orient_image(file_safety::decode_bounded(&path)?,draft.working_turn,draft.mirrored))
        })().ok()
    }else{None};
    let finished=finish::output_with_target(image,source,operation,&snapshot["plan"]["expansion"],snapshot["draft"]["formValues"]["__preserveColor"]!=false,&snapshot["plan"]["outputTarget"]);
    let correction=finished.correction;
    let mut warnings=finished.warnings;
    let mut post_process=finished.post_process;
    let mut asset=if post_process=="none"{raw_asset.clone()}else{
        match local::save_image(app,&finished.image,"image"){
            Ok(asset)=>asset,
            Err(_)=>{warnings.push("Необязательная коррекция не сохранена. Показан исходный ответ модели.".into());post_process="none";raw_asset.clone()},
        }
    };
    asset["providerOutputSha256"]=metadata["sha256"].clone();
    asset["providerRawAssetId"]=raw_asset["id"].clone();
    asset["postProcess"]=json!(post_process);
    asset["outputCorrection"]=correction_record(correction,&metadata["sha256"],&asset,post_process);
    asset["warnings"]=json!(warnings);
    asset["outputIndex"]=json!(index);
    store::connection(app)?.execute("UPDATE editor_file_operations SET metadata=json_set(metadata,'$.readyResult',json(?2)),error=NULL WHERE id=?1",params![operation_id(job,index),asset.to_string()]).map_err(|e|e.to_string())?;
    Ok(asset)
}

fn correction_record(correction:Option<Value>,raw_sha:&Value,asset:&Value,post_process:&str)->Value{
    let Some(mut value)=correction else{return Value::Null};
    value["rawSha256"]=raw_sha.clone();value["actual"]=json!({"width":asset["width"],"height":asset["height"]});
    if value["applied"]==true&&post_process!="auto-crop-center-v1"{value["applied"]=json!(false);value["cropRect"]=Value::Null;value["fallback"]=json!("raw-output");}
    value
}

pub(super) fn cache_response(app:&tauri::AppHandle,job:&str,output:&Value)->Result<(),String>{
    store::connection(app)?.execute("UPDATE editor_attempts SET response_meta=json_set(COALESCE(response_meta,'{}'),'$.providerResponse',json(?2)) WHERE job_id=?1 AND status<>'notSent'",params![job,output.to_string()]).map_err(|e|e.to_string())?;
    Ok(())
}

pub(super) fn cached_response(app:&tauri::AppHandle,job:&str)->Result<Option<Value>,String>{
    let raw:Option<String>=store::connection(app)?.query_row("SELECT response_meta FROM editor_attempts WHERE job_id=?1 AND response_meta IS NOT NULL ORDER BY rowid DESC LIMIT 1",[job],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    Ok(raw.and_then(|raw|serde_json::from_str::<Value>(&raw).ok()).and_then(|value|value.get("providerResponse").cloned()))
}

pub(super) fn saved(app:&tauri::AppHandle,job:&str,snapshot:&Value,output:&Value)->Result<Option<Vec<Value>>,String>{
    let db=store::connection(app)?;
    let count=wanted(snapshot,output);
    let Some(indices)=saved_indices(&db,job,count)? else{return Ok(None)};
    drop(db);
    let mut results=Vec::new();let mut errors=Vec::new();
    for index in indices{match process(app,job,index,snapshot){Ok(asset)=>results.push(asset),Err(error)=>errors.push(error)}}
    if results.is_empty(){return Err(errors.join("; "));}
    partial_warning(&mut results,count);
    Ok(Some(results))
}

// outputsFinished records an attempted batch, not successful file persistence.
// Old interrupted jobs can have this flag with no journal entries at all.
fn saved_indices(db:&Connection,job:&str,count:usize)->Result<Option<Vec<usize>>,String>{
    if count==0{return Ok(None);}
    let finished:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_attempts WHERE job_id=?1 AND json_extract(response_meta,'$.outputsFinished')=1)",[job],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut indices=Vec::new();
    for index in 0..count{
        let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_file_operations WHERE id=?1 AND state IN ('fileSaved','committed'))",[operation_id(job,index)],|r|r.get(0)).map_err(|e|e.to_string())?;
        if exists{indices.push(index);}
    }
    if indices.is_empty()||(!finished&&indices.len()!=count){return Ok(None);}
    Ok(Some(indices))
}

pub(super) fn partial_warning(results:&mut [Value],expected:usize){
    let received=results.len();
    if received<expected{
        if let Some(first)=results.first_mut(){
            let mut warnings=first["warnings"].as_array().cloned().unwrap_or_default();
            warnings.push(json!(format!("Сохранено {received} из {expected} результатов. Остальные выходы не удалось загрузить или декодировать; пригодные кадры доступны.")));
            first["warnings"]=json!(warnings);
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn recovery_db(finished:bool)->Connection{
        let db=Connection::open_in_memory().unwrap();db.execute_batch(store::SCHEMA).unwrap();
        db.execute_batch("INSERT INTO editor_assets(id,kind,relative_path,preview_path,sha256,width,height,bytes,content_type) VALUES('source','image','assets/source.png','previews/source.png','sha',1,1,1,'image/png');
        INSERT INTO editor_projects(id,title,source_name,source_path,source_sha256,draft,created_at_unix,updated_at_unix) VALUES('p','test','test.png','','sha','{}',1,1);
        INSERT INTO editor_jobs(id,project_id,state,operation,model_id,source_asset_id,snapshot,credential_ref,created_at_unix,updated_at_unix) VALUES('j','p','interrupted','universal-edit','test','source','{}','test',1,1);").unwrap();
        db.execute("INSERT INTO editor_attempts(id,job_id,status,reserved_micros,created_at_unix,response_meta) VALUES('a','j','remotePending',20000,1,?1)",[json!({"outputsFinished":finished}).to_string()]).unwrap();db
    }
    #[test]fn failed_download_with_finished_flag_requests_network_instead_of_missing_row(){
        let db=recovery_db(true);assert_eq!(saved_indices(&db,"j",1).unwrap(),None);
        db.execute("INSERT INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES('provider-output-j-0','assets/a.png','planned','{}',1)",[]).unwrap();
        assert_eq!(saved_indices(&db,"j",1).unwrap(),None);
    }
    #[test]fn partial_offline_recovery_only_reads_real_saved_indices(){
        let db=recovery_db(true);db.execute("INSERT INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES('provider-output-j-1','assets/a.png','fileSaved','{}',1)",[]).unwrap();
        assert_eq!(saved_indices(&db,"j",2).unwrap(),Some(vec![1]));
        db.execute("UPDATE editor_attempts SET response_meta='{}'",[]).unwrap();assert_eq!(saved_indices(&db,"j",2).unwrap(),None);
        db.execute("INSERT INTO editor_file_operations(id,relative_path,state,metadata,created_at_unix) VALUES('provider-output-j-0','assets/b.png','committed','{}',1)",[]).unwrap();assert_eq!(saved_indices(&db,"j",2).unwrap(),Some(vec![0,1]));
    }
    #[test]fn correction_fallback_is_portable_and_keeps_requested_received_and_raw_sha(){
        let correction=json!({"version":1,"applied":true,"requested":{"width":600,"height":600},"received":{"width":800,"height":500},"cropRect":{"x":100,"y":0,"width":600,"height":500}});
        let raw=json!({"width":800,"height":500});let fallback=correction_record(Some(correction.clone()),&json!("raw-sha"),&raw,"none");assert_eq!(fallback["applied"],false);assert!(fallback["cropRect"].is_null());assert_eq!(fallback["actual"],raw);assert_eq!(fallback["rawSha256"],"raw-sha");assert_eq!(fallback["requested"],correction["requested"]);assert!(fallback.get("assetId").is_none());
        let saved=correction_record(Some(correction),&json!("raw-sha"),&json!({"width":600,"height":500}),"auto-crop-center-v1");assert_eq!(saved["applied"],true);assert_eq!(saved["cropRect"]["x"],100);assert!(correction_record(None,&json!("sha"),&raw,"none").is_null());
    }
    #[test]
    fn requested_outputs_are_bounded_but_descriptors_keep_their_indices(){
        let output=json!({"images":[{"url":"bad"},{"url":"good"},{"url":"extra"}]});
        assert_eq!(wanted(&json!({"plan":{"outputCount":2}}),&output),2);
        assert_eq!(descriptors(&output)[1]["url"],"good");
        assert_eq!(wanted(&json!({"plan":{"outputCount":100}}),&output),3);
        assert_eq!(descriptors(&json!({"image":{"url":"one"}})).len(),1);
        assert!(descriptors(&json!({"error":"no output"})).is_empty());
    }
    #[test]
    fn partial_result_keeps_the_original_output_index_and_warning(){
        let mut results=vec![json!({"outputIndex":1,"warnings":["size mismatch"]})];
        partial_warning(&mut results,2);
        assert_eq!(results[0]["outputIndex"],1);
        assert_eq!(results[0]["warnings"].as_array().unwrap().len(),2);
    }
}
