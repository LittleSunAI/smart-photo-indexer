//! Annotation references belong to the document, not to an operation or selection row.
use super::{
    contracts::{AnnotationReference, Draft, Reference},
    local, store,
};
use crate::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock};

fn pending() -> &'static Mutex<HashSet<String>> {
    static P: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    P.get_or_init(|| Mutex::new(HashSet::new()))
}
pub fn has_pending() -> Result<bool, String> {
    Ok(!pending()
        .lock()
        .map_err(|_| "Reference import lock failed")?
        .is_empty())
}
pub fn is_pending(project: &str) -> Result<bool, String> {
    Ok(pending()
        .lock()
        .map_err(|_| "Reference import lock failed")?
        .contains(project))
}
pub struct ImportPermit(String);
impl Drop for ImportPermit {
    fn drop(&mut self) {
        if let Ok(mut p) = pending().lock() {
            p.remove(&self.0);
        }
    }
}
pub fn claim(
    app: &tauri::AppHandle,
    project: &str,
    revision: u64,
    target: &str,
    replace: bool,
    count: usize,
) -> Result<ImportPermit, String> {
    let mut db = store::connection(app)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    store::require_mutable(&tx, project)?;
    let (current, raw): (u64, String) = tx
        .query_row(
            "SELECT revision,draft FROM editor_projects WHERE id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if current != revision {
        return Err("Проект изменился до прикрепления. Повторите для выбранной метки.".into());
    }
    let d: Draft = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    check_target(&d, target, replace, count)?;
    let limit = route_limit(app, &d)?;
    // The completed candidate is checked atomically after canonical deduplication.
    // Counting selected files here would incorrectly reject duplicate references at capacity.
    let _ = limit;
    let mut p = pending()
        .lock()
        .map_err(|_| "Reference import lock failed")?;
    if !p.insert(project.into()) {
        return Err("Дождитесь завершения прикрепления.".into());
    }
    drop(p);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ImportPermit(project.into()))
}
fn check_target(d: &Draft, target: &str, replace: bool, count: usize) -> Result<(), String> {
    if count == 0 || count > 5000 {return Err("Выберите дополнительные картинки в пределах лимита модели.".into());}
    if target == "image" {
        return if ["universal-edit","replace-object","compose"].contains(&d.operation.as_str()){Ok(())}else{Err("Общие дополнительные картинки доступны для правки по описанию, добавления объекта и композиции.".into())};
    }
    let a = d
        .annotations
        .iter()
        .find(|a| a["id"].as_str() == Some(target))
        .ok_or("Метка уже удалена")?;
    if !super::instructions::local(&d.operation)
        || !["point", "rect"].contains(&a["kind"].as_str().unwrap_or(""))
    {
        return Err("Дополнительная картинка прикрепляется к точке или области локальной правки.".into());
    }
    if a["kind"] == "point"
        && (count != 1
            || !replace
                && d.annotation_references
                    .get(target)
                    .is_some_and(|v| !v.is_empty()))
    {
        return Err("У точки одна дополнительная картинка. Используйте «Заменить изображение».".into());
    }
    Ok(())
}
fn clear_references(d:&mut Draft,target:&str,replace:bool){
    if !replace{return;}
    if target=="image"{d.references.clear();}else{d.annotation_references.remove(target);}
}
fn add_reference(d:&mut Draft,target:&str,id:String,display_name:Option<String>){
    if target=="image"{
        if !d.references.iter().any(|r|r.asset_id==id){d.references.push(Reference{asset_id:id,role:"reference".into()});}
    }else{
        let items=d.annotation_references.entry(target.into()).or_default();
        if !items.iter().any(|r|r.asset_id==id){items.push(AnnotationReference{asset_id:id,included:true,display_name,..Default::default()});}
    }
}
pub fn route_limit(app: &tauri::AppHandle, d: &Draft) -> Result<usize, String> {
    let c = super::review::active(app)?;
    let r = c["routes"]
        .as_array()
        .ok_or("Нет каталога")?
        .iter()
        .find(|r| r["id"].as_str() == d.model_id.as_deref())
        .ok_or("Выберите модель перед добавлением дополнительной картинки")?;
    if !r["operations"]
        .as_array()
        .is_some_and(|ops| ops.iter().any(|op| op == &d.operation))
    {
        return Err("Модель не поддерживает эту правку".into());
    }
    let n = r["maxInputs"].as_u64().unwrap_or(1) as usize;
    if n <= 1 {
        return Err(
            "Эта модель не принимает дополнительные картинки. Выберите совместимую модель.".into(),
        );
    }
    Ok(n)
}
pub fn validate(
    annotations: &[Value],
    refs: &BTreeMap<String, Vec<AnnotationReference>>,
) -> Result<(), String> {
    if refs.len() > 5000 {
        return Err("Слишком много привязок дополнительных картинок".into());
    }
    for (id, items) in refs {
        let a = annotations
            .iter()
            .find(|a| a["id"].as_str() == Some(id))
            .ok_or("Дополнительная картинка ссылается на удалённую метку")?;
        if !["point", "rect"].contains(&a["kind"].as_str().unwrap_or(""))
            || a["kind"] == "point" && items.len() > 1
            || items.len() > 5000
        {
            return Err("Неверная привязка фотографии к метке".into());
        }
        let mut unique = HashSet::new();
        for r in items {
            if r.asset_id.is_empty() || r.asset_id.len() > 160 || r.display_name.as_ref().is_some_and(|v|v.len()>1024) || r.role_description.as_ref().is_some_and(|v|v.len()>8000) || !unique.insert(&r.asset_id) {
                return Err("Некорректная или повторяющаяся дополнительная картинка метки".into());
            }
        }
    }
    Ok(())
}
pub fn participating(d: &Draft) -> Vec<String> {
    if !super::instructions::local(&d.operation) { return vec![]; }
    d.annotations.iter().filter(|a|a["included"]==true&&["point","rect"].contains(&a["kind"].as_str().unwrap_or("")))
        .filter_map(|a|a["id"].as_str().map(String::from)).collect()
}
pub fn capacity(app: &tauri::AppHandle, project: &str, d: &Draft, prospective: Option<&str>) -> Result<Vec<Value>, String> {
    inputs_with(d, prospective, false, |id| canonical_asset(app, local::project_asset(app, project, id)?, d))
}
fn canonical_asset(app: &tauri::AppHandle, mut a: Value, d: &Draft) -> Result<Value,String> {
    use sha2::{Digest,Sha256};
    static KEYS: OnceLock<Mutex<std::collections::HashMap<(String,u8,bool),String>>> = OnceLock::new();
    let cache = KEYS.get_or_init(||Mutex::new(std::collections::HashMap::new()));
    let raw=a["sha256"].as_str().ok_or("Asset hash missing")?.to_string();
    let id=a["id"].as_str().unwrap_or("");
    let orientation=if id==d.asset_id {(d.working_turn,d.mirrored)}else{(0,false)};
    for (field,turn,mirror) in [("canonicalSha256",0,false),("orientedCanonicalSha256",orientation.0,orientation.1)] {
        let key=(raw.clone(),turn,mirror);
        let cached=cache.lock().map_err(|_|"Canonical cache lock failed")?.get(&key).cloned();
        let hash=if let Some(hash)=cached {hash}else{
            let path=store::owned_path(app,a["relativePath"].as_str().ok_or("Asset path missing")?)?;
            let image=super::geometry::orient_image(file_safety::decode_bounded(&path)?,turn,mirror).into_rgba8();
            let mut hash=Sha256::new();hash.update(image.width().to_le_bytes());hash.update(image.height().to_le_bytes());hash.update(image.as_raw());
            let hash=format!("{:x}",hash.finalize());
            let mut values=cache.lock().map_err(|_|"Canonical cache lock failed")?;
            if values.len()>4096 {values.clear();}values.insert(key,hash.clone());hash
        };
        a[field]=json!(hash);
    }
    Ok(a)
}
pub fn inputs(
    app: &tauri::AppHandle,
    project: &str,
    d: &Draft,
    prospective: Option<&str>,
) -> Result<Vec<Value>, String> {
    inputs_with(d, prospective, true, |id| canonical_asset(app, local::project_asset(app, project, id)?, d))
}
fn inputs_with(
    d: &Draft,
    prospective: Option<&str>,
    request_only: bool,
    mut asset: impl FnMut(&str) -> Result<Value, String>,
) -> Result<Vec<Value>, String> {
    let a = asset(&d.asset_id)?;
    let mut out = vec![
        json!({"assetId":d.asset_id,"role":"main","width":a["width"],"height":a["height"],"annotationIds":[],"bindings":[]}),
    ];
    let mut keys = BTreeMap::new();
    keys.insert(
        a["orientedCanonicalSha256"].as_str().map(String::from).unwrap_or_else(||format!("{}|{}|{}", a.get("canonicalSha256").unwrap_or(&a["sha256"]), d.working_turn, d.mirrored)),
        0usize,
    );
    let mut add = |id: &str, role: &str, target: Option<&str>, binding: Option<&AnnotationReference>| -> Result<(), String> {
        let a = asset(id)?;
        if a["kind"] != "image" {
            return Err("Дополнительная картинка должна быть фотографией, не маской".into());
        }
        let key = a["canonicalSha256"].as_str().map(String::from).unwrap_or_else(||format!("{}|0|false", a["sha256"]));
        let index = if let Some(i) = keys.get(&key) {
            *i
        } else {
            let i = out.len();
            keys.insert(key, i);
            out.push(json!({"assetId":id,"role":role,"width":a["width"],"height":a["height"],"annotationIds":[],"bindings":[]}));
            i
        };
        if let Some(target) = target {
            let ids = out[index]["annotationIds"].as_array_mut().unwrap();
            if !ids.iter().any(|v| v == target) {
                ids.push(json!(target));
            }
            let description=super::instructions::tasks(d)?.1.into_iter().find(|(id,_)|id==target).map(|(_,text)|text).unwrap_or_default();
            out[index]["bindings"].as_array_mut().unwrap().push(json!({"annotationId":target,"displayName":binding.and_then(|r|r.display_name.as_deref()).unwrap_or(id),"roleDescription":binding.and_then(|r|r.role_description.as_deref()).unwrap_or(""),"instruction":description}));
        }
        Ok(())
    };
    for r in &d.references {
        add(&r.asset_id, &r.role, None, None)?;
    }
    let mut targets = if request_only { super::instructions::tasks(d)?.1.into_iter().map(|(id,_)|id).collect() } else { participating(d) };
    if let Some(id) = prospective {
        if !targets.iter().any(|t| t == id) {
            targets.push(id.into());
        }
    }
    for a in &d.annotations {
        let Some(id) = a["id"].as_str() else {
            continue;
        };
        if !targets.iter().any(|t| t == id) {
            continue;
        }
        if let Some(items) = d.annotation_references.get(id) {
            for r in items.iter().filter(|r| r.included) {
                add(&r.asset_id, "annotation-reference", Some(id), Some(r))?;
            }
        }
    }
    Ok(out)
}
pub fn attach(
    app: &tauri::AppHandle,
    project: &str,
    revision: u64,
    target: &str,
    replace: bool,
    paths: &[std::path::PathBuf],
    still_current: impl Fn() -> Result<(), String>,
) -> Result<Value, String> {
    let _claim = claim(app, project, revision, target, replace, paths.len())?;
    let mut imported = vec![];
    for path in paths {
        let path = validate_existing_image_file(&path_to_string(path))?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let image = file_safety::decode_bounded(&path)?;
        let mut saved=local::save_image(app, &image, "image")?;saved["displayName"]=json!(file_name(&path));imported.push(saved);
    }
    commit_prepared(app, project, revision, target, replace, imported, still_current)
}
fn commit_prepared(
    app: &tauri::AppHandle, project: &str, revision: u64, target: &str,
    replace: bool, imported: Vec<Value>, still_current: impl Fn() -> Result<(), String>,
) -> Result<Value, String> {
    still_current()?;
    let mut db = store::connection(app)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let (now, raw): (u64, String) = tx
        .query_row(
            "SELECT revision,draft FROM editor_projects WHERE id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if now != revision {
        return Err(
            "Проект изменился во время импорта. Прежняя дополнительная картинка сохранена; повторите прикрепление."
                .into(),
        );
    }
    let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_jobs WHERE project_id=?1 AND state NOT IN ('completed','failed','cancelled','unknownSubmit','interrupted'))",[project],|r|r.get(0)).map_err(|e|e.to_string())?;
    if busy {
        return Err("Во время импорта началась задача; дополнительная картинка не прикреплена".into());
    }
    let mut d: Draft = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    check_target(&d, target, replace, imported.len())?;
    if target!="image"{d.push_annotation_undo();}
    clear_references(&mut d,target,replace);
    for a in imported {
        let id = a["id"]
            .as_str()
            .ok_or("Нет импортированного asset")?
            .to_string();
        tx.execute("INSERT OR IGNORE INTO editor_project_assets(project_id,asset_id,role) VALUES(?1,?2,'reference')",params![project,id]).map_err(|e|e.to_string())?;
        add_reference(&mut d,target,id,a["displayName"].as_str().map(String::from));
    }
    let limit = route_limit(app, &d)?;
    let count = inputs_with(&d, Some(target), false, |id| canonical_asset(app, store::asset_value(&tx, id)?, &d))?.len();
    if count > limit {
        return Err("Превышен общий лимит фотографий; прежние связи сохранены".into());
    }
    store::validate_draft_resources(&tx, project, &d)?;
    tx.execute(
        "UPDATE editor_projects SET draft=?2,revision=revision+1,updated_at_unix=?3 WHERE id=?1",
        params![
            project,
            serde_json::to_string(&d).map_err(|e| e.to_string())?,
            unix_now()
        ],
    )
    .map_err(|e| e.to_string())?;
    store::stash_step_draft(&tx, project, &d)?;
    tx.commit().map_err(|e| e.to_string())?;
    store::project_snapshot(app, project)
}
/// Attach the visible frame, including its saved working orientation, as immutable pixels.
/// No new history step is created and no caller-supplied path or rotation is trusted.
pub fn attach_step(
    app: &tauri::AppHandle, project: &str, revision: u64, target: &str,
    replace: bool, step: &str, still_current: impl Fn() -> Result<(), String>,
) -> Result<Value, String> {
    let _claim = claim(app, project, revision, target, replace, 1)?;
    let db = store::connection(app)?;
    let (asset_id, title): (String,String) = db.query_row(
        "SELECT s.asset_id,s.title FROM editor_steps s LEFT JOIN editor_step_visibility v ON v.step_id=s.id WHERE s.id=?1 AND s.project_id=?2 AND COALESCE(v.hidden,0)=0",
        params![step,project], |r|Ok((r.get(0)?,r.get(1)?))
    ).optional().map_err(|e|e.to_string())?.ok_or("Кадр не принадлежит текущей ленте проекта.")?;
    let (active,raw):(String,String)=db.query_row("SELECT active_step_id,draft FROM editor_projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    let source = if active==step {Some(serde_json::from_str::<Draft>(&raw).map_err(|e|e.to_string())?)} else {store::step_draft(&db,step)?};
    let (turn,mirrored)=source.map(|d|(d.working_turn,d.mirrored)).unwrap_or_default();
    drop(db);
    let asset=local::project_asset(app,project,&asset_id)?;
    if asset["kind"]!="image" {return Err("Дополнительная картинка должна быть изображением.".into());}
    let path=store::owned_path(app,asset["relativePath"].as_str().ok_or("Нет файла кадра")?)?;
    if compute_file_sha256(&path)?!=asset["sha256"].as_str().unwrap_or(""){return Err("Файл кадра изменился; дополнительная картинка не прикреплена.".into());}
    let mut prepared=if turn==0&&!mirrored {asset} else {
        let image=super::geometry::orient_image(file_safety::decode_bounded(&path)?,turn,mirrored);
        local::save_image(app,&image,"image")?
    };
    prepared["displayName"]=json!(title);
    commit_prepared(app,project,revision,target,replace,vec![prepared],still_current)
}
pub fn attach_existing(
    app: &tauri::AppHandle,
    project: &str,
    revision: u64,
    target: &str,
    replace: bool,
    ids: &[String],
    still_current: impl Fn() -> Result<(), String>,
) -> Result<Value, String> {
    let mut db = store::connection(app)?;
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    store::require_mutable(&tx, project)?;
    still_current()?;
    let (now, raw): (u64, String) = tx
        .query_row(
            "SELECT revision,draft FROM editor_projects WHERE id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if now != revision {
        return Err("Проект изменился до прикрепления источника".into());
    }
    let mut d: Draft = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    check_target(&d, target, replace, ids.len())?;
    if target!="image"{d.push_annotation_undo();}
    clear_references(&mut d,target,replace);
    for id in ids {
        let belongs:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM editor_project_assets WHERE project_id=?1 AND asset_id=?2)",params![project,id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !belongs || store::asset_value(&tx, id)?["kind"] != "image" {
            return Err(
                "Выбранный источник не принадлежит проекту или не является фотографией".into(),
            );
        }
        add_reference(&mut d,target,id.clone(),None);
    }
    let limit = route_limit(app, &d)?;
    if inputs_with(&d, Some(target), false, |id| canonical_asset(app, store::asset_value(&tx, id)?, &d))?.len() > limit {
        return Err("Превышен общий лимит изображений; прежняя связь сохранена".into());
    }
    store::validate_draft_resources(&tx, project, &d)?;
    tx.execute(
        "UPDATE editor_projects SET draft=?2,revision=revision+1,updated_at_unix=?3 WHERE id=?1",
        params![
            project,
            serde_json::to_string(&d).map_err(|e| e.to_string())?,
            unix_now()
        ],
    )
    .map_err(|e| e.to_string())?;
    store::stash_step_draft(&tx, project, &d)?;
    tx.commit().map_err(|e| e.to_string())?;
    store::project_snapshot(app, project)
}

