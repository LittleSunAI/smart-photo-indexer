use crate::*;
use serde_json::{json,Value};
use tauri::Emitter;
#[derive(Serialize,Deserialize)]#[serde(rename_all="camelCase")]
struct VerifiedPath{path:String,revision:String,size_bytes:u64}
#[derive(Serialize,Deserialize)]
struct DuplicateSnapshot{version:u32,verified:Vec<VerifiedPath>,members:Vec<String>}
#[derive(Deserialize)]#[serde(untagged)]
enum StoredSnapshot{Current(DuplicateSnapshot),Legacy(Vec<VerifiedPath>)}
fn read_snapshot(raw:&str)->Result<DuplicateSnapshot,String>{
 let snapshot=match serde_json::from_str::<StoredSnapshot>(raw).map_err(|e|e.to_string())?{
  StoredSnapshot::Current(value) if value.version==1=>value,
  StoredSnapshot::Current(_)=>return Err("Версия группы копий не поддерживается. Повторите поиск.".into()),
  StoredSnapshot::Legacy(verified)=>DuplicateSnapshot{version:1,members:verified.iter().map(|v|v.path.clone()).collect(),verified},
 };
 let members:std::collections::HashSet<_>=snapshot.members.iter().collect();
 let verified:std::collections::HashSet<_>=snapshot.verified.iter().map(|v|&v.path).collect();
 if members.len()!=snapshot.members.len()||verified.len()!=snapshot.verified.len()||!verified.is_subset(&members){return Err("Состав сохранённой группы повреждён. Повторите поиск.".into());}
 Ok(snapshot)
}
fn paths(db:&Connection,image:&str)->Result<Vec<String>,String>{let mut q=db.prepare("SELECT path FROM image_paths WHERE image_id=?1 AND is_current=1 AND missing_at_unix IS NULL ORDER BY path").map_err(|e|e.to_string())?;let rows=q.query_map([image],|r|r.get(0)).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())}
fn fingerprint(sha:&str,verified:&[VerifiedPath])->String{
 let mut paths:Vec<String>=verified.iter().map(|p|p.path.replace('/',"\\").to_lowercase()).collect();paths.sort();paths.dedup();
 format!("{:x}",Sha256::digest(serde_json::to_vec(&(sha,paths)).expect("String paths serialize")))
}
fn plan(db:&Connection,image:&str,sha:&str)->Result<Option<Value>,String>{
 let members=paths(db,image)?;let mut verified=Vec::new();let mut skipped=Vec::new();for path in members.iter().cloned(){let source=Path::new(&path);match index_observations::observe(db,source,true){Ok((actual,_,stamp)) if actual==sha=>{let size=fs::metadata(source).map_err(|e|e.to_string())?.len();verified.push(VerifiedPath{path,revision:index_observations::observed_revision(&stamp,&actual),size_bytes:size});},Ok(_)=>skipped.push(json!({"path":path,"reason":"Содержимое изменилось"})),Err(error)=>skipped.push(json!({"path":path,"reason":error}))}}
 if verified.len()<2{return Ok(None);}
 let accepted:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM duplicate_decisions WHERE fingerprint=?1)",[fingerprint(sha,&verified)],|r|r.get(0)).map_err(|e|e.to_string())?;if accepted{return Ok(None);}
 let count=verified.len();let snapshot=DuplicateSnapshot{version:1,verified,members};let nonce=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();let id=format!("duplicates-{}-{nonce}",std::process::id());db.execute("INSERT INTO duplicate_plans(id,image_id,sha256,paths,created_at_unix) VALUES(?1,?2,?3,?4,?5)",params![id,image,sha,serde_json::to_string(&snapshot).map_err(|e|e.to_string())?,unix_now()]).map_err(|e|e.to_string())?;Ok(Some(json!({"planId":id,"imageId":image,"copies":count,"skipped":skipped})))
}
fn accept_group(db:&mut Connection,plan_id:&str)->Result<Value,String>{
 let(image,sha,raw,state,created):(String,String,String,String,u64)=db.query_row("SELECT image_id,sha256,paths,state,created_at_unix FROM duplicate_plans WHERE id=?1",[plan_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|"Группа недоступна. Повторите поиск копий.")?;
 if !["ready","accepted"].contains(&state.as_str())||unix_now().saturating_sub(created)>3600{return Err("Группа устарела. Повторите поиск копий.".into());}
 let snapshot=read_snapshot(&raw)?;let current=paths(db,&image)?;if current!=snapshot.members{return Err("Состав группы изменился. Повторите поиск копий.".into());}
 let mut verified=Vec::new();for path in &current{if let Ok(hash)=compute_file_sha256(Path::new(path)){if hash==sha{verified.push(VerifiedPath{path:path.clone(),revision:String::new(),size_bytes:0});}}}
 if verified.len()<2||fingerprint(&sha,&verified)!=fingerprint(&sha,&snapshot.verified){return Err("Подтверждённые копии изменились. Обновите группу перед решением.".into());}
 let fingerprint=fingerprint(&sha,&verified);let saved_paths:Vec<_>=verified.iter().map(|p|p.path.as_str()).collect();
 let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 if paths(&tx,&image)?!=current{return Err("Состав группы изменился. Повторите поиск копий.".into());}
 tx.execute("INSERT INTO duplicate_decisions(fingerprint,sha256,paths,created_at_unix) VALUES(?1,?2,?3,?4) ON CONFLICT(fingerprint) DO NOTHING",params![fingerprint,sha,json!(saved_paths).to_string(),unix_now()]).map_err(|e|e.to_string())?;
 tx.execute("UPDATE duplicate_plans SET state='accepted' WHERE id=?1",[plan_id]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(json!({"accepted":true,"copies":verified.len()}))
}
#[tauri::command]
pub async fn accept_duplicate_group(app:tauri::AppHandle,plan_id:String)->Result<Value,String>{let owner=app.clone();let result=tauri::async_runtime::spawn_blocking(move||{let mut db=open_index_connection(&owner)?;run_index_migrations(&mut db)?;accept_group(&mut db,&plan_id)}).await.map_err(|e|e.to_string())??;app.emit("smart-photo-indexer:duplicates-changed",&result).map_err(|e|e.to_string())?;Ok(result)}
#[tauri::command]
pub async fn reset_duplicate_decisions(app:tauri::AppHandle)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||{let db=open_index_connection(&app)?;let count=db.execute("DELETE FROM duplicate_decisions",[]).map_err(|e|e.to_string())?;Ok(json!({"reset":count}))}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn get_duplicate_groups(app:tauri::AppHandle,after_id:Option<String>,image_id:Option<String>)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||{
 let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let candidates:Vec<(String,String,String)>={let mut q=db.prepare("SELECT i.id,i.file_name,i.sha256 FROM images i JOIN image_paths p ON p.image_id=i.id AND p.is_current=1 AND p.missing_at_unix IS NULL WHERE i.sha256 IS NOT NULL AND i.id>?1 AND (?2 IS NULL OR i.id=?2) GROUP BY i.id HAVING COUNT(p.id)>1 ORDER BY i.id LIMIT 33").map_err(|e|e.to_string())?;let rows=q.query_map(params![after_id.unwrap_or_default(),image_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;rows.collect::<Result<_,_>>().map_err(|e|e.to_string())?};let more=candidates.len()>32;let next=if more{candidates.get(31).map(|v|v.0.clone())}else{None};let mut items=Vec::new();for(image,name,sha)in candidates.into_iter().take(32){if let Some(mut group)=plan(&db,&image,&sha)?{group["fileName"]=json!(name);items.push(group);}}Ok(json!({"items":items,"nextAfterId":next}))}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn get_duplicate_paths(app:tauri::AppHandle,image_id:String,plan_id:String,after_path:Option<String>)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||{
 let db=open_index_connection(&app)?;let raw:String=db.query_row("SELECT paths FROM duplicate_plans WHERE id=?1 AND image_id=?2 AND state='ready'",params![plan_id,image_id],|r|r.get(0)).map_err(|_|"Группа устарела. Повторите поиск копий.")?;let entries=read_snapshot(&raw)?.verified;let start=after_path.map(|p|entries.iter().position(|v|v.path==p).map(|v|v+1).ok_or("Страница группы устарела")).transpose()?.unwrap_or(0);let page=&entries[start..entries.len().min(start+64)];Ok(json!({"imageId":image_id,"items":page,"nextAfterPath":if start+64<entries.len(){page.last().map(|v|v.path.clone())}else{None},"total":entries.len()}))}).await.map_err(|e|e.to_string())?}
#[tauri::command]
pub async fn keep_duplicate_copies(app:tauri::AppHandle,plan_id:String,keep_paths:Vec<String>)->Result<Value,String>{let owner=app.clone();let result=tauri::async_runtime::spawn_blocking(move||processing::cloud_fence(&owner,||{
 let _files=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File lock failed")?;let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;let mut db=open_index_connection(&owner)?;
 let(image,sha,raw,state,created):(String,String,String,String,u64)=db.query_row("SELECT image_id,sha256,paths,state,created_at_unix FROM duplicate_plans WHERE id=?1",[&plan_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(|_|"План копий недоступен")?;
 if state!="ready"||unix_now().saturating_sub(created)>3600{return Err("План уже применён или устарел. Обновите копии.".into());}let snapshot=read_snapshot(&raw)?;let entries=snapshot.verified;let keep:std::collections::HashSet<String>=keep_paths.into_iter().collect();if keep.is_empty()||keep.iter().any(|p|!entries.iter().any(|e|&e.path==p)){return Err("Отметьте хотя бы одну копию из подтверждённой группы.".into());}
 let active:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM file_operations WHERE image_id=?1 AND status IN('applying','needsRecovery')) OR EXISTS(SELECT 1 FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.image_id=?1 AND j.status IN('queued','running','paused','interrupted'))",[&image],|r|r.get(0)).map_err(|e|e.to_string())?;if active{return Err("Копии защищены незавершённой файловой операцией или обработкой.".into());}
 let current=paths(&db,&image)?;if current!=snapshot.members{return Err("Состав группы изменился. Повторите поиск перед очисткой.".into());}
 let mut protected=Vec::new();for entry in &entries{if keep.contains(&entry.path){let mut guard=guarded_file::GuardedFile::open(Path::new(&entry.path),false)?;if guard.sha()?!=sha||index_observations::content_revision(Path::new(&entry.path))?!=entry.revision{return Err("Сохраняемая копия изменилась; файлы не удалены.".into());}protected.push(guard);}}
 // Preflight all members before the first file move; each deletion reopens and hashes its own stable handle.
 for entry in &entries{if index_observations::content_revision(Path::new(&entry.path))?!=entry.revision||compute_file_sha256(Path::new(&entry.path))?!=sha{return Err("Копия изменилась или недоступна. Файлы не удалены; обновите группу.".into());}}
 db.execute("UPDATE duplicate_plans SET state='applying' WHERE id=?1",[&plan_id]).map_err(|e|e.to_string())?;let mut removed=Vec::new();let mut errors=Vec::new();for entry in entries.iter().filter(|v|!keep.contains(&v.path)){let outcome=(||{let mut guard=guarded_file::GuardedFile::open(Path::new(&entry.path),true)?;if guard.sha()?!=sha||index_observations::content_revision(Path::new(&entry.path))?!=entry.revision{return Err("Копия изменилась перед удалением".into());}file_operations::trash_duplicate_locked(&mut db,&entry.path,guard,&sha,&image)})();match outcome{Ok(())=>removed.push(entry.path.clone()),Err(error)=>{errors.push(json!({"path":entry.path,"reason":error}));break;}}}
 let result=json!({"removedPaths":removed,"keptPaths":keep,"errors":errors,"complete":errors.is_empty(),"total":entries.len()});db.execute("UPDATE duplicate_plans SET state=?2,result=?3 WHERE id=?1",params![plan_id,if errors.is_empty(){"completed"}else{"partial"},result.to_string()]).map_err(|e|e.to_string())?;drop(protected);Ok(result)
})).await.map_err(|e|e.to_string())??;if let Some(paths)=result["removedPaths"].as_array(){for path in paths{if let Some(path)=path.as_str(){viewer_sessions::remove_path(&app,path)?;}}}app.emit("smart-photo-indexer:duplicates-changed",&result).map_err(|e|e.to_string())?;Ok(result)}

#[tauri::command]
pub async fn verify_duplicate_paths(app:tauri::AppHandle,image_id:String,paths:Vec<String>)->Result<serde_json::Value,String>{
    tauri::async_runtime::spawn_blocking(move||{
        if paths.is_empty() || paths.len()>64{return Err("За одну проверку можно сверить 1–64 копии.".into());}
        let db=open_index_connection(&app)?;let hash:String=db.query_row("SELECT sha256 FROM images WHERE id=?1",[&image_id],|r|r.get(0)).map_err(|_|"Фотография не найдена.")?;
        for path in &paths{let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM image_paths WHERE image_id=?1 AND path=?2 AND is_current=1)",params![image_id,path],|r|r.get(0)).map_err(|e|e.to_string())?;if !valid{return Err("Выбранный путь больше не относится к этой фотографии; обновите список.".into());}}
        drop(db);
        let items=paths.into_iter().map(|path|match compute_file_sha256(Path::new(&path)){Ok(actual)=>serde_json::json!({"path":path,"matches":actual==hash,"sha256":actual}),Err(error)=>serde_json::json!({"path":path,"matches":false,"error":error})}).collect::<Vec<_>>();
        // Verifying candidates never removes, relocates or rewrites any physical copy.
        Ok(serde_json::json!({"imageId":image_id,"expectedSha256":hash,"items":items}))
    }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn accepted_composition_survives_new_plan_but_new_copy_reappears_and_reset_keeps_files(){
  let root=crate::test_support::temp_dir("smart-photo-duplicate-decisions");let database=root.join("index.sqlite");let mut db=Connection::open(&database).unwrap();run_index_migrations(&mut db).unwrap();
  let first=root.join("a.png");fs::write(&first,b"same").unwrap();let sha=compute_file_sha256(&first).unwrap();
  db.execute("INSERT INTO images(id,identity_key,file_name,sha256,description,created_at_unix,updated_at_unix) VALUES('i','owned','a.png',?1,'Keep description',1,1)",[&sha]).unwrap();
  for name in ["a.png","b.png"]{let path=root.join(name);fs::write(&path,b"same").unwrap();db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('i',?1,1,1,1)",[path_to_string(&path)]).unwrap();}
  let group=plan(&db,"i",&sha).unwrap().unwrap();accept_group(&mut db,group["planId"].as_str().unwrap()).unwrap();drop(db);let mut db=Connection::open(&database).unwrap();run_index_migrations(&mut db).unwrap();assert!(plan(&db,"i",&sha).unwrap().is_none());
  fs::write(&first,b"same").unwrap();assert!(plan(&db,"i",&sha).unwrap().is_none());
  let third=root.join("c.png");fs::write(&third,b"same").unwrap();db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('i',?1,1,1,1)",[path_to_string(&third)]).unwrap();assert_eq!(plan(&db,"i",&sha).unwrap().unwrap()["copies"],3);
  assert!(accept_group(&mut db,group["planId"].as_str().unwrap()).is_err());db.execute("DELETE FROM duplicate_decisions",[]).unwrap();let fresh=plan(&db,"i",&sha).unwrap().unwrap();assert_eq!(db.query_row("SELECT description FROM images WHERE id='i'",[],|r|r.get::<_,String>(0)).unwrap(),"Keep description");
  let mut readonly=Connection::open_with_flags(&database,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();assert!(accept_group(&mut readonly,fresh["planId"].as_str().unwrap()).is_err());assert_eq!(db.query_row("SELECT COUNT(*) FROM duplicate_decisions",[],|r|r.get::<_,u32>(0)).unwrap(),0);drop(readonly);drop(db);
  for name in ["a.png","b.png","c.png"]{let path=root.join(name);assert_eq!(fs::read(&path).unwrap(),b"same");fs::remove_file(path).unwrap();}fs::remove_file(database).unwrap();fs::remove_dir(root).unwrap();
 }
 fn entry(path:&str)->Value{json!({"path":path,"revision":"revision","sizeBytes":1})}
 #[test]fn legacy_snapshot_preserves_its_original_membership(){let snapshot=read_snapshot(&json!([entry("a"),entry("b")]).to_string()).unwrap();assert_eq!(snapshot.members,vec!["a","b"]);assert_eq!(snapshot.verified.len(),2);}
 #[test]fn versioned_snapshot_keeps_excluded_candidates_separate_from_verified_paths(){let snapshot=read_snapshot(&json!({"version":1,"verified":[entry("a"),entry("b")],"members":["a","b","changed"]}).to_string()).unwrap();assert_eq!(snapshot.members,vec!["a","b","changed"]);assert_eq!(snapshot.verified.len(),2);}
 #[test]fn corrupt_or_future_snapshots_are_rejected(){
  for raw in [json!({"version":2,"verified":[entry("a")],"members":["a"]}),json!({"version":1,"verified":[entry("a")],"members":["b"]}),json!({"version":1,"verified":[entry("a"),entry("a")],"members":["a"]}),json!({"version":1,"verified":[entry("a")],"members":["a","a"]})]{assert!(read_snapshot(&raw.to_string()).is_err());}
 }
 #[test]fn actual_plan_freezes_all_indexed_members_but_only_returns_matching_files(){
  let root=crate::test_support::temp_dir("smart-photo-duplicate-plan");
  let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE image_paths(path TEXT,image_id TEXT,is_current INTEGER,missing_at_unix INTEGER);CREATE TABLE duplicate_plans(id TEXT,image_id TEXT,sha256 TEXT,paths TEXT,created_at_unix INTEGER);CREATE TABLE duplicate_decisions(fingerprint TEXT PRIMARY KEY,sha256 TEXT,paths TEXT,created_at_unix INTEGER);").unwrap();
  for(name,bytes)in [("a.png",b"same".as_slice()),("b.png",b"same".as_slice()),("changed.png",b"foreign".as_slice())]{let path=root.join(name);fs::write(&path,bytes).unwrap();db.execute("INSERT INTO image_paths VALUES(?1,'image',1,NULL)",[path_to_string(&path)]).unwrap();}
  let sha=compute_file_sha256(&root.join("a.png")).unwrap();let group=plan(&db,"image",&sha).unwrap().unwrap();assert_eq!(group["copies"],2);assert_eq!(group["skipped"].as_array().unwrap().len(),1);
  let raw:String=db.query_row("SELECT paths FROM duplicate_plans",[],|r|r.get(0)).unwrap();let snapshot=read_snapshot(&raw).unwrap();assert_eq!(snapshot.members,paths(&db,"image").unwrap());assert_eq!(snapshot.verified.len(),2);assert_eq!(snapshot.members.len(),3);
  for name in ["a.png","b.png","changed.png"]{fs::remove_file(root.join(name)).unwrap();}fs::remove_dir(root).unwrap();
 }
}
