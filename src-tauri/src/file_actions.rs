//! One native file-action owner shared by menu, keyboard and Alt gestures.
use crate::*;
use std::collections::HashSet;
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use tauri::Emitter;
mod types;
mod io;
pub(crate) mod metadata;
pub(crate) mod recycle;
mod engine;
use types::*;

#[derive(Default)]
pub struct FileActionsRuntime {
    state:Mutex<State>, pub ready:AtomicBool,
}
#[derive(Default)]
struct State {
    revision:u64, capture_cancel:Option<Arc<AtomicBool>>, clipboard:Option<Clipboard>, batch:Option<Batch>,
}
struct Batch {
    id:String, kind:Kind, files:Arc<Vec<CapturedFile>>, destination:Option<String>,
    source_identities:Arc<HashSet<String>>, counts:Counts, warnings:Vec<String>,
    clipboard_revision:Option<u64>, next:usize, revision:u64, state:String,
    conflict:Option<Conflict>, decision:Option<engine::Decision>, all_choice:Option<Choice>,
    outcomes:Vec<Outcome>, cancel:bool,
}
impl Batch {
    fn view(&self,offset:usize,limit:usize)->OperationView {
        let end=offset.saturating_add(limit).min(self.outcomes.len());
        OperationView{operation_id:self.id.clone(),revision:self.revision,state:if self.state=="completed"&&!self.warnings.is_empty(){"needsRecovery".into()}else{self.state.clone()},counts:self.counts.clone(),
            total:self.files.len(),processed:self.outcomes.len(),conflict:self.conflict.clone(),
            changed_paths:if offset<end{self.outcomes[offset..end].to_vec()}else{Vec::new()},warnings:self.warnings.clone(),
            has_more:end<self.outcomes.len()}
    }
    fn record(&mut self,item:Outcome){
        match item.status.as_str(){"copied"=>self.counts.copied+=1,"moved"=>self.counts.moved+=1,"trashed"=>self.counts.trashed+=1,
            "skipped"=>self.counts.skipped+=1,"partial"=>self.counts.partial+=1,_=>self.counts.failed+=1}
        if item.filesystem_applied&&(!item.index_applied||!item.editor_applied)&&self.warnings.len()<10{
            self.warnings.push(item.message.clone().unwrap_or_else(||"Обновление сведений ещё не завершено.".into()));
        }
        self.outcomes.push(item);
    }
}
fn trim_cut(state:&mut State){
    let Some(batch)=&state.batch else{return;};
    let moved:HashSet<&str>=batch.outcomes.iter().filter(|o|o.status=="moved").map(|o|o.old_path.as_str()).collect();
    if let Some(clip)=state.clipboard.as_mut().filter(|c|Some(c.info.revision)==batch.clipboard_revision&&c.info.kind==Some(Kind::Move)){
        Arc::make_mut(&mut clip.files).retain(|f|!moved.contains(f.path.as_str()));clip.info.count=clip.files.len();clip.info.ready=!clip.files.is_empty();
    }
}
fn nonce(prefix:&str)->String{stable_text_id(prefix,&format!("{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos())).replace(':',"-")}
fn main_only(window:&tauri::WebviewWindow)->Result<(),String>{
    if window.label()!="main"{Err("Файловые команды доступны в основном проводнике.".into())}else{Ok(())}
}
fn batch_reserves_path(batch:&Batch,path:&str)->bool{
    if !["running","awaitingConflict"].contains(&batch.state.as_str()){return false;}
    let Some(file)=batch.files.get(batch.next)else{return false;};
    let key=|value:&str|value.replace('/',"\\").to_lowercase();
    key(&file.path)==key(path)||batch.destination.as_ref().is_some_and(|folder|key(&path_to_string(&Path::new(folder).join(file_name(Path::new(&file.path)))))==key(path))
}
/// Called under the coordinator fence. No reverse state -> coordinator lock exists.
pub(crate) fn reserves_path(app:&tauri::AppHandle,path:&str)->Result<bool,String>{
    let runtime=app.state::<FileActionsRuntime>();let state=runtime.state.lock().map_err(|_|"File action lock failed")?;
    Ok(state.batch.as_ref().is_some_and(|batch|batch_reserves_path(batch,path)))
}
fn emit(app:&tauri::AppHandle,id:Option<&str>,revision:u64){let _=app.emit("smart-photo-indexer:file-actions-changed",serde_json::json!({"operationId":id,"revision":revision}));}
fn selection_info(state:&State)->SelectionInfo{
    state.clipboard.as_ref().map(|b|b.info.clone()).unwrap_or(SelectionInfo{revision:state.revision,..Default::default()})
}
fn capture(app:&tauri::AppHandle,source:&Source,cancel:&AtomicBool)->Result<Vec<CapturedFile>,String>{
    let paths=match source{
        Source::Explicit{paths}=>paths.clone(),
        Source::Query{query_id,excluded_paths}=>search::capture_file_selection(app,query_id,excluded_paths,cancel)?,
        Source::Clipboard{selection_id,revision}=>{
            let runtime=app.state::<FileActionsRuntime>();let state=runtime.state.lock().map_err(|_|"File action lock failed")?;
            return state.clipboard.as_ref().filter(|b|b.info.selection_id.as_deref()==Some(selection_id)&&b.info.revision==*revision)
                .map(|b|b.files.as_ref().clone()).ok_or("Выбранный набор изменился. Повторите команду копирования или вырезания.".into());
        }
    };
    let mut seen=HashSet::new();let mut files=Vec::new();
    for path in paths{
        if cancel.load(Ordering::SeqCst){return Err("Подготовка выбранного набора отменена.".into());}
        if !is_supported_image(Path::new(&path)){continue;}
        // Keep unavailable photos in the immutable set: execution reports each failed file.
        let file=io::capture(&path).unwrap_or(CapturedFile{path:path.clone(),identity:String::new(),size_bytes:0,modified_ns:String::new()});
        if seen.insert(file.path.clone()){files.push(file);}
    }
    if files.is_empty(){return Err("Выберите фотографии для файловой команды.".into());}
    Ok(files)
}

#[tauri::command]
pub async fn explorer_file_selection(app:tauri::AppHandle,window:tauri::WebviewWindow,input:SelectionInput)->Result<SelectionInfo,String>{
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move||{
        let runtime=app.state::<FileActionsRuntime>();
        match input{
            SelectionInput::Inspect=>Ok(selection_info(&*runtime.state.lock().map_err(|_|"File action lock failed")?)),
            SelectionInput::Clear=>{
                let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
                if let Some(cancel)=state.capture_cancel.take(){cancel.store(true,Ordering::SeqCst);}
                state.revision+=1;state.clipboard=None;Ok(selection_info(&state))
            }
            SelectionInput::Capture{kind,source}=>{
                if kind==Kind::Trash||matches!(source,Source::Clipboard{..}){return Err("Неверная команда подготовки набора.".into());}
                let cancel=Arc::new(AtomicBool::new(false));
                let revision={let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
                    if let Some(old)=state.capture_cancel.replace(cancel.clone()){old.store(true,Ordering::SeqCst);}
                    state.revision+=1;state.clipboard=None;state.revision};
                let files=capture(&app,&source,&cancel)?;
                let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
                if state.revision!=revision{return Err("Подготовлен более новый выбор фотографий.".into());}
                let info=SelectionInfo{selection_id:Some(nonce("file-selection")),revision,kind:Some(kind),count:files.len(),ready:true};
                state.clipboard=Some(Clipboard{info:info.clone(),files:Arc::new(files)});state.capture_cancel=None;
                drop(state);emit(&app,None,revision);Ok(info)
            }
        }
    }).await.map_err(|e|e.to_string())?
}

fn stored(app:&tauri::AppHandle,id:&str,offset:usize,limit:usize)->Result<Option<OperationView>,String>{
    let db=engine::database(app)?;
    let mut q=db.prepare("SELECT payload FROM file_operations WHERE kind IN('explorerCopy','explorerMove','explorerTrash') AND json_extract(payload,'$.batchId')=?1 ORDER BY CAST(json_extract(payload,'$.ordinal') AS INTEGER)").map_err(|e|e.to_string())?;
    let rows=q.query_map([id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
    let mut outcomes=Vec::new();let mut total=0;
    for row in rows{let p:Payload=serde_json::from_str(&row.map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;total+=1;
        outcomes.push(p.outcome.unwrap_or(Outcome::refused(p.ordinal,&p.source_path,"failed","Прерванная операция требует сверки. Новый перенос не запускался.".into())));}
    if total==0{return Ok(None);}
    let mut batch=Batch{id:id.into(),kind:Kind::Copy,files:Arc::new(Vec::new()),destination:None,source_identities:Arc::new(HashSet::new()),counts:Counts::default(),warnings:Vec::new(),clipboard_revision:None,next:0,revision:total as u64,state:"completed".into(),conflict:None,decision:None,all_choice:None,outcomes:Vec::new(),cancel:false};
    for outcome in outcomes{batch.record(outcome);}
    let mut view=batch.view(offset,limit);view.total=total;
    if !view.warnings.is_empty(){view.state="needsRecovery".into();}Ok(Some(view))
}
#[tauri::command]
pub async fn explorer_file_operation(app:tauri::AppHandle,window:tauri::WebviewWindow,input:OperationInput)->Result<Option<OperationView>,String>{
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move||command(&app,input)).await.map_err(|e|e.to_string())?
}
fn command(app:&tauri::AppHandle,input:OperationInput)->Result<Option<OperationView>,String>{
    let runtime=app.state::<FileActionsRuntime>();
    match input{
        OperationInput::Inspect{operation_id,offset,limit}=>{
            let state=runtime.state.lock().map_err(|_|"File action lock failed")?;let limit=limit.unwrap_or(128).clamp(1,128);
            if let Some(batch)=state.batch.as_ref().filter(|b|operation_id.as_ref().is_none_or(|id|id==&b.id)){return Ok(Some(batch.view(offset,limit)));}
            drop(state);if let Some(id)=operation_id{stored(app,&id,offset,limit)}else{Ok(None)}
        }
        OperationInput::Start{request_id,kind,source,destination_folder}=>{
            if !runtime.ready.load(Ordering::SeqCst){return Err("Проверяется состояние файлов после запуска. Повторите команду через мгновение.".into());}
            if request_id.len()<8||request_id.len()>100||!request_id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'){return Err("Неверный идентификатор файловой команды.".into());}
            let id=stable_text_id("file-action",&request_id).replace(':',"-");
            {let state=runtime.state.lock().map_err(|_|"File action lock failed")?;
                if let Some(batch)=&state.batch{
                    if batch.id==id{return Ok(Some(batch.view(0,128)));}
                    if ["running","awaitingConflict"].contains(&batch.state.as_str()){return Err("Сейчас выполняется файловая операция. Дождитесь её завершения.".into());}
                }}
            if let Some(old)=stored(app,&id,0,128)?{return Ok(Some(old));}
            let destination=if kind==Kind::Trash{if destination_folder.is_some(){return Err("Для удаления папка назначения не нужна.".into());}None}
                else{Some(io::folder(destination_folder.as_deref().ok_or("Выберите папку назначения")?)?)};
            let clipboard_revision=match source{Source::Clipboard{revision,..}=>Some(revision),_=>None};
            let files=capture(app,&source,&AtomicBool::new(false))?;
            let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
            // A concurrent start may have won during selection materialization.
            if let Some(batch)=&state.batch{
                if batch.id==id{return Ok(Some(batch.view(0,128)));}
                if ["running","awaitingConflict"].contains(&batch.state.as_str()){return Err("Сейчас выполняется файловая операция. Дождитесь её завершения.".into());}
            }
            let source_identities=Arc::new(files.iter().filter(|f|!f.identity.is_empty()).map(|f|f.identity.clone()).collect());
            state.batch=Some(Batch{id:id.clone(),kind,files:Arc::new(files),destination,source_identities,counts:Counts::default(),warnings:Vec::new(),clipboard_revision,next:0,revision:1,state:"running".into(),conflict:None,decision:None,all_choice:None,outcomes:Vec::new(),cancel:false});
            let result=state.batch.as_ref().unwrap().view(0,128);drop(state);spawn_worker(app.clone());Ok(Some(result))
        }
        OperationInput::ResolveConflict{operation_id,conflict_id,choice,apply_to_all}=>{
            let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
            let batch=state.batch.as_mut().filter(|b|b.id==operation_id).ok_or("Операция больше не активна")?;
            let conflict=batch.conflict.as_ref().filter(|c|c.conflict_id==conflict_id).ok_or("Этот конфликт уже изменился")?;
            if batch.state!="awaitingConflict"{return Err("Решение уже принято.".into());}
            if choice==Choice::Replace&&!conflict.replace_allowed{return Err("Файл входит в исходный набор. Выберите «Сохранить оба» или «Пропустить».".into());}
            batch.decision=Some(engine::Decision{choice,expected:Some(conflict.target_fingerprint.clone())});
            if apply_to_all{batch.all_choice=Some(choice);}
            batch.conflict=None;batch.state="running".into();batch.revision+=1;
            let view=batch.view(0,128);drop(state);spawn_worker(app.clone());Ok(Some(view))
        }
        OperationInput::CancelRemaining{operation_id}=>{
            let mut state=runtime.state.lock().map_err(|_|"File action lock failed")?;
            let batch=state.batch.as_mut().filter(|b|b.id==operation_id).ok_or("Операция больше не активна")?;
            batch.cancel=true;batch.revision+=1;
            if batch.state=="awaitingConflict"{batch.state="cancelled".into();batch.conflict=None;}
            let view=batch.view(0,128);let revision=batch.revision;
            if view.state=="cancelled"{trim_cut(&mut state);}
            drop(state);emit(app,Some(&operation_id),revision);Ok(Some(view))
        }
    }
}
fn spawn_worker(app:tauri::AppHandle){tauri::async_runtime::spawn_blocking(move||worker(&app));}
fn worker(app:&tauri::AppHandle){
    loop{
        let runtime=app.state::<FileActionsRuntime>();
        let work={
            let Ok(mut state)=runtime.state.lock()else{return;};
            let Some(batch)=state.batch.as_mut()else{return;};
            if batch.state!="running"{return;}
            if batch.cancel||batch.next>=batch.files.len(){
                batch.state=if batch.cancel{"cancelled"}else{"completed"}.into();batch.revision+=1;
                let id=batch.id.clone();let revision=batch.revision;
                trim_cut(&mut state);
                drop(state);emit(app,Some(&id),revision);return;
            }
            (batch.id.clone(),batch.next,batch.kind,batch.files[batch.next].clone(),batch.destination.clone(),
                batch.decision.take().or_else(||batch.all_choice.map(|choice|engine::Decision{choice,expected:None})),batch.source_identities.clone())
        };
        let (id,ordinal,kind,file,destination,decision,sources)=work;
        let result=engine::one(app,&id,ordinal,kind,&file,destination.as_deref(),decision.as_ref(),&sources);
        let Ok(mut state)=runtime.state.lock()else{return;};let Some(batch)=state.batch.as_mut().filter(|b|b.id==id)else{return;};
        match result{
            Ok(engine::Step::Conflict(conflict))=>{
                if conflict.requires_fresh_decision{batch.all_choice=None;}
                batch.conflict=Some(conflict);batch.state=if batch.cancel{"cancelled"}else{"awaitingConflict"}.into();
            }
            Ok(engine::Step::Done(outcome))=>{batch.record(outcome);batch.next+=1;}
            Err(error)=>{let (status,message)=if let Some(reason)=error.strip_prefix("busy:"){("skipped",reason.to_string())}else{("failed",error)};
                batch.record(Outcome::refused(ordinal,&file.path,status,message));batch.next+=1;}
        }
        batch.revision+=1;let revision=batch.revision;let stop=batch.state!="running";
        if stop&&batch.state=="cancelled"{trim_cut(&mut state);}
        drop(state);emit(app,Some(&id),revision);if stop{return;}
    }
}

/// Reconciliation precedes writer admission, with no hashing/COM on the window thread.
pub fn start(app:tauri::AppHandle){
    tauri::async_runtime::spawn(async move{
        let owner=app.clone();
        let result=tauri::async_runtime::spawn_blocking(move||engine::recover_startup(&owner)).await;
        if let Err(error)=result.as_ref().map_err(|e|e.to_string()).and_then(|r|r.as_ref().map_err(Clone::clone)){
            eprintln!("File-action reconciliation: {error}");
        }
        app.state::<FileActionsRuntime>().ready.store(true,Ordering::SeqCst);
        processing::start_coordinator(&app);photo_editor::jobs::start(&app);library_roots::start(app.clone());
    });
}

#[cfg(test)]
mod tests;
