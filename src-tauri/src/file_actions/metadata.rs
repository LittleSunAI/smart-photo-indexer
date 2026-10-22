use super::{io,types::*};
use crate::*;
use std::collections::HashSet;

pub fn check_recognition_busy(db:&Connection, paths:&[String], leased:&HashSet<String>) -> Result<(),String> {
    for path in paths {
        let mut q=db.prepare("SELECT t.id,t.status,j.status,EXISTS(SELECT 1 FROM processing_attempt_context a WHERE a.task_id=t.id AND a.status IN('reserved','running','remotePending','unknown','responseSaved') AND (COALESCE(json_extract(a.effective_config,'$.providerMode'),'')<>'local' OR (a.status IN('reserved','running','responseSaved') AND a.run_revision=t.run_revision AND a.generation=j.generation AND j.status NOT IN('stopped','completed')))) FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.source_path=?1 COLLATE NOCASE").map_err(|e|e.to_string())?;
        let rows=q.query_map([path],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,bool>(3)?))).map_err(|e|e.to_string())?;
        for row in rows { let (id,status,job,uncertain)=row.map_err(|e|e.to_string())?;
            if leased.contains(&id)||uncertain||["preparing","running","committing"].contains(&status.as_str())||
                (["pending","waitingRetry"].contains(&status.as_str())&&["running","queued","preparing"].contains(&job.as_str())) {
                return Err(format!("busy:Фотография сейчас используется распознаванием: {}. Попробуйте после завершения этого запроса.",file_name(Path::new(path))));
            }
        }
        let preparing:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_sources s JOIN processing_commands c ON c.id=s.command_id WHERE s.path=?1 COLLATE NOCASE AND c.status='preparing')",[path],|r|r.get(0)).map_err(|e|e.to_string())?;
        if preparing{return Err("busy:Для этой фотографии ещё готовится обработка. Попробуйте после завершения подготовки.".into());}
    }
    Ok(())
}
pub fn paused_tasks(db:&Connection,path:&str) -> Result<Vec<String>,String> {
    let mut q=db.prepare("SELECT t.id FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.source_path=?1 COLLATE NOCASE AND t.status IN('pending','waitingRetry') AND j.status IN('paused','prepared','stopped','interrupted')").map_err(|e|e.to_string())?;
    let result=q.query_map([path],|r|r.get(0)).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string());result
}
pub fn check_rebind(db:&Connection, p:&Payload) -> Result<(),String> {
    if p.kind!=Kind::Move{return Ok(());}
    for id in &p.paused_task_ids {
        let duplicate:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM processing_tasks other JOIN processing_tasks own ON own.job_id=other.job_id WHERE own.id=?1 AND other.id<>own.id AND other.source_path=?2 COLLATE NOCASE)",params![id,p.target_path],|r|r.get(0)).map_err(|e|e.to_string())?;
        if duplicate{return Err("В этой очереди уже есть задача для нового пути. Файл не перемещён.".into());}
    }
    Ok(())
}
pub fn check_recovery_busy(db:&Connection,paths:&[String],own:&str)->Result<(),String>{
    for path in paths{
        let busy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM file_operations WHERE id<>?1 AND status IN('applying','needsRecovery') AND (source_path=?2 COLLATE NOCASE OR target_path=?2 COLLATE NOCASE))",params![own,path],|r|r.get(0)).map_err(|e|e.to_string())?;
        if busy{return Err("busy:Для этого файла ещё сверяется предыдущая файловая операция. Остальные фотографии доступны.".into());}
    }
    Ok(())
}
pub fn check_publications(db:&Connection,paths:&[String])->Result<(),String>{
    for path in paths {
        let busy:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM editor_publications WHERE target_path=?1 COLLATE NOCASE AND state IN('fileSaved','indexPending','replacing','needsRecovery'))",[path],|r|r.get(0)).map_err(|e|e.to_string())?;
        if busy{return Err(format!("busy:Сейчас сохраняется изображение: {}. Дождитесь завершения сохранения.",file_name(Path::new(path))));}
    }
    Ok(())
}
pub fn invalidate_publications(db:&Connection,paths:&[String])->Result<(),String>{
    for path in paths { db.execute("UPDATE editor_publications SET state='cancelled',error='Путь изменён. Подготовьте сохранение заново.' WHERE target_path=?1 COLLATE NOCASE AND state='planned'",[path]).map_err(|e|e.to_string())?; }
    Ok(())
}
pub fn bindings(db:&Connection,path:&str)->Result<Vec<ProjectBinding>,String>{
    let mut q=db.prepare("SELECT id,source_path,source_sha256 FROM editor_projects WHERE source_path=?1 COLLATE NOCASE").map_err(|e|e.to_string())?;
    let result=q.query_map([path],|r|Ok(ProjectBinding{project_id:r.get(0)?,expected_source_path:r.get(1)?,expected_source_sha:r.get(2)?})).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string());result
}
fn retire(db:&Connection,path:&str)->Result<(),String>{
    let owner:Option<String>=db.query_row("SELECT image_id FROM image_paths WHERE path=?1",[path],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    db.execute("UPDATE image_paths SET is_current=0,missing_at_unix=?2,last_seen_at_unix=?2 WHERE path=?1",params![path,unix_now()]).map_err(|e|e.to_string())?;
    if let Some(owner)=owner {
        db.execute("UPDATE images SET status_before_missing=CASE WHEN status<>'missing' THEN status ELSE status_before_missing END,status='missing' WHERE id=?1 AND NOT EXISTS(SELECT 1 FROM image_paths WHERE image_id=?1 AND is_current=1 AND missing_at_unix IS NULL)",[owner]).map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn commit_index(db:&mut Connection,p:&mut Payload)->Result<(),String>{
    let tx=db.transaction().map_err(|e|e.to_string())?;
    if p.kind==Kind::Trash || p.source_removed {
        let original:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM image_paths p JOIN images i ON i.id=p.image_id WHERE p.path=?1 AND i.sha256=?2)",params![p.source_path,p.source_fingerprint.sha256],|r|r.get(0)).map_err(|e|e.to_string())?;
        if original{retire(&tx,&p.source_path)?;}
    }
    if let Some(target)=&p.target_path {
        if !io::matches(Path::new(target),p.published_fingerprint.as_ref().ok_or("Нет отпечатка результата")?){return Err("Сохранённый файл изменился. Обновление индекса отложено для сверки.".into());}
        let sha=&p.source_fingerprint.sha256;
        let owner:Option<String>=tx.query_row("SELECT id FROM images WHERE sha256=?1 ORDER BY created_at_unix,id LIMIT 1",[sha],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let image=owner.unwrap_or_else(||stable_text_id("image",&format!("sha256:{sha}")));
        let info=fs::metadata(target).map_err(|e|e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO images(id,identity_key,file_name,extension,size_bytes,modified_at_unix,sha256,status,created_at_unix,updated_at_unix) VALUES(?1,?2,?3,?4,?5,?6,?7,'notProcessed',?8,?8)",params![image,format!("sha256:{sha}"),file_name(Path::new(target)),extension(Path::new(target)),u64_to_i64_saturating(info.len()),info.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs() as i64),sha,unix_now()]).map_err(|e|e.to_string())?;
        let old:Option<String>=tx.query_row("SELECT image_id FROM image_paths WHERE path=?1",[target],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        // Retire the previous target before changing the unique path's content owner.
        if old.as_ref().is_some_and(|id|id!=&image){retire(&tx,target)?;}
        tx.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix,size_bytes,modified_at_unix) VALUES(?1,?2,1,?3,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET image_id=excluded.image_id,is_current=1,missing_at_unix=NULL,last_seen_at_unix=excluded.last_seen_at_unix,size_bytes=excluded.size_bytes,modified_at_unix=excluded.modified_at_unix",params![image,target,unix_now(),u64_to_i64_saturating(info.len()),info.modified().ok().and_then(|t|t.duration_since(UNIX_EPOCH).ok()).map(|d|d.as_secs() as i64)]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE images SET status=COALESCE(status_before_missing,'notProcessed') WHERE id=?1 AND status='missing'",[&image]).map_err(|e|e.to_string())?;
        if !p.source_removed && io::matches(Path::new(&p.source_path),&p.source_fingerprint){
            tx.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix,size_bytes,modified_at_unix) VALUES(?1,?2,1,?3,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET image_id=excluded.image_id,is_current=1,missing_at_unix=NULL,last_seen_at_unix=excluded.last_seen_at_unix",params![image,p.source_path,unix_now(),u64_to_i64_saturating(p.source_fingerprint.size_bytes),p.source_fingerprint.modified_ns.parse::<u128>().ok().map(|v|(v/1_000_000_000)as i64)]).map_err(|e|e.to_string())?;
        }
        image_metadata::save(&tx,&image,target,&image_metadata::read(Path::new(target)))?;
        p.image_id=Some(image);
    }
    for id in &p.paused_task_ids {
        if p.kind==Kind::Move && p.source_removed {
            let target=p.target_path.as_deref().ok_or("Нет нового пути")?;
            tx.execute("UPDATE processing_tasks SET source_path=?2,snapshot=CASE WHEN json_type(snapshot,'$.sourceSnapshot')='object' THEN json_set(snapshot,'$.targetPath',?2,'$.fileName',?3,'$.sourceSnapshot.path',?2,'$.sourceSnapshot.name',?3) ELSE json_set(snapshot,'$.targetPath',?2,'$.fileName',?3) END WHERE id=?1 AND source_path=?4 COLLATE NOCASE AND status IN('pending','waitingRetry')",params![id,target,file_name(Path::new(target)),p.source_path]).map_err(|e|e.to_string())?;
        } else if p.kind==Kind::Trash {
            tx.execute("UPDATE processing_tasks SET status='failed',error='Фотография отправлена в корзину. Выберите доступный файл для новой обработки.' WHERE id=?1 AND source_path=?2 COLLATE NOCASE AND status IN('pending','waitingRetry')",params![id,p.source_path]).map_err(|e|e.to_string())?;
        }
    }
    // A replace target may own a different paused task. Never let it process the new content as the old one.
    if p.target_before.is_some(){if let Some(target)=&p.target_path {
        tx.execute("UPDATE processing_tasks SET status='failed',error='Файл назначения заменён другим содержимым.' WHERE source_path=?1 COLLATE NOCASE AND sha256<>?2 AND status IN('pending','waitingRetry')",params![target,p.source_fingerprint.sha256]).map_err(|e|e.to_string())?;
    }}
    let marker=format!("file-action:{}",p.id());
    tx.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) SELECT ?1,?2,?3,?4,?5,?6 WHERE NOT EXISTS(SELECT 1 FROM action_logs WHERE message=?5)",params![p.kind.journal(),p.image_id,p.source_path,p.target_path,marker,unix_now()]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())?;
    p.index_applied=true;io::phase(db,p,"indexCommitted")
}
pub fn commit_editor(db:&mut Connection,p:&mut Payload)->Result<(),String>{
    if p.kind==Kind::Move && p.source_removed {
        let target=p.target_path.as_deref().ok_or("Нет нового пути")?;
        let tx=db.transaction().map_err(|e|e.to_string())?;
        for binding in &p.project_bindings {
            let current:(String,String)=tx.query_row("SELECT source_path,source_sha256 FROM editor_projects WHERE id=?1",[&binding.project_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
            if current.0==target&&current.1==binding.expected_source_sha{continue;}
            if current.0!=binding.expected_source_path||current.1!=binding.expected_source_sha{return Err("Связь проекта изменилась; новый путь сохранён для сверки.".into());}
            tx.execute("UPDATE editor_projects SET source_path=?2,source_name=?3 WHERE id=?1",params![binding.project_id,target,file_name(Path::new(target))]).map_err(|e|e.to_string())?;
        }
        tx.commit().map_err(|e|e.to_string())?;
    }
    p.editor_applied=true;Ok(())
}
