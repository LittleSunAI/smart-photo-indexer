//! Imported task history is inert: it never enters the worker or monetary ledger.
use super::{contracts::Draft, store};
use crate::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS editor_imported_jobs(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES editor_projects(id),record TEXT NOT NULL);";

fn validate_metadata(record:&Value)->Result<(),String>{
 if record["version"]!=2||record["availability"]!="metadata-only"||record.as_object().is_none_or(|o|o.keys().any(|k|!["version","availability","job","attempts"].contains(&k.as_str()))){return Err("Неверный формат квитанции без файлов.".into());}
 let j=&record["job"];if j["readOnly"]!=true||j["metadataOnly"]!=true||!j["sourceAssetId"].is_null()||j["results"]!=json!([])||j["resultAssetIds"]!=json!([]){return Err("Квитанция без файлов не может открывать кадры.".into());}
 if j.as_object().is_none_or(|o|o.keys().any(|k|!["id","projectId","modelId","operation","state","requestId","reservedMicros","spentMicros","error","createdAtUnix","updatedAtUnix","startedAtUnix","completedAtUnix","sourceSize","requestedSize","requestedResolution","requestedQuality","requestedOutputCount","providerOutputCount","resultSizes","readOnly","metadataOnly","sourceAssetId","results","resultAssetIds"].contains(&k.as_str()))){return Err("Неизвестные поля квитанции без файлов.".into());}
 for k in ["id","projectId","modelId","operation","state"]{if j[k].as_str().is_none_or(|s|s.is_empty()||s.len()>500){return Err("Повреждены сведения квитанции.".into());}}
 for k in ["createdAtUnix","updatedAtUnix","reservedMicros"]{if j[k].as_u64().is_none(){return Err("Повреждена статистика квитанции.".into());}}
 for k in ["spentMicros","startedAtUnix","completedAtUnix","requestedOutputCount","providerOutputCount"]{if j.get(k).is_some_and(|v|!v.is_null()&&v.as_u64().is_none()){return Err("Неверный расход или время квитанции.".into());}}
 let attempts=record["attempts"].as_array().filter(|a|a.len()<=10000).ok_or("Нет попыток квитанции")?;
 for a in attempts{if a.as_object().is_none_or(|o|o.keys().any(|k|!["id","status","reservedMicros","costMicros","requestId","createdAtUnix"].contains(&k.as_str())))||a["id"].as_str().is_none_or(|s|s.is_empty()||s.len()>500)||a["status"].as_str().is_none()||a["reservedMicros"].as_u64().is_none()||a["createdAtUnix"].as_u64().is_none(){return Err("Повреждена попытка квитанции.".into());}}
 Ok(())
}

#[cfg(test)]mod metadata_tests{use super::*;
 #[test]fn metadata_only_receipt_remaps_without_fake_assets_or_replay(){let record=json!({"version":2,"availability":"metadata-only","job":{"id":"j","projectId":"old","modelId":"m","operation":"replace-object","state":"completed","createdAtUnix":1,"updatedAtUnix":2,"reservedMicros":100,"spentMicros":50,"requestId":"receipt","sourceAssetId":null,"readOnly":true,"metadataOnly":true,"results":[],"resultAssetIds":[]},"attempts":[{"id":"a","status":"completed","createdAtUnix":1,"reservedMicros":100,"costMicros":50,"requestId":"receipt"}]});let empty=HashMap::new();let(rows,ids)=remap(&[record.clone()],"old","new",&empty,&empty,&HashMap::new(),&|_|panic!("Metadata receipt must not translate a draft")).unwrap();assert_eq!(rows[0]["job"]["projectId"],"new");assert_eq!(rows[0]["job"]["id"],ids["j"]);assert_eq!(rows[0]["job"]["requestId"],"receipt");assert!(rows[0]["job"]["sourceAssetId"].is_null());let mut bad=record;bad["job"]["sourceAssetId"]=json!("made-up-asset");assert!(validate_metadata(&bad).is_err());}
}

