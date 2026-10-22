use super::{io,metadata,recycle,types::*};
use crate::*;
use crate::guarded_file::GuardedFile;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Decision { pub choice:Choice, pub expected:Option<Fingerprint> }
pub enum Step { Conflict(Conflict), Done(Outcome) }

fn affected(p:&Payload)->Vec<String>{
    let mut paths=Vec::new();
    if p.kind!=Kind::Copy{paths.push(p.source_path.clone());}
    if let Some(target)=&p.target_path{paths.push(target.clone());}
    paths
}
pub fn database(app:&tauri::AppHandle)->Result<Connection,String>{
    let mut db=open_index_connection(app)?;run_index_migrations(&mut db)?;Ok(db)
}
fn guarded<T>(app:&tauri::AppHandle,p:&Payload,run:impl FnOnce()->Result<T,String>)->Result<T,String>{
    processing::file_action_fence(app,&affected(p),||{
        let _files=file_operations::OPERATIONS_LOCK.lock().map_err(|_|"File operation lock failed")?;
        let _index=INDEX_SYNC_LOCK.lock().map_err(|_|"Index lock failed")?;
        if index_backups::restore_blocked(app)?{return Err("Сначала завершите восстановление библиотеки.".into());}
        metadata::check_recovery_busy(&database(app)?,&affected(p),&p.id())?;
        let editor=photo_editor::store::connection(app)?;
        metadata::check_publications(&editor,&affected(p))?;
        run()
    })
}
fn finish_metadata(app:&tauri::AppHandle,db:&mut Connection,p:&mut Payload)->Result<(),String>{
    if !p.index_applied{metadata::commit_index(db,p)?;}
    if !p.editor_applied{
        let actual=path_to_string(&photo_editor::store::root(app)?.join("editor.sqlite"));
        if p.editor_database.as_ref().is_some_and(|expected|expected!=&actual){return Err("Хранилище проектов сменилось. Файлы сохранены; связи требуют сверки.".into());}
        metadata::commit_editor(&mut photo_editor::store::connection(app)?,p)?;
        io::phase(db,p,"editorCommitted")?;
    }
    Ok(())
}
pub fn one(app:&tauri::AppHandle,batch:&str,ordinal:usize,kind:Kind,file:&CapturedFile,
    destination:Option<&str>,decision:Option<&Decision>,sources:&HashSet<String>)->Result<Step,String>{
    // Check existing leases before opening the strict source handle. New leases
    // for this current batch path are held off by processing::admit.
    let mut admission_paths=vec![file.path.clone()];
    if let Some(folder)=destination{admission_paths.push(path_to_string(&Path::new(folder).join(file_name(Path::new(&file.path)))));}
    processing::file_action_fence(app,&admission_paths,||Ok(()))?;
    let mut source=GuardedFile::open(Path::new(&file.path),kind!=Kind::Copy)?;
    let fingerprint=io::fingerprint(&mut source)?;
    if fingerprint.identity!=file.identity||fingerprint.size_bytes!=file.size_bytes||fingerprint.modified_ns!=file.modified_ns {
        return Err("Исходная фотография изменилась. Выберите её заново.".into());
    }
    let mut target=if kind==Kind::Trash{None}else{Some(path_to_string(&Path::new(destination.ok_or("Нет папки назначения")?).join(file_name(Path::new(&file.path)))))};
    let mut before=None;
    if let Some(path)=target.clone(){
        let same=io::identity_at(Path::new(&path)).ok().is_some_and(|id|id==file.identity);
        if kind==Kind::Move&&same{return Ok(Step::Done(Outcome::refused(ordinal,&file.path,"skipped","Фотография уже находится в этой папке.".into())));}
        // Release the DELETE-sharing guard before reading a hardlink to the same file.
        drop(source);
        if let Some(existing)=io::at(Path::new(&path))?{
            let protected=sources.contains(&existing.identity);
            let fresh=decision.and_then(|d|d.expected.as_ref()).is_some_and(|expected|expected!=&existing);
            let choice=decision.filter(|_|!fresh).map(|d|d.choice);
            match choice {
                Some(Choice::Skip)=>return Ok(Step::Done(Outcome::refused(ordinal,&file.path,"skipped","Конфликт пропущен.".into()))),
                Some(Choice::KeepBoth)=>{
                    target=Some(io::next_free(Path::new(&path))?);
                }
                Some(Choice::Replace) if !protected=>before=Some(existing),
                _=>return Ok(Step::Conflict(Conflict{
                    conflict_id:stable_text_id("file-conflict",&format!("{batch}|{ordinal}|{}|{}|{}",existing.identity,existing.sha256,existing.modified_ns)),
                    ordinal,source:io::info(&file.path,&fingerprint),target:io::info(&path,&existing),target_fingerprint:existing,
                    requires_fresh_decision:fresh,replace_allowed:!protected,
                })),
            }
        }
    } else {drop(source);}
    let mut db=database(app)?;
    let image_id:Option<String>=db.query_row("SELECT p.image_id FROM image_paths p JOIN images i ON i.id=p.image_id WHERE p.path=?1 AND i.sha256=?2",params![file.path,fingerprint.sha256],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let mut p=Payload{version:1,batch_id:batch.into(),ordinal,kind,phase:"prepared".into(),source_path:file.path.clone(),target_path:target,
        source_fingerprint:fingerprint,target_before:before,stage_path:None,stage_identity:None,backup_path:None,published_fingerprint:None,
        source_removed:false,editor_database:None,project_bindings:Vec::new(),paused_task_ids:Vec::new(),trash_receipt:None,image_id,
        filesystem_applied:false,index_applied:false,editor_applied:false,outcome:None};
    let initial=p.clone();
    guarded(app,&initial,||{
        let editor=photo_editor::store::connection(app)?;
        if kind==Kind::Move{
            p.editor_database=Some(path_to_string(&photo_editor::store::root(app)?.join("editor.sqlite")));
            p.project_bindings=metadata::bindings(&editor,&file.path)?;
        }
        if kind!=Kind::Copy{p.paused_task_ids=metadata::paused_tasks(&db,&file.path)?;}
        metadata::check_rebind(&db,&p)?;io::persist(&db,&p)?;
        db.execute("UPDATE file_operations SET status='applying',error=NULL WHERE id=?1",[p.id()]).map_err(|e|e.to_string())?;Ok(())
    })?;
    let result=(||{
        if kind==Kind::Trash{
            let check=p.clone();
            guarded(app,&check,||{
                let editor=photo_editor::store::connection(app)?;
                metadata::invalidate_publications(&editor,&affected(&p))?;
                io::phase(&db,&mut p,"trashIntent")?;
                p.trash_receipt=Some(recycle::recycle(&p.source_path,&p.source_fingerprint)?);
                p.filesystem_applied=true;p.source_removed=true;
                io::phase(&db,&mut p,"recycledReceipt")?;
                finish_metadata(app,&mut db,&mut p)
            })?;
        }else{
            let mut staged=Some(io::stage(&db,&mut p)?);
            let keep_both=decision.is_some_and(|d|d.choice==Choice::KeepBoth)&&p.target_before.is_none();
            let base=Path::new(destination.ok_or("Нет папки назначения")?).join(file_name(Path::new(&file.path)));
            let mut retries=0;
            loop{
                let check=p.clone();
                let published=guarded(app,&check,||{
                    metadata::check_rebind(&db,&p)?;
                    metadata::invalidate_publications(&photo_editor::store::connection(app)?,&affected(&p))?;
                    io::publish(&db,&mut p,staged.as_mut().ok_or("Нет подготовленной копии")?)?;
                    let removal=io::finish_move(&db,&mut p,staged.take().ok_or("Нет подготовленной копии")?);
                    // Publication is real even if cross-volume source removal failed.
                    finish_metadata(app,&mut db,&mut p)?;
                    removal
                });
                if published.is_err()&&keep_both&&!p.filesystem_applied&&retries<64&&io::at(Path::new(p.target_path.as_ref().ok_or("Нет назначения")?))?.is_some(){
                    // A competing process claimed the index after preview. Reuse the
                    // verified stage and re-enter the full fence for the next free name.
                    p.target_path=Some(io::next_free(&base)?);io::persist(&db,&p)?;retries+=1;continue;
                }
                published?;break;
            }
        }
        Ok::<(),String>(())
    })();
    if result.is_err()&&!p.filesystem_applied&&p.backup_path.is_none(){
        if let (Some(path),Some(previous))=(&p.target_path,&p.target_before){
            if let Some(current)=io::at(Path::new(path))?{
                if &current!=previous&&io::cleanup(&p).is_empty(){
                    db.execute("UPDATE file_operations SET status='failed',error='Назначение изменилось; требуется новое решение.' WHERE id=?1",[p.id()]).map_err(|e|e.to_string())?;
                    return Ok(Step::Conflict(Conflict{
                        conflict_id:stable_text_id("file-conflict",&format!("{batch}|{ordinal}|{}|{}|{}",current.identity,current.sha256,current.modified_ns)),
                        ordinal,source:io::info(&p.source_path,&p.source_fingerprint),target:io::info(path,&current),
                        replace_allowed:!sources.contains(&current.identity),target_fingerprint:current,requires_fresh_decision:true,
                    }));
                }
            }
        }
    }
    let status=if !p.filesystem_applied{"failed"}else if kind==Kind::Move&&!p.source_removed{"partial"}
        else{match kind{Kind::Copy=>"copied",Kind::Move=>"moved",Kind::Trash=>"trashed"}};
    let message=result.err().map(|e|if status=="partial"{format!("Копия создана, но исходник не удалось убрать. Оба файла сохранены. {e}")}
        else if p.filesystem_applied{format!("Файл сохранён. Обновление сведений ещё не завершено: {e}")}else{e});
    let mut outcome=Outcome{ordinal,old_path:p.source_path.clone(),new_path:p.target_path.clone(),image_id:p.image_id.clone(),status:status.into(),
        message,filesystem_applied:p.filesystem_applied,index_applied:p.index_applied,editor_applied:p.editor_applied};
    p.outcome=Some(outcome.clone());io::persist(&db,&p)?;
    let state=if p.filesystem_applied&&p.index_applied&&p.editor_applied{"committed"}
        else if p.filesystem_applied||p.backup_path.is_some()||p.kind==Kind::Trash&&!io::matches(Path::new(&p.source_path),&p.source_fingerprint){"needsRecovery"}else{"failed"};
    db.execute("UPDATE file_operations SET status=?2,error=?3 WHERE id=?1",params![p.id(),state,outcome.message]).map_err(|e|e.to_string())?;
    if state=="committed" || (state=="failed"&&p.backup_path.is_none()){
        let warnings=io::cleanup(&p);
        if !warnings.is_empty(){
            let warning=format!("Временные файлы не удалось полностью убрать: {}",warnings.join("; "));
            outcome.message=Some(outcome.message.map(|message|format!("{message} {warning}")).unwrap_or(warning));
            p.outcome=Some(outcome.clone());io::persist(&db,&p)?;
            db.execute("UPDATE file_operations SET error=?2 WHERE id=?1",params![p.id(),outcome.message]).map_err(|e|e.to_string())?;
        }
    }
    if p.filesystem_applied { notify_paths(app,&p); }
    Ok(Step::Done(outcome))
}
fn notify_paths(app:&tauri::AppHandle,p:&Payload){
    if p.kind==Kind::Move&&p.source_removed{
        if let Some(target)=&p.target_path{let _=viewer_sessions::rename_path(app,&p.source_path,target);}
    }else if p.kind==Kind::Trash{let _=viewer_sessions::remove_path(app,&p.source_path);}
    if let Some(target)=&p.target_path{let _=app.asset_protocol_scope().allow_file(target);let _=viewer_sessions::refresh_contents(app,target);}
}

pub fn recover_startup(app:&tauri::AppHandle)->Result<(),String>{
    let mut db=database(app)?;
    let rows:Vec<(String,String)>={let mut q=db.prepare("SELECT id,payload FROM file_operations WHERE kind IN('explorerCopy','explorerMove','explorerTrash') AND status IN('planned','applying','needsRecovery') AND payload IS NOT NULL").map_err(|e|e.to_string())?;
        let result=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())?;result};
    for(id,raw)in rows{
        let result=(||{
            let mut p:Payload=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
            if p.version!=1{return Err("Неизвестная версия файлового журнала".into());}
            if !io::inspect_recovery(&mut p)?{
                db.execute("UPDATE file_operations SET status='failed',error='Прервано до публикации. Исходник сохранён.' WHERE id=?1",[&id]).map_err(|e|e.to_string())?;
                return Ok(());
            }
            // No new publish/source deletion/POST is allowed on this path.
            finish_metadata(app,&mut db,&mut p)?;
            p.outcome=Some(Outcome{ordinal:p.ordinal,old_path:p.source_path.clone(),new_path:p.target_path.clone(),image_id:p.image_id.clone(),
                status:if p.kind==Kind::Move&&!p.source_removed{"partial"}else{match p.kind{Kind::Copy=>"copied",Kind::Move=>"moved",Kind::Trash=>"trashed"}}.into(),
                message:if p.kind==Kind::Move&&!p.source_removed{Some("Копия создана, исходник сохранён после перезапуска.".into())}else{None},
                filesystem_applied:true,index_applied:p.index_applied,editor_applied:p.editor_applied});
            io::persist(&db,&p)?;db.execute("UPDATE file_operations SET status='committed',error=NULL WHERE id=?1",[&id]).map_err(|e|e.to_string())?;
            let _=io::cleanup(&p);Ok::<(),String>(())
        })();
        if let Err(error)=result{db.execute("UPDATE file_operations SET status='needsRecovery',error=?2 WHERE id=?1",params![id,error]).map_err(|e|e.to_string())?;}
    }
    Ok(())
}