#[cfg(test)]
mod image_target_tests{
    use super::*;
    #[test]
    fn whole_target_uses_existing_general_references_and_canonical_inputs(){
        let mut d=Draft::initial("main");d.operation="compose".into();
        assert!(check_target(&d,"image",false,2).is_ok());
        add_reference(&mut d,"image","sample".into(),None);add_reference(&mut d,"image","sample".into(),None);
        assert_eq!(d.references.len(),1);assert!(d.annotation_references.is_empty());
        let inputs=inputs_with(&d,Some("image"),false,|id|Ok(json!({"id":id,"kind":"image","sha256":id,"width":512,"height":512}))).unwrap();
        assert_eq!(inputs.len(),2);assert_eq!(inputs[1]["assetId"],"sample");
        d.operation="outpaint".into();assert!(check_target(&d,"image",false,1).is_err());
    }
    #[test]
    fn replacing_general_samples_keeps_local_bindings(){
        let mut d=Draft::initial("main");add_reference(&mut d,"image","one".into(),None);add_reference(&mut d,"point","local".into(),None);
        clear_references(&mut d,"image",true);add_reference(&mut d,"image","two".into(),None);
        assert_eq!(d.references[0].asset_id,"two");assert_eq!(d.annotation_references["point"][0].asset_id,"local");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Draft {
        let mut d = Draft::initial("main");
        d.annotations = vec![
            json!({"id":"p","assetId":"main","kind":"point","point":{"x":0.2,"y":0.4},"instruction":"cap","visible":false,"included":true}),
            json!({"id":"r","assetId":"main","kind":"rect","start":{"x":0.1,"y":0.1},"end":{"x":0.5,"y":0.5},"instruction":"table","visible":true,"included":true}),
        ];
        d.selected_annotation_id = Some("p".into());
        d
    }
    #[test]
    fn point_cardinality_and_dangling_bindings_rejected() {
        let mut d = draft();
        d.annotation_references.insert(
            "p".into(),
            vec![
                AnnotationReference {
                    asset_id: "ref".into(),
                    included: true,
                ..Default::default()
                },
                AnnotationReference {
                    asset_id: "ref2".into(),
                    included: true,
                ..Default::default()
                },
            ],
        );
        assert!(d.validate().is_err());
        d.annotation_references.remove("p");
        d.annotation_references.insert("missing".into(), vec![]);
        assert!(d.validate().is_err());
    }
    #[test]
    fn shared_content_deduplicated_without_losing_target_mapping() {
        let mut d = draft();
        d.form_values.insert(
            "__instructionContext".into(),
            json!({"version":1,"operations":{"universal-edit":{"mode":"multi","target":"p","enabled":["p","r"],"texts":{"p":"cap","r":"table"},"globalText":"","reviewed":true}},"projection":{"prompt":"","annotations":{"p":"cap","r":"table"}}}),
        );
        for id in ["p", "r"] {
            d.annotation_references.insert(
                id.into(),
                vec![AnnotationReference {
                    asset_id: "ref".into(),
                    included: true,
                ..Default::default()
                }],
            );
        }
        let out = inputs_with(&d, None, false, |id| {
            Ok(json!({"kind":"image","sha256":id,"width":500,"height":300}))
        })
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["annotationIds"], json!(["p", "r"]));
        d.mirrored = true;
        d.references.push(super::super::contracts::Reference {
            asset_id: "main".into(),
            role: "reference".into(),
        });
        let out = inputs_with(&d, None, false, |id| {
            Ok(json!({"kind":"image","sha256":id,"width":500,"height":300}))
        })
        .unwrap();
        assert_eq!(out.len(), 3);
    }
    #[test]
    fn disabled_target_preserved_but_not_sent() {
        let mut d = draft();
        d.annotations[1]["included"]=json!(false);
        d.annotation_references.insert(
            "r".into(),
            vec![AnnotationReference {
                asset_id: "ref".into(),
                included: true,
                ..Default::default()
            }],
        );
        let out = inputs_with(&d, None, false, |id| {
            Ok(json!({"kind":"image","sha256":id,"width":500,"height":300}))
        })
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(d.annotation_references.len(), 1);
        assert_eq!(
            inputs_with(&d, Some("r"), false, |id| Ok(
                json!({"kind":"image","sha256":id,"width":500,"height":300})
            ))
            .unwrap()
            .len(),
            2
        );
    }
    #[test]
    fn empty_tasks_reserve_capacity_but_not_request_inputs() {
        let mut d=draft();d.annotations[1]["instruction"]=json!("");
        d.annotation_references.insert("r".into(),vec![AnnotationReference{asset_id:"ref".into(),included:true,..Default::default()}]);
        let asset=|id:&str|Ok(json!({"kind":"image","sha256":id,"width":500,"height":300}));
        assert_eq!(inputs_with(&d,None,false,asset).unwrap().len(),2);
        assert_eq!(inputs_with(&d,None,true,asset).unwrap().len(),1);
        d.annotations[1]["instruction"]=json!("Стол");
        assert_eq!(inputs_with(&d,None,true,asset).unwrap().len(),2);
    }

}