fn scrub_consent(draft: &mut Value) {
    if let Some(form) = draft["formValues"].as_object_mut() {
        form.remove("__consentScope");
        if let Some(contexts) = form.get_mut("__contexts").and_then(Value::as_object_mut) {
            for context in contexts.values_mut() {
                if let Some(form) = context["formValues"].as_object_mut() {
                    form.remove("__consentScope");
                }
            }
        }
    }
}

pub fn export(app: &tauri::AppHandle, project: &str) -> Result<Vec<Value>, String> {
    let jobs = super::jobs::list(app, Some(project))?;
    let db = store::connection(app)?;
    let mut records = Vec::new();
    for job in jobs.as_array().ok_or("Нет списка задач")? {
        let id = job["id"].as_str().ok_or("Нет ID задачи")?;
        if job["readOnly"] == true {
            let raw: String = db
                .query_row(
                    "SELECT record FROM editor_imported_jobs WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let mut record: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            record["job"] = job.clone();
            records.push(record);
            continue;
        }
        if job["metadataOnly"]==true{
            let mut summary=serde_json::Map::new();for key in ["id","projectId","modelId","operation","state","requestId","reservedMicros","spentMicros","error","createdAtUnix","updatedAtUnix","startedAtUnix","completedAtUnix","sourceSize","requestedSize","requestedResolution","requestedQuality","requestedOutputCount","providerOutputCount","resultSizes"]{if let Some(v)=job.get(key){summary.insert(key.into(),v.clone());}}
            summary.insert("readOnly".into(),json!(true));summary.insert("metadataOnly".into(),json!(true));summary.insert("sourceAssetId".into(),Value::Null);summary.insert("results".into(),json!([]));summary.insert("resultAssetIds".into(),json!([]));
            let attempts={let mut q=db.prepare("SELECT id,status,reserved_micros,cost_micros,request_id,created_at_unix FROM editor_attempts WHERE job_id=?1 ORDER BY created_at_unix,id").map_err(|e|e.to_string())?;let rows=q.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"reservedMicros":r.get::<_,u64>(2)?,"costMicros":r.get::<_,Option<u64>>(3)?,"requestId":r.get::<_,Option<String>>(4)?,"createdAtUnix":r.get::<_,u64>(5)?}))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
            records.push(json!({"version":2,"availability":"metadata-only","job":summary,"attempts":attempts}));continue;
        }
        let raw: String = db
            .query_row("SELECT snapshot FROM editor_jobs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let frozen: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let mut draft = frozen["draft"].clone();
        scrub_consent(&mut draft);
        let snapshot = json!({"version":1,"draft":draft,"parentId":frozen["parentId"],"plan":frozen["plan"],"consent":false});
        let activation: Option<Value> = db.query_row("SELECT source_step_id,state,activated_step_id,updated_at_unix FROM editor_job_activation WHERE job_id=?1",[id],|r|Ok(json!({"sourceStepId":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?,"activatedStepId":r.get::<_,Option<String>>(2)?,"updatedAtUnix":r.get::<_,u64>(3)?}))).optional().map_err(|e|e.to_string())?;
        let outputs = {
            let mut q = db.prepare("SELECT output_index,asset_id,step_id,accepted,provider_descriptor FROM editor_job_outputs WHERE job_id=?1 ORDER BY output_index").map_err(|e|e.to_string())?;
            let rows=q.query_map([id],|r| {let raw:String=r.get(4)?;let original:Value=serde_json::from_str(&raw).unwrap_or(Value::Null);let mut descriptor=serde_json::Map::new();for key in ["width","height","content_type","file_name","file_size","warnings","actualSize","postProcess"] {if let Some(v)=original.get(key){descriptor.insert(key.into(),v.clone());}}Ok(json!({"outputIndex":r.get::<_,u64>(0)?,"assetId":r.get::<_,Option<String>>(1)?,"stepId":r.get::<_,Option<String>>(2)?,"accepted":r.get::<_,bool>(3)?,"providerDescriptor":descriptor}))}).map_err(|e|e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };
        let attempts = {
            let mut q=db.prepare("SELECT id,status,reserved_micros,cost_micros,request_id,created_at_unix FROM editor_attempts WHERE job_id=?1 ORDER BY created_at_unix,rowid").map_err(|e|e.to_string())?;
            let rows=q.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"reservedMicros":r.get::<_,u64>(2)?,"costMicros":r.get::<_,Option<u64>>(3)?,"requestId":r.get::<_,Option<String>>(4)?,"createdAtUnix":r.get::<_,u64>(5)?}))).map_err(|e|e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };
        records.push(json!({"version":1,"job":job,"requestSnapshot":snapshot,"activation":activation,"outputs":outputs,"attempts":attempts}));
    }
    Ok(records)
}

fn mapped(map: &HashMap<String, String>, value: &Value, what: &str) -> Result<Value, String> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    Ok(json!(map.get(value.as_str().ok_or(what)?).ok_or(what)?))
}

pub fn remap(
    records: &[Value],
    old_project: &str,
    project: &str,
    assets: &HashMap<String, String>,
    steps: &HashMap<String, String>,
    step_jobs: &HashMap<String, Option<String>>,
    translate: &dyn Fn(&mut Draft) -> Result<(), String>,
) -> Result<(Vec<Value>, HashMap<String, String>), String> {
    let mut jobs = HashMap::new();
    for record in records {
        if record.as_object().is_none_or(|map| {
            map.keys().any(|key| {
                ![
                    "version",
                    "job",
                    "requestSnapshot",
                    "activation",
                    "outputs",
                    "attempts",
                    "availability",
                ]
                .contains(&key.as_str())
            })
        }) {
            return Err("Неизвестные поля истории задач пакета.".into());
        }
        let job = &record["job"];
        let id = job["id"]
            .as_str()
            .filter(|id| !id.is_empty() && id.len() <= 500)
            .ok_or("Неверный ID задачи пакета")?;
        if (record["version"] != 1&&record["version"]!=2)
            || job["projectId"] != old_project
            || jobs
                .insert(
                    id.into(),
                    stable_text_id("editor-history", &format!("{project}|{id}")),
                )
                .is_some()
        {
            return Err("История задач содержит дубли или чужой проект.".into());
        }
    }
    let mut result = Vec::new();
    for original in records {
        let mut record = original.clone();
        if record["version"]==2{validate_metadata(&record)?;let id=record["job"]["id"].as_str().unwrap().to_string();record["job"]["id"]=json!(jobs[&id]);record["job"]["projectId"]=json!(project);for attempt in record["attempts"].as_array_mut().unwrap(){attempt["id"]=json!(stable_text_id("editor-history-attempt",&format!("{project}|{}",attempt["id"].as_str().unwrap())));}result.push(record);continue;}
        let job = record["job"]
            .as_object_mut()
            .ok_or("Неверная карточка задачи")?;
        for name in ["modelId", "operation", "state", "sourceAssetId"] {
            if job.get(name).and_then(Value::as_str).is_none() {
                return Err("Повреждены сведения о модели или состоянии задачи.".into());
            }
        }
        for name in ["results", "resultSizes"] {
            if job.get(name).is_some_and(|v| !v.is_null() && !v.is_array()) {
                return Err("Повреждён список результатов задачи.".into());
            }
        }
        for name in ["createdAtUnix", "updatedAtUnix", "reservedMicros"] {
            if job.get(name).and_then(Value::as_u64).is_none() {
                return Err("Повреждены время или расходы задачи.".into());
            }
        }
        for name in [
            "spentMicros",
            "startedAtUnix",
            "completedAtUnix",
            "requestedOutputCount",
            "providerOutputCount",
        ] {
            if job
                .get(name)
                .is_some_and(|v| !v.is_null() && v.as_u64().is_none())
            {
                return Err("Повреждена статистика задачи.".into());
            }
        }
        for name in [
            "error",
            "requestId",
            "requestedResolution",
            "requestedQuality",
        ] {
            if job
                .get(name)
                .is_some_and(|v| !v.is_null() && v.as_str().is_none())
            {
                return Err("Повреждены текстовые сведения задачи.".into());
            }
        }
        job.insert("id".into(), mapped(&jobs, &job["id"], "Нет ID задачи")?);
        job.insert("projectId".into(), json!(project));
        job.insert(
            "sourceAssetId".into(),
            mapped(assets, &job["sourceAssetId"], "Нет исходника задачи")?,
        );
        job.insert("readOnly".into(), json!(true));
        job.remove("admission");
        let ids = job
            .get("resultAssetIds")
            .and_then(Value::as_array)
            .ok_or("Нет результатов задачи")?
            .iter()
            .map(|id| mapped(assets, id, "Нет изображения результата задачи"))
            .collect::<Result<Vec<_>, _>>()?;
        job.insert("resultAssetIds".into(), json!(ids));
        if let Some(results) = job.get_mut("results").and_then(Value::as_array_mut) {
            for output in results {
                let old = output["stepId"].as_str().ok_or("Нет ID результата")?;
                if step_jobs.get(old).and_then(|id| id.as_deref()) != original["job"]["id"].as_str()
                {
                    return Err("Результат задачи связан с чужим кадром.".into());
                }
                output["stepId"] = mapped(steps, &output["stepId"], "Нет кадра результата")?;
                output["assetId"] =
                    mapped(assets, &output["assetId"], "Нет изображения результата")?;
            }
        }
        let snapshot = &mut record["requestSnapshot"];
        if !snapshot.is_null() {
            snapshot["parentId"] =
                mapped(steps, &snapshot["parentId"], "Нет исходного шага задачи")?;
            snapshot["consent"] = json!(false);
            if !snapshot["draft"].is_null() {
                let mut d: Draft = serde_json::from_value(snapshot["draft"].clone())
                    .map_err(|_| "Черновик задачи пакета повреждён")?;
                translate(&mut d)?;
                snapshot["draft"] = serde_json::to_value(d).map_err(|e| e.to_string())?;
            }
            if !snapshot["plan"]["derivedMask"].is_null(){
                let mask=&mut snapshot["plan"]["derivedMask"];
                mask["sourceAssetId"]=mapped(assets,&mask["sourceAssetId"],"Нет исходника производной маски")?;
            }
            // Imported history is inert. Keep its original plan identity as provenance;
            // remapped identifiers must never be mistaken for a new admitted plan.
            if let Some(hash)=snapshot["plan"].as_object_mut().and_then(|p|p.remove("planHash")){
                snapshot["originalPlanHash"]=hash;
            }
            for field in ["inputs", "masks"] {
                if let Some(items) = snapshot["plan"][field].as_array_mut() {
                    for item in items {
                        for key in ["assetId", "maskAssetId"] {
                            if item.get(key).is_some() {
                                item[key] =
                                    mapped(assets, &item[key], "Нет входного изображения задачи")?;
                            }
                        }
                    }
                }
            }
        }
        if !record["activation"].is_null() {
            let a = &mut record["activation"];
            a["sourceStepId"] = mapped(steps, &a["sourceStepId"], "Нет кадра активации")?;
            a["activatedStepId"] = mapped(
                steps,
                &a["activatedStepId"],
                "Нет активированного результата",
            )?;
            if a["state"] != "ineligible" {
                a["originalState"] = a["state"].clone();
            }
            a["state"] = json!("ineligible");
        }
        let mut ordinals = HashSet::new();
        if let Some(outputs) = record["outputs"].as_array_mut() {
            for output in outputs {
                let ordinal = output["outputIndex"]
                    .as_u64()
                    .ok_or("Нет номера результата")?;
                if !ordinals.insert(ordinal) {
                    return Err("Номер результата задачи повторяется.".into());
                }
                if output["accepted"] == true {
                    let id = output["stepId"].as_str().ok_or("Нет принятого кадра")?;
                    if step_jobs.get(id).and_then(|id| id.as_deref())
                        != original["job"]["id"].as_str()
                    {
                        return Err("Принятый результат относится к другой задаче.".into());
                    }
                }
                output["assetId"] = mapped(assets, &output["assetId"], "Нет файла ответа")?;
                output["stepId"] = mapped(steps, &output["stepId"], "Нет шага ответа")?;
            }
        }
        if let Some(attempts) = record["attempts"].as_array_mut() {
            for attempt in attempts {
                let id = attempt["id"].as_str().ok_or("Нет ID попытки")?;
                attempt["id"] = json!(stable_text_id(
                    "editor-history-attempt",
                    &format!("{project}|{id}")
                ));
            }
        }
        result.push(record);
    }
    Ok((result, jobs))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"version":1,"job":{"id":"j","projectId":"old","state":"completed","operation":"replace-object","modelId":"model","sourceAssetId":"a","createdAtUnix":1,"updatedAtUnix":3,"reservedMicros":50,"spentMicros":20,"resultAssetIds":["b","b"],"providerOutputCount":3,"results":[{"stepId":"one","assetId":"b","outputIndex":0,"hidden":false},{"stepId":"two","assetId":"b","outputIndex":1,"hidden":false}]},"requestSnapshot":{"draft":Draft::initial("a"),"parentId":"root","consent":true,"plan":{"inputs":[{"assetId":"a"},{"assetId":"b","bindings":[{"annotationId":"p","instruction":"snow"}]}],"fields":{"prompt":"Keep the original historical prompt"}}},"activation":{"sourceStepId":"root","activatedStepId":"one","state":"consumed"},"outputs":[{"outputIndex":0,"assetId":"b","stepId":"one","accepted":true},{"outputIndex":1,"assetId":"b","stepId":"two","accepted":true},{"outputIndex":2,"assetId":null,"stepId":null,"accepted":false}],"attempts":[{"id":"attempt","costMicros":20}]})
    }
    fn run(
        records: &[Value],
        owners: &HashMap<String, Option<String>>,
    ) -> Result<(Vec<Value>, HashMap<String, String>), String> {
        let assets = HashMap::from([("a".into(), "A".into()), ("b".into(), "B".into())]);
        let steps = HashMap::from([
            ("root".into(), "ROOT".into()),
            ("one".into(), "ONE".into()),
            ("two".into(), "TWO".into()),
        ]);
        remap(records, "old", "new", &assets, &steps, owners, &|d| {
            d.asset_id = assets.get(&d.asset_id).cloned().ok_or("Missing asset")?;
            d.validate()
        })
    }
    fn owners() -> HashMap<String, Option<String>> {
        HashMap::from([
            ("root".into(), None),
            ("one".into(), Some("j".into())),
            ("two".into(), Some("j".into())),
        ])
    }
    #[test]
    fn portable_history_remaps_identity_and_preserves_identical_outputs_without_execution_rights() {
        let (record, ids) = run(&[fixture()], &owners()).unwrap();
        let r = &record[0];
        assert_ne!(ids["j"], "j");
        assert_eq!(r["job"]["readOnly"], true);
        assert_eq!(r["job"]["projectId"], "new");
        assert_eq!(r["job"]["results"][0]["stepId"], "ONE");
        assert_eq!(r["job"]["results"][1]["stepId"], "TWO");
        assert_eq!(r["job"]["resultAssetIds"], json!(["B", "B"]));
        assert_eq!(r["activation"]["state"], "ineligible");
        assert_eq!(r["activation"]["originalState"], "consumed");
        assert_eq!(r["activation"]["activatedStepId"], "ONE");
        assert_eq!(r["requestSnapshot"]["consent"], false);
        assert_eq!(r["requestSnapshot"]["plan"]["inputs"][1]["assetId"], "B");
        assert_eq!(
            r["requestSnapshot"]["plan"]["inputs"][1]["bindings"][0]["instruction"],
            "snow"
        );
        assert_eq!(
            r["requestSnapshot"]["plan"]["fields"]["prompt"],
            fixture()["requestSnapshot"]["plan"]["fields"]["prompt"]
        );
        assert_eq!(r["outputs"].as_array().unwrap().len(), 3);
        assert_eq!(r["job"]["spentMicros"], 20);
        assert_ne!(r["attempts"][0]["id"], "attempt");
    }
    #[test]
    fn foreign_and_malformed_portable_receipts_cannot_create_cross_project_links() {
        let mut r = fixture();
        r["job"]["projectId"] = json!("foreign");
        assert!(run(&[r], &owners()).is_err());
        assert!(run(&[fixture(), fixture()], &owners()).is_err());
        let mut owner = owners();
        owner.insert("two".into(), Some("other-job".into()));
        assert!(run(&[fixture()], &owner).is_err());
        let mut r = fixture();
        r["activation"]["sourceStepId"] = json!("missing");
        assert!(run(&[r], &owners()).is_err());
        let mut r = fixture();
        r["job"]["spentMicros"] = json!("twenty");
        assert!(run(&[r], &owners()).is_err());
        let mut r = fixture();
        r["credentialRef"] = json!("never-import");
        assert!(run(&[r], &owners()).is_err());
    }
}

pub fn insert(db: &Connection, project: &str, records: &[Value]) -> Result<(), String> {
    for record in records {
        db.execute(
            "INSERT INTO editor_imported_jobs(id,project_id,record) VALUES(?1,?2,?3)",
            params![
                record["job"]["id"]
                    .as_str()
                    .ok_or("Нет ID архивной задачи")?,
                project,
                record.to_string()
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn list(db: &Connection, project: Option<&str>) -> Result<Vec<Value>, String> {
    let mut q=db.prepare("SELECT record FROM editor_imported_jobs WHERE (?1 IS NULL OR project_id=?1) ORDER BY rowid DESC").map_err(|e|e.to_string())?;
    let rows = q
        .query_map([project], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for raw in rows {
        let record: Value =
            serde_json::from_str(&raw.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mut job = record["job"].clone();
        job["readOnly"] = json!(true);
        // Imported receipts keep their immutable ledger after project cleanup too.
        let owner=job["projectId"].as_str();
        let unavailable:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_tombstones WHERE project_id=?1) OR EXISTS(SELECT 1 FROM json_tree(?2) t JOIN editor_assets a ON a.id=t.value WHERE NOT EXISTS(SELECT 1 FROM editor_resource_owners own WHERE own.asset_id=a.id AND own.project_id=?1)) OR EXISTS(SELECT 1 FROM json_each(?3) result WHERE NOT EXISTS(SELECT 1 FROM editor_working_steps w WHERE w.step_id=json_extract(result.value,'$.stepId') AND w.project_id=?1))",params![owner,record.to_string(),job["results"].as_array().map(|v|json!(v)).unwrap_or(json!([])).to_string()],|r|r.get(0)).map_err(|e|e.to_string())?;
        if unavailable {job["metadataOnly"]=json!(true);job["sourceAssetId"]=Value::Null;job["results"]=json!([]);job["resultAssetIds"]=json!([]);}

        if let Some(results) = job["results"].as_array_mut() {
            for result in results {
                let hidden:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_step_visibility WHERE step_id=?1 AND hidden=1)",[result["stepId"].as_str()],|r|r.get(0)).map_err(|e|e.to_string())?;
                result["hidden"] = json!(hidden);
            }
        }
        out.push(job);
    }
    Ok(out)
}

pub fn validate(db: &Connection) -> Result<(), String> {
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='editor_imported_jobs')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !exists {
        return Ok(());
    }
    let mut q = db
        .prepare("SELECT id,project_id,record FROM editor_imported_jobs")
        .map_err(|e| e.to_string())?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (id, project, raw) = row.map_err(|e| e.to_string())?;
        let record: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let job = &record["job"];
        if record["version"]==2{validate_metadata(&record)?;if job["id"]!=id||job["projectId"]!=project{return Err("Чужая квитанция в истории проекта.".into());}continue;}
        if record["version"] != 1
            || job["id"] != id
            || job["projectId"] != project
            || job["readOnly"] != true
            || !record["activation"].is_null() && record["activation"]["state"] != "ineligible"
        {
            return Err("Повреждена связь перенесённой задачи или право её активации.".into());
        }
        for asset in std::iter::once(&job["sourceAssetId"])
            .chain(job["resultAssetIds"].as_array().into_iter().flatten())
        {
            let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_assets WHERE project_id=?1 AND asset_id=?2)",params![project,asset.as_str()],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !belongs {
                return Err("Файл перенесённой задачи относится к другому проекту.".into());
            }
        }
        for result in job["results"].as_array().into_iter().flatten() {
            let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_steps WHERE id=?1 AND project_id=?2 AND asset_id=?3 AND job_id=?4)",params![result["stepId"].as_str(),project,result["assetId"].as_str(),id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !belongs {
                return Err("Кадр перенесённой задачи относится к другому проекту.".into());
            }
        }
        for output in record["outputs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|o| o["accepted"] == true)
        {
            let belongs:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_steps WHERE id=?1 AND project_id=?2 AND asset_id=?3 AND job_id=?4)",params![output["stepId"].as_str(),project,output["assetId"].as_str(),id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !belongs {
                return Err("Принятый архивный ответ связан с чужим кадром.".into());
            }
        }
        for step in [
            &record["requestSnapshot"]["parentId"],
            &record["activation"]["sourceStepId"],
            &record["activation"]["activatedStepId"],
        ]
        .into_iter()
        .filter(|s| !s.is_null())
        {
            let belongs: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM editor_steps WHERE id=?1 AND project_id=?2)",
                    params![step.as_str(), project],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !belongs {
                return Err(
                    "Снимок или активация архивной задачи относится к чужому проекту.".into(),
                );
            }
        }
        if !record["requestSnapshot"]["draft"].is_null() {
            let draft: Draft = serde_json::from_value(record["requestSnapshot"]["draft"].clone())
                .map_err(|e| e.to_string())?;
            store::validate_draft_resources(db, &project, &draft)?;
        }
    }
    Ok(())
}

#[cfg(test)]mod cleanup_receipt_tests{
 use super::*;
 #[test]fn imported_receipt_loses_file_actions_but_preserves_ledger_after_cleanup(){
  let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE editor_imported_jobs(project_id TEXT,record TEXT);CREATE TABLE editor_project_tombstones(project_id TEXT);CREATE TABLE editor_assets(id TEXT);CREATE TABLE editor_resource_owners(project_id TEXT,asset_id TEXT);CREATE TABLE editor_working_steps(project_id TEXT,step_id TEXT);CREATE TABLE editor_step_visibility(step_id TEXT,hidden INTEGER);INSERT INTO editor_assets VALUES('a'),('b');INSERT INTO editor_resource_owners VALUES('p','a'),('p','b');INSERT INTO editor_working_steps VALUES('p','s');").unwrap();
  let record=json!({"version":1,"job":{"id":"receipt","projectId":"p","sourceAssetId":"a","resultAssetIds":["b"],"results":[{"assetId":"b","stepId":"s"}],"spentMicros":77,"requestId":"already-paid","state":"completed"}});db.execute("INSERT INTO editor_imported_jobs VALUES('p',?1)",[record.to_string()]).unwrap();
  let live=list(&db,Some("p")).unwrap();assert_eq!(live[0]["results"].as_array().unwrap().len(),1);
  for boundary in ["INSERT INTO editor_project_tombstones VALUES('p')","DELETE FROM editor_resource_owners WHERE asset_id='b'","DELETE FROM editor_working_steps"]{
   db.execute_batch(boundary).unwrap();let rows=list(&db,Some("p")).unwrap();assert_eq!(rows[0]["metadataOnly"],true);assert!(rows[0]["sourceAssetId"].is_null());assert_eq!(rows[0]["results"],json!([]));assert_eq!(rows[0]["resultAssetIds"],json!([]));assert_eq!(rows[0]["spentMicros"],77);assert_eq!(rows[0]["requestId"],"already-paid");
   let raw:String=db.query_row("SELECT record FROM editor_imported_jobs",[],|r|r.get(0)).unwrap();assert_eq!(serde_json::from_str::<Value>(&raw).unwrap(),record);db.execute_batch("DELETE FROM editor_project_tombstones;INSERT INTO editor_resource_owners SELECT 'p','b' WHERE NOT EXISTS(SELECT 1 FROM editor_resource_owners WHERE asset_id='b');").unwrap();
  }
 }
}
