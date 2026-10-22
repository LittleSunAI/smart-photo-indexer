use crate::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc,Mutex};
use std::time::{Duration,Instant};

#[derive(Default)]
pub struct SearchSessions{sessions:Mutex<HashMap<String,Arc<Mutex<SearchSession>>>>}
struct SearchSession{db:Connection,input:IndexSearchInputDto,created:Instant,last_read:Instant,path_query:Option<(String,Vec<rusqlite::types::Value>)>}
fn prune(sessions:&mut HashMap<String,Arc<Mutex<SearchSession>>>){
    sessions.retain(|_,session|match session.try_lock(){Ok(session)=>session.last_read.elapsed()<Duration::from_secs(180) && session.created.elapsed()<Duration::from_secs(600),Err(_)=>true});
}
pub fn start_cleanup(app:tauri::AppHandle){
    tauri::async_runtime::spawn(async move{loop{tokio::time::sleep(Duration::from_secs(30)).await;
        let cleanup_app=app.clone();let _=tauri::async_runtime::spawn_blocking(move||{
            let state=cleanup_app.state::<SearchSessions>();if let Ok(mut sessions)=state.sessions.lock(){prune(&mut sessions);};
        }).await;
    }});
}
#[tauri::command]
pub async fn close_search_snapshot(app:tauri::AppHandle,query_id:String)->Result<(),String>{
    tauri::async_runtime::spawn_blocking(move||{app.state::<SearchSessions>().sessions.lock().map_err(|_|"Search session lock failed.")?.remove(&query_id);Ok(())}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn fork_search_snapshot(app:tauri::AppHandle,query_id:String)->Result<String,String>{
    tauri::async_runtime::spawn_blocking(move||fork(&app,query_id)).await.map_err(|e|e.to_string())?
}
fn fork(app:&tauri::AppHandle,query_id:String)->Result<String,String>{
    let state=app.state::<SearchSessions>();let mut sessions=state.sessions.lock().map_err(|_|"Search session lock failed.")?;prune(&mut sessions);
    if sessions.len()>=8{return Err("Finish existing search operations before starting another.".into());}
    let session=sessions.get(&query_id).cloned().ok_or("Search snapshot expired; run the search again.")?;
    let id=stable_text_id("search",&format!("{}|{}",query_id,SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    sessions.insert(id.clone(),session);Ok(id)
}

fn search_fields(value:Option<&[String]>)->Result<Vec<&str>,String>{
    let fields=value.map(|v|v.iter().map(String::as_str).collect()).unwrap_or_else(||vec!["description","tags","fileName","path"]);
    if fields.is_empty()||fields.len()>4||fields.iter().any(|f|!matches!(*f,"description"|"tags"|"fileName"|"path")){return Err("Выберите хотя бы одно допустимое поле поиска.".into());}Ok(fields)
}

pub(super) fn capture_file_selection(app:&tauri::AppHandle,query_id:&str,excluded:&[String],cancel:&std::sync::atomic::AtomicBool)->Result<Vec<String>,String>{
    let id=fork(app,query_id.to_string())?;
    let mut paths=Vec::new();
    let result=materialize_processing_paths(app,&id,excluded,cancel,|batch|{paths.extend_from_slice(batch);Ok(())});
    app.state::<SearchSessions>().sessions.lock().map_err(|_|"Search session lock failed")?.remove(&id);
    result.map(|_|paths)
}
fn field_expression(terms:&[String],expanded:bool,fields:&[&str])->String{
    let mut columns=Vec::new();for field in fields{let column=match *field{"description"=>"description","tags"=>"tags",_=>"paths"};if !columns.contains(&column){columns.push(column);}}
    format!("{{{}}}: ({})",columns.join(" "),lexical_search::expression(terms,expanded))
}

#[tauri::command]
pub async fn search_indexed_images(app_handle:tauri::AppHandle,input:IndexSearchInputDto)->Result<IndexSearchResultDto,String>{
    tauri::async_runtime::spawn_blocking(move||search(&app_handle,input)).await.map_err(|e|e.to_string())?
}

fn search(app:&tauri::AppHandle,input:IndexSearchInputDto)->Result<IndexSearchResultDto,String>{
    if input.query_id.is_none(){
        search_fields(input.fields.as_deref())?;
        if input.query.len()>4096 || input.query.split(|c:char|!c.is_alphanumeric()).filter(|s|!s.is_empty()).count()>32{return Err("Search query is too long.".into());}
        let empty=input.query.trim().is_empty();
        if !empty && !input.query.chars().any(|c|c.is_alphanumeric()){return Err("Search phrase has no searchable words.".into());}
        if input.tags.as_ref().is_some_and(|tags|tags.len()>30){return Err("Too many tag filters.".into());}
        let local_date_filter=input.date_from_local.as_ref().is_some_and(|s|!s.is_empty()) || input.date_to_local.as_ref().is_some_and(|s|!s.is_empty());
        if local_date_filter && input.date_field.as_deref()!=Some("taken"){return Err("Calendar camera dates require the capture-date field.".into());}
        if input.date_field.as_deref()==Some("taken") && (input.date_from_unix.is_some() || input.date_to_unix.is_some()){return Err("Capture dates require calendar bounds, not inferred UTC dates.".into());}
        if input.date_from_unix.zip(input.date_to_unix).is_some_and(|(from,to)|from>to){return Err("The start date must precede the end date.".into());}
        if input.date_from_local.as_ref().zip(input.date_to_local.as_ref()).is_some_and(|(from,to)|!from.is_empty() && !to.is_empty() && from>=to){return Err("The capture-date end bound must follow the start bound.".into());}
        if empty && normalize_search_tags(input.tags.as_deref().unwrap_or(&[])).is_empty() && input.statuses.iter().all(|s|s.trim().is_empty() || s.trim()=="all") && !input.favorite_only.unwrap_or(false) && !input.untagged_only.unwrap_or(false) && input.date_from_unix.is_none() && input.date_to_unix.is_none() && !local_date_filter && input.collection_id.as_ref().is_none_or(|s|s.is_empty()) && input.root_path.as_ref().is_none_or(|s|s.is_empty()){return Err("Choose a search phrase or a filter.".into());}
        if input.statuses.len()>16 || input.statuses.iter().any(|s|!matches!(s.trim(),""|"all"|"processed"|"queued"|"notProcessed"|"error"|"relocated"|"missing")){return Err("Unsupported status filter.".into());}
        if input.collection_id.as_ref().is_some_and(|id|id.len()>256){return Err("Invalid collection id.".into());}
        if input.root_path.as_ref().is_some_and(|root|!root.is_empty() && (root.len()>32768 || !Path::new(root).is_absolute())){return Err("Выберите абсолютный путь корневой папки.".into());}
        if !matches!(input.sort_order.as_deref().unwrap_or("relevance"),"readyFirst"|"readyLast"|"errorsFirst"|"errorsLast"|"relevance"|"nameAsc"|"nameDesc"|"analyzedAsc"|"analyzedDesc"|"modifiedAsc"|"modifiedDesc"|"takenAsc"|"takenDesc"|"indexedAsc"|"indexedDesc"|"sizeAsc"|"sizeDesc"|"dimensionsAsc"|"dimensionsDesc"|"typeAsc"|"typeDesc"|"statusAsc"|"statusDesc"|"cameraAsc"|"cameraDesc"|"pathAsc"|"pathDesc"|"tagsAsc"|"tagsDesc"|"favoriteAsc"|"favoriteDesc"){return Err("Unsupported search order.".into());}
        if !matches!(input.date_field.as_deref().unwrap_or("modified"),"modified"|"indexed"|"taken"|"analyzed"){return Err("Unsupported date field.".into());}
        for value in [input.date_from_local.as_ref(),input.date_to_local.as_ref()].into_iter().flatten().filter(|s|!s.is_empty()){
            chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").map_err(|_|"Invalid capture date filter.")?;
        }
    }
    let offset=input.offset.unwrap_or(0);let limit=input.limit.unwrap_or(200).clamp(1,500);
    let state=app.state::<SearchSessions>();
    let (query_id,session,new_snapshot)={
        let mut sessions=state.sessions.lock().map_err(|_|"Search session lock failed.")?;prune(&mut sessions);
        if let Some(id)=input.query_id.as_ref(){(id.clone(),sessions.get(id).cloned().ok_or("Search snapshot expired; run the search again.")?,false)}else{
            if sessions.len()>=8{return Err("Too many retained searches. Close earlier search operations.".into());}
            let mut db=open_index_connection(app)?;run_index_migrations(&mut db)?;
            db.execute_batch("BEGIN DEFERRED;").map_err(|e|e.to_string())?;
            let now=Instant::now();let session=Arc::new(Mutex::new(SearchSession{db,input:input.clone(),created:now,last_read:now,path_query:None}));
            let id=stable_text_id("search",&SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos().to_string());
            sessions.insert(id.clone(),session.clone());(id,session,true)
        }
    };
    let result=(||{
    let mut session=session.lock().map_err(|_|"Search snapshot lock failed.")?;session.last_read=Instant::now();
    let input=session.input.clone();
    let query=input.query.trim().to_string();
    if query.len()>4096{return Err("Search query is too long.".into());}
    let requested_tags=normalize_search_tags(input.tags.as_deref().unwrap_or(&[]));
    if requested_tags.len()>30{return Err("Too many tag filters.".into());}
    let tag_mode=if input.tag_mode.as_deref()==Some("all"){"all"}else{"any"}.to_string();
    let statuses:Vec<String>=input.statuses.iter().map(|v|v.trim().to_string()).filter(|v|!v.is_empty() && v!="all").collect();
    let favorite=input.favorite_only.unwrap_or(false);
    let terms:Vec<String>=query.split(|c:char|!c.is_alphanumeric()).filter(|s|!s.is_empty()).take(32).map(|s|s.to_lowercase()).collect();
    if !query.is_empty() && terms.is_empty(){return Err("Search phrase has no searchable words.".into());}
    let fields=search_fields(input.fields.as_deref())?;
    let expression=field_expression(&terms,input.expand_terms.unwrap_or(false),&fields);
    let from=if terms.is_empty(){"images i INNER JOIN image_paths p ON p.image_id=i.id"}else{"image_search INNER JOIN images i ON i.id=image_search.image_id INNER JOIN image_paths p ON p.image_id=i.id"};
    let rank=if terms.is_empty(){"0.0"}else{"bm25(image_search,0.0,1.0,1.0,4.0,6.0)"};
    let mut clauses=vec!["p.is_current=1".to_string()];
    let mut bindings:Vec<rusqlite::types::Value>=Vec::new();
    if !terms.is_empty(){clauses.push("image_search MATCH ?".into());bindings.push(expression.into());}
    if !terms.is_empty()&&(fields.contains(&"fileName")||fields.contains(&"path")){
        // FTS paths aggregate copies of one content identity. Constrain filename/path
        // matches to the returned physical copy instead of matching every sibling alias.
        let mut alternatives=Vec::new();let content:Vec<&str>=fields.iter().copied().filter(|f|matches!(*f,"description"|"tags")).collect();
        if !content.is_empty(){alternatives.push("image_search.rowid IN (SELECT rowid FROM image_search WHERE image_search MATCH ?)".to_string());bindings.push(field_expression(&terms,input.expand_terms.unwrap_or(false),&content).into());}
        for (field,column) in [("fileName","photo_file_name(p.path)"),("path","p.path")]{if fields.contains(&field){let mut matches=Vec::new();for term in &terms{matches.push(format!("instr(unicode_lower({column}),?)>0"));bindings.push(term.clone().into());}alternatives.push(format!("({})",matches.join(" OR ")));}}
        clauses.push(format!("({})",alternatives.join(" OR ")));
    }
    if !statuses.is_empty(){clauses.push("i.status IN (SELECT value FROM json_each(?))".into());bindings.push(json!(statuses).to_string().into());}
    if favorite{clauses.push("i.is_favorite=1".into());}
    if input.untagged_only.unwrap_or(false){clauses.push("NOT EXISTS(SELECT 1 FROM image_tags it WHERE it.image_id=i.id)".into());}
    if let Some(id)=input.collection_id.as_ref().filter(|id|!id.is_empty()){
        if id.len()>256{return Err("Invalid collection id.".into());}
        clauses.push("EXISTS(SELECT 1 FROM collection_images ci WHERE ci.image_id=i.id AND ci.collection_id=?)".into());bindings.push(id.clone().into());
    }
    if let Some(root)=input.root_path.as_ref().filter(|root|!root.is_empty()){
        if root.len()>32768 || !Path::new(root).is_absolute(){return Err("Выберите абсолютный путь корневой папки.".into());}
        let root=root.replace('/',"\\").trim_end_matches('\\').to_lowercase();
        clauses.push("(unicode_lower(replace(p.path,'/','\\'))=? OR instr(unicode_lower(replace(p.path,'/','\\')),?||'\\')=1)".into());
        bindings.push(root.clone().into());bindings.push(root.into());
    }
    let date_field=input.date_field.as_deref().unwrap_or("modified");
    if date_field=="taken"{
        for (value,comparison) in [(input.date_from_local.as_ref(),">="),(input.date_to_local.as_ref(),"<")]{
            if let Some(value)=value.filter(|s|!s.is_empty()){
                let date=chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").map_err(|_|"Invalid capture date filter.")?;
                clauses.push(format!("i.date_taken_local{comparison}?"));bindings.push(format!("{}T00:00:00",date.format("%Y-%m-%d")).into());
            }
        }
    }else{
        let date_column=match date_field{"modified"=>"COALESCE(p.modified_at_unix,i.modified_at_unix)","indexed"=>"i.created_at_unix","analyzed"=>"i.ai_processed_at_unix",_=>return Err("Unsupported date field.".into())};
        if let Some(from)=input.date_from_unix{clauses.push(format!("{date_column}>=?"));bindings.push(u64_to_i64_saturating(from).into());}
        if let Some(to)=input.date_to_unix{let comparison=if input.date_to_exclusive.unwrap_or(false){"<"}else{"<="};clauses.push(format!("{date_column}{comparison}?"));bindings.push(u64_to_i64_saturating(to).into());}
    }
    if !requested_tags.is_empty(){
        let matched="SELECT COUNT(DISTINCT unicode_lower(t.name)) FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id AND unicode_lower(t.name) IN (SELECT value FROM json_each(?))";
        clauses.push(if tag_mode=="all"{format!("({matched})=?")}else{format!("({matched})>0")});bindings.push(json!(requested_tags).to_string().into());
        if tag_mode=="all"{bindings.push((requested_tags.len() as i64).into());}
    }
    let where_sql=clauses.join(" AND ");
    let order=search_order_sql(input.sort_order.as_deref().unwrap_or("relevance"))?;
    session.path_query=Some((format!("SELECT p.path,{rank} AS rank FROM {from} WHERE {where_sql} ORDER BY {order},photo_sort_key(photo_file_name(p.path)) ASC,photo_sort_key(p.path) ASC,p.id ASC"),bindings.clone()));
    let tx=&session.db;
    let total:i64=tx.query_row(&format!("SELECT COUNT(*) FROM {from} WHERE {where_sql}"),rusqlite::params_from_iter(&bindings),|r|r.get(0)).map_err(|e|format!("Full-text count failed: {e}"))?;
    let status_counts=tx.query_row(&format!("SELECT COALESCE(SUM(i.status='processed'),0),COALESCE(SUM(i.status='queued'),0),COALESCE(SUM(i.status IN ('notProcessed','missing','relocated')),0),COALESCE(SUM(i.status='error'),0) FROM {from} WHERE {where_sql}"),rusqlite::params_from_iter(&bindings),|r|Ok(json!({"processed":r.get::<_,i64>(0)?,"inQueue":r.get::<_,i64>(1)?,"notProcessed":r.get::<_,i64>(2)?,"errors":r.get::<_,i64>(3)?}))).map_err(|e|e.to_string())?;
    let sql=format!("SELECT i.id,p.path,i.extension,COALESCE(p.size_bytes,i.size_bytes),COALESCE(p.modified_at_unix,i.modified_at_unix),i.status,i.description,i.is_favorite,p.first_seen_at_unix,p.last_seen_at_unix,i.description_version,i.tags_version,{rank} AS rank,i.ai_processed_at_unix FROM {from} WHERE {where_sql} ORDER BY {order},photo_sort_key(photo_file_name(p.path)) ASC,photo_sort_key(p.path) ASC,p.id ASC LIMIT ? OFFSET ?");
    bindings.push(i64::from(limit).into());bindings.push(i64::from(offset).into());
    let mut items=Vec::new();
    {
        let mut stmt=tx.prepare(&sql).map_err(|e|format!("Full-text page failed: {e}"))?;
        let rows=stmt.query_map(rusqlite::params_from_iter(&bindings),|r|{
            let path:String=r.get(1)?;let rank:f64=r.get(12)?;
            Ok(IndexSearchResultItemDto{ai_processed_at_unix:r.get(13)?,description_format:None,file_metadata:None,image_id:r.get(0)?,file_name:file_name(Path::new(&path)),path,extension:r.get(2)?,size_bytes:optional_i64_to_u64(r.get(3)?),modified_at_unix:optional_i64_to_u64(r.get(4)?),status:r.get(5)?,description:r.get(6)?,is_favorite:r.get(7)?,first_seen_at_unix:optional_i64_to_u64(r.get(8)?),last_seen_at_unix:optional_i64_to_u64(r.get(9)?),description_version:r.get(10)?,tags_version:r.get(11)?,tags:vec![],collections:vec![],score:(-rank*1_000_000.0).round().max(1.0) as i64,matched_fields:if terms.is_empty(){vec!["filters".into()]}else{vec!["fullText".into()]}})
        }).map_err(|e|e.to_string())?;
        for row in rows{items.push(row.map_err(|e|e.to_string())?);}
    }
    // Only the requested page needs metadata hydration, not the entire archive.
    for item in &mut items{item.tags=read_tags_for_image_from_connection(&tx,&item.image_id)?;item.collections=read_collection_names_for_image_from_connection(&tx,&item.image_id)?;item.file_metadata=image_metadata::stored(tx,&item.image_id)?;item.description_format=description_format::read(tx,&item.image_id)?;}
    let now=unix_now();
    let writer=open_index_connection(app)?;
    if !query.is_empty() && new_snapshot{writer.execute("INSERT INTO search_history(query,result_count,created_at_unix) VALUES(?1,?2,?3)",params![query,total,now]).map_err(|e|e.to_string())?;}
    let recent_searches=read_recent_searches(&writer)?;
    let next=offset.saturating_add(items.len() as u32);let has_more=(next as usize)<total as usize;
    Ok(IndexSearchResultDto{status_counts,query_id:query_id.clone(),has_more,next_cursor:if has_more{Some(next)}else{None},query,total_count:total as usize,returned_count:items.len(),items,recent_searches,applied_statuses:statuses,applied_tags:requested_tags,applied_tag_mode:tag_mode,applied_date_from_unix:input.date_from_unix,applied_date_to_unix:input.date_to_unix,applied_favorite_only:favorite,searched_at_unix:now})
    })();
    // A failed initial query never hands its ID to the caller. Release only that
    // new handle; a failed page must preserve the already applied read snapshot.
    if result.is_err()&&new_snapshot {if let Ok(mut sessions)=state.sessions.lock(){sessions.remove(&query_id);}}
    result
}

#[tauri::command]
pub async fn export_search_snapshot(app:tauri::AppHandle,mut input:IndexSearchInputDto,format:String,selected_paths:Option<Vec<String>>,excluded_paths:Option<Vec<String>>,target_path:Option<String>, ui_locale:Option<String>)->Result<serde_json::Value,String>{
    if format!="csv" && format!="json"{return Err("Unsupported export format.".into());}
    let id=fork_search_snapshot(app.clone(),input.query_id.clone().ok_or("Run the search before exporting.")?).await?;
    input.query_id=Some(id.clone());
    let target=match target_path{
        Some(path)=>Some(PathBuf::from(path)),
        None=>rfd::AsyncFileDialog::new().set_file_name(format!("smart-photo-search.{}",format)).add_filter(crate::ui_locale_generated::caption(ui_locale.as_deref(),"native.e978b00de465a271"),&[&format]).save_file().await.map(|file|file.path().to_path_buf()),
    };
    let result=if let Some(target)=target{
        let worker_app=app.clone();
        tauri::async_runtime::spawn_blocking(move||export_file(&worker_app,input,&format,selected_paths,excluded_paths,target)).await.map_err(|e|e.to_string()).and_then(|result|result)
    }else{Ok(json!({"cancelled":true}))};
    close_search_snapshot(app,id).await?;result
}
fn csv_cell(value:&str)->String{
    let stripped=value.trim_start_matches(|c:char|c.is_whitespace() || c.is_control());
    let value=if stripped.starts_with(['=','+','-','@']){format!("'{value}")}else{value.into()};
    format!("\"{}\"",value.replace('"',"\"\""))
}
fn export_file(app:&tauri::AppHandle,mut input:IndexSearchInputDto,format:&str,selected_paths:Option<Vec<String>>,excluded_paths:Option<Vec<String>>,target:PathBuf)->Result<serde_json::Value,String>{
    use std::io::Write;
    let selected=selected_paths.map(|paths|paths.into_iter().map(|path|path.to_lowercase()).collect::<std::collections::HashSet<_>>());
    let excluded=excluded_paths.unwrap_or_default().into_iter().map(|path|path.to_lowercase()).collect::<std::collections::HashSet<_>>();
    if excluded.len()>5000{return Err("За одну операцию допускается не более 5000 исключённых путей.".into());}
    if selected.as_ref().is_some_and(|paths|paths.is_empty() || paths.len()>5000){return Err("Choose 1 to 5000 source copies for a selected export.".into());}
    let mut count=0usize;let mut total=0usize;input.offset=Some(0);input.limit=Some(32);
    file_safety::atomic_save(&target,|file,_|{
        let mut output=std::io::BufWriter::new(file);
        if format=="csv"{output.write_all(b"\xef\xbb\xbfimage_id,file_name,path,status,extension,size_bytes,modified_at_unix,tags,description\r\n").map_err(|e|e.to_string())?;}
        else{output.write_all(b"{\"items\":[").map_err(|e|e.to_string())?;}
        loop{
            let page=search(app,input.clone())?;total=page.total_count;
            for item in page.items{
                if selected.as_ref().is_some_and(|paths|!paths.contains(&item.path.to_lowercase())){continue;}
                if excluded.contains(&item.path.to_lowercase()){continue;}
                if format=="json"{
                    if count>0{output.write_all(b",").map_err(|e|e.to_string())?;}
                    serde_json::to_writer(&mut output,&item).map_err(|e|e.to_string())?;
                }else{
                    let fields=vec![item.image_id,item.file_name,item.path,item.status,item.extension.unwrap_or_default(),item.size_bytes.map(|v|v.to_string()).unwrap_or_default(),item.modified_at_unix.map(|v|v.to_string()).unwrap_or_default(),item.tags.join("; "),item.description.unwrap_or_default()];
                    let line=fields.iter().map(|value|csv_cell(value)).collect::<Vec<_>>().join(",")+"\r\n";
                    output.write_all(line.as_bytes()).map_err(|e|e.to_string())?;
                }
                count+=1;
            }
            if let Some(next)=page.next_cursor{input.offset=Some(next);}else{break;}
        }
        if format=="json"{
            write!(output,"],\"resultCount\":{total},\"exportedCount\":{count},\"query\":").map_err(|e|e.to_string())?;
            serde_json::to_writer(&mut output,&input.query).map_err(|e|e.to_string())?;
            output.write_all(b"}\n").map_err(|e|e.to_string())?;
        }
        output.flush().map_err(|e|e.to_string())?;Ok(())
    })?;
    Ok(json!({"cancelled":false,"fileName":file_name(&target),"path":path_to_string(&target),"exportedCount":count,"resultCount":total,"format":format}))
}

#[tauri::command]
pub async fn mutate_search_collection(app:tauri::AppHandle,mut input:IndexSearchInputDto,collection_id:String,excluded_paths:Vec<String>,remove:bool)->Result<CollectionMutationResultDto,String>{
    if collection_id.is_empty() || collection_id.len()>256 || excluded_paths.len()>5000{return Err("Недопустимая коллекция или слишком много исключённых путей.".into());}
    let id=fork_search_snapshot(app.clone(),input.query_id.clone().ok_or("Сначала выполните поиск.")?).await?;
    input.query_id=Some(id.clone());
    let worker_app=app.clone();
    let result=tauri::async_runtime::spawn_blocking(move||{
        let excluded:std::collections::HashSet<_>=excluded_paths.into_iter().map(|path|path.to_lowercase()).collect();
        let mut db=open_index_connection(&worker_app)?;run_index_migrations(&mut db)?;
        // SQLite owns the full selection; Rust and the WebView retain one bounded page.
        db.execute_batch("PRAGMA temp_store=FILE; CREATE TEMP TABLE selected_images(id TEXT PRIMARY KEY); CREATE TEMP TABLE changed_images(id TEXT PRIMARY KEY);").map_err(|e|e.to_string())?;
        input.offset=Some(0);input.limit=Some(128);
        loop {
            let page=search(&worker_app,input.clone())?;
            for item in page.items {
                if !excluded.contains(&item.path.to_lowercase()) {db.execute("INSERT OR IGNORE INTO temp.selected_images(id) VALUES(?1)",[item.image_id]).map_err(|e|e.to_string())?;}
            }
            if let Some(next)=page.next_cursor{input.offset=Some(next);}else{break;}
        }
        let requested:usize=db.query_row("SELECT COUNT(*) FROM temp.selected_images",[],|row|row.get(0)).map_err(|e|e.to_string())?;
        if requested==0{return Err("В снимке поиска не осталось выбранных фотографий.".into());}
        let tx=db.transaction().map_err(|e|e.to_string())?;
        read_collection_by_id(&tx,&collection_id)?.ok_or("Коллекция не найдена.")?;
        let condition=if remove {"EXISTS"} else {"NOT EXISTS"};
        tx.execute(&format!("INSERT INTO temp.changed_images SELECT s.id FROM temp.selected_images s JOIN images i ON i.id=s.id WHERE {condition}(SELECT 1 FROM collection_images ci WHERE ci.collection_id=?1 AND ci.image_id=s.id)"),[&collection_id]).map_err(|e|e.to_string())?;
        let now=unix_now();
        let changed=if remove {
            tx.execute("DELETE FROM collection_images WHERE collection_id=?1 AND image_id IN (SELECT id FROM temp.changed_images)",[&collection_id]).map_err(|e|e.to_string())?
        } else {
            tx.execute("INSERT INTO collection_images(collection_id,image_id,added_at_unix) SELECT ?1,id,?2 FROM temp.changed_images",params![collection_id,now]).map_err(|e|e.to_string())?
        };
        tx.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) SELECT ?1,id,?2,?3,?4,?5 FROM temp.changed_images",params![if remove{"collection_removed"}else{"collection_added"},if remove{Some(collection_id.as_str())}else{None},if remove{None}else{Some(collection_id.as_str())},"Выбор полного снимка поиска",now]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE collections SET updated_at_unix=?1 WHERE id=?2",params![now,collection_id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(CollectionMutationResultDto{collection:read_collection_by_id(&db,&collection_id)?,requested_images:requested,changed_images:changed,skipped_images:requested-changed,action_log_entries:changed,updated_at_unix:now})
    }).await.map_err(|e|e.to_string()).and_then(|result|result);
    close_search_snapshot(app,id).await?;result
}

#[tauri::command]
pub async fn get_search_selection_image_ids(app:tauri::AppHandle,mut input:IndexSearchInputDto,excluded_paths:Vec<String>,maximum:usize)->Result<Vec<String>,String>{
    if !(1..=256).contains(&maximum) || excluded_paths.len()>5000{return Err("Допустимо до 256 разных фотографий и 5000 исключённых путей.".into());}
    let id=fork_search_snapshot(app.clone(),input.query_id.clone().ok_or("Сначала выполните поиск.")?).await?;
    input.query_id=Some(id.clone());let worker_app=app.clone();
    let result=tauri::async_runtime::spawn_blocking(move||{
        let excluded:std::collections::HashSet<_>=excluded_paths.into_iter().map(|path|path.to_lowercase()).collect();
        let mut ids=std::collections::BTreeSet::new();input.offset=Some(0);input.limit=Some(128);
        loop {
            let page=search(&worker_app,input.clone())?;
            for item in page.items {
                if !excluded.contains(&item.path.to_lowercase()){ids.insert(item.image_id);}
                if ids.len()>maximum{return Err(format!("Для этой операции выберите не более {maximum} разных фотографий. Выбор не был обрезан, изменения не внесены."));}
            }
            if let Some(next)=page.next_cursor{input.offset=Some(next);}else{break;}
        }
        Ok(ids.into_iter().collect())
    }).await.map_err(|e|e.to_string()).and_then(|result|result);
    close_search_snapshot(app,id).await?;result
}

pub(super) fn path_context(app:&tauri::AppHandle,query_id:&str)->Result<Vec<String>,String>{
    let session=app.state::<SearchSessions>().sessions.lock().map_err(|_|"Search session lock failed.")?
        .get(query_id).cloned().ok_or("Search snapshot expired; run the search again.")?;
    let mut session=session.lock().map_err(|_|"Search snapshot lock failed.")?;session.last_read=Instant::now();
    let (sql,bindings)=session.path_query.clone().ok_or("Выполните поиск перед открытием просмотрщика.")?;
    let mut statement=session.db.prepare(&sql).map_err(|error|error.to_string())?;
    let rows=statement.query_map(rusqlite::params_from_iter(&bindings),|row|row.get::<_,String>(0)).map_err(|error|error.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|error|error.to_string())
}
/// Hold the applied read snapshot only for bounded path materialization, never hashing/inference.
pub(super) fn materialize_processing_paths(app:&tauri::AppHandle,query_id:&str,excluded:&[String],cancel:&std::sync::atomic::AtomicBool,mut consume:impl FnMut(&[String])->Result<(),String>)->Result<(),String>{
 let session={let state=app.state::<SearchSessions>();let mut sessions=state.sessions.lock().map_err(|_|"Search session lock failed")?;prune(&mut sessions);sessions.get(query_id).cloned().ok_or("Выдача поиска устарела. Выполните поиск снова.")?};
 let mut session=session.lock().map_err(|_|"Search snapshot lock failed")?;session.last_read=Instant::now();
 let (sql,bindings)=session.path_query.clone().ok_or("Выполните поиск перед подготовкой задания.")?;
 let excluded:std::collections::HashSet<String>=excluded.iter().map(|p|p.to_lowercase()).collect();let mut query=session.db.prepare(&sql).map_err(|e|e.to_string())?;
 let mut rows=query.query(rusqlite::params_from_iter(&bindings)).map_err(|e|e.to_string())?;let mut batch=Vec::with_capacity(128);
 while let Some(row)=rows.next().map_err(|e|e.to_string())?{if cancel.load(std::sync::atomic::Ordering::SeqCst){return Err("Подготовка отменена.".into());}let path:String=row.get(0).map_err(|e|e.to_string())?;if !excluded.contains(&path.to_lowercase()){batch.push(path);if batch.len()==128{consume(&batch)?;batch.clear();}}}
 if !batch.is_empty(){consume(&batch)?;}Ok(())
}
pub(super) fn selection_statistics(app:&tauri::AppHandle,query_id:&str,excluded:&[String],cancel:&std::sync::atomic::AtomicBool)->Result<selection_statistics::Statistics,String>{
    let session={let state=app.state::<SearchSessions>();let sessions=state.sessions.lock().map_err(|_|"Search session lock failed")?;sessions.get(query_id).cloned().ok_or("Выдача поиска устарела. Выполните поиск снова.")?};
    let mut session=session.lock().map_err(|_|"Search snapshot lock failed")?;session.last_read=Instant::now();
    let (sql,bindings)=session.path_query.clone().ok_or("Выполните поиск перед подсчётом.")?;
    let excluded:std::collections::HashSet<String>=excluded.iter().map(|p|p.to_lowercase()).collect();
    let sql=format!("WITH selected AS ({sql}) SELECT p.path,COALESCE(p.size_bytes,i.size_bytes) FROM selected s JOIN image_paths p ON p.path=s.path AND p.is_current=1 JOIN images i ON i.id=p.image_id");
    let mut q=session.db.prepare(&sql).map_err(|e|e.to_string())?;let mut rows=q.query(rusqlite::params_from_iter(&bindings)).map_err(|e|e.to_string())?;let mut stats=selection_statistics::Statistics::default();
    while let Some(row)=rows.next().map_err(|e|e.to_string())?{if cancel.load(std::sync::atomic::Ordering::SeqCst){return Err("Selection changed".into());}let path:String=row.get(0).map_err(|e|e.to_string())?;if !excluded.contains(&path.to_lowercase()){let size:Option<i64>=row.get(1).map_err(|e|e.to_string())?;stats.add(Path::new(&path),size.and_then(|s|u64::try_from(s).ok()));}}
    Ok(stats)
}
pub(super) fn limited_path_context(app:&tauri::AppHandle,query_id:&str,maximum:usize)->Result<Vec<String>,String>{
    let session=app.state::<SearchSessions>().sessions.lock().map_err(|_|"Search session lock failed.")?
        .get(query_id).cloned().ok_or("Выделение поиска устарело; выполните поиск снова.")?;
    let mut session=session.lock().map_err(|_|"Search snapshot lock failed.")?;session.last_read=Instant::now();
    let (sql,mut bindings)=session.path_query.clone().ok_or("Выполните поиск перед переименованием.")?;
    bindings.push(rusqlite::types::Value::Integer(maximum as i64));
    let mut statement=session.db.prepare(&format!("SELECT * FROM ({sql}) LIMIT ?")).map_err(|e|e.to_string())?;
    let rows=statement.query_map(rusqlite::params_from_iter(&bindings),|row|row.get::<_,String>(0)).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

fn search_order_sql(order:&str)->Result<&'static str,String>{
    Ok(match order{
        "relevance"=>"rank ASC", "nameAsc"=>"photo_sort_key(photo_file_name(p.path)) ASC", "nameDesc"=>"photo_sort_key(photo_file_name(p.path)) DESC",
        "modifiedAsc"=>"COALESCE(p.modified_at_unix,i.modified_at_unix) IS NULL,COALESCE(p.modified_at_unix,i.modified_at_unix) ASC", "modifiedDesc"=>"COALESCE(p.modified_at_unix,i.modified_at_unix) IS NULL,COALESCE(p.modified_at_unix,i.modified_at_unix) DESC",
        "takenAsc"=>"i.date_taken_local IS NULL,i.date_taken_local ASC", "takenDesc"=>"i.date_taken_local IS NULL,i.date_taken_local DESC",
        "analyzedAsc"=>"i.ai_processed_at_unix IS NULL,i.ai_processed_at_unix ASC", "analyzedDesc"=>"i.ai_processed_at_unix IS NULL,i.ai_processed_at_unix DESC",
        "indexedAsc"=>"i.created_at_unix ASC", "indexedDesc"=>"i.created_at_unix DESC",
        "sizeAsc"=>"COALESCE(p.size_bytes,i.size_bytes) IS NULL,COALESCE(p.size_bytes,i.size_bytes) ASC", "sizeDesc"=>"COALESCE(p.size_bytes,i.size_bytes) IS NULL,COALESCE(p.size_bytes,i.size_bytes) DESC",
        "dimensionsAsc"=>"i.width IS NULL OR i.height IS NULL,(i.width*i.height) ASC", "dimensionsDesc"=>"i.width IS NULL OR i.height IS NULL,(i.width*i.height) DESC",
        "typeAsc"=>"i.extension IS NULL,unicode_lower(i.extension) ASC", "typeDesc"=>"i.extension IS NULL,unicode_lower(i.extension) DESC",
        "readyFirst"=>"CASE WHEN i.status='processed' THEN 0 ELSE 1 END ASC", "readyLast"=>"CASE WHEN i.status='processed' THEN 1 ELSE 0 END ASC",
        "errorsFirst"=>"CASE WHEN i.status='error' THEN 0 ELSE 1 END ASC", "errorsLast"=>"CASE WHEN i.status='error' THEN 1 ELSE 0 END ASC",
        "statusAsc"=>"CASE i.status WHEN 'processed' THEN 1 WHEN 'queued' THEN 2 WHEN 'notProcessed' THEN 3 WHEN 'relocated' THEN 4 WHEN 'missing' THEN 5 ELSE 6 END ASC", "statusDesc"=>"CASE i.status WHEN 'processed' THEN 1 WHEN 'queued' THEN 2 WHEN 'notProcessed' THEN 3 WHEN 'relocated' THEN 4 WHEN 'missing' THEN 5 ELSE 6 END DESC",
        "cameraAsc"=>"COALESCE(json_extract(i.file_metadata,'$.cameraMake'),'') || COALESCE(json_extract(i.file_metadata,'$.cameraModel'),'') ASC", "cameraDesc"=>"COALESCE(json_extract(i.file_metadata,'$.cameraMake'),'') || COALESCE(json_extract(i.file_metadata,'$.cameraModel'),'') DESC",
        "pathAsc"=>"unicode_lower(p.path) ASC", "pathDesc"=>"unicode_lower(p.path) DESC",
        "tagsAsc"=>"(SELECT COUNT(*) FROM image_tags st WHERE st.image_id=i.id) ASC", "tagsDesc"=>"(SELECT COUNT(*) FROM image_tags st WHERE st.image_id=i.id) DESC",
        "favoriteAsc"=>"i.is_favorite ASC", "favoriteDesc"=>"i.is_favorite DESC",
        _=>return Err("Unsupported search order.".into())
    })
}


#[cfg(test)]
mod tests{
    #[test]fn status_group_orders_are_distinct_and_stable(){
        let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE images(name TEXT,status TEXT);INSERT INTO images VALUES('a','queued'),('b','processed'),('c','error'),('d','notProcessed'),('e','processed'),('f','missing'),('g','relocated');").unwrap();
        for (mode,expected) in [("readyFirst","beacdfg"),("readyLast","acdfgbe"),("errorsFirst","cabdefg"),("errorsLast","abdefgc")]{
            let mut q=db.prepare(&format!("SELECT name FROM images i ORDER BY {},name ASC",search_order_sql(mode).unwrap())).unwrap();let actual=q.query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap().join("");assert_eq!(actual,expected, "{mode}");
        }
        assert_ne!(search_order_sql("statusAsc").unwrap(),search_order_sql("readyFirst").unwrap());assert!(search_order_sql("invalid").is_err());
    }
    #[test]
    fn chosen_columns_do_not_match_unselected_text(){
        let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE VIRTUAL TABLE image_search USING fts5(image_id UNINDEXED,file_name,paths,description,tags);INSERT INTO image_search VALUES('1','needle.png','D:/needle.png','forest','river'),('2','other.png','D:/other.png','needle','lake'),('3','third.png','D:/third.png','house','needle');").unwrap();
        for (fields,expected) in [(vec!["description"],1),(vec!["tags"],1),(vec!["description","tags"],2),(vec!["path"],1),(vec!["fileName"],1),(vec!["description","tags","path","fileName"],3)]{
            let expression=field_expression(&["needle".into()],false,&fields);let count:i64=db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH ?1",[expression],|r|r.get(0)).unwrap();assert_eq!(count,expected,"{fields:?}");
        }
        assert!(search_fields(Some(&[])).is_err());assert!(search_fields(Some(&["description) OR 1=1".into()])).is_err());assert_eq!(search_fields(None).unwrap().len(),4);
    }
    use super::*;
    #[test]
    fn csv_guards_formulas_even_after_whitespace_and_preserves_quotes_and_lines(){
        assert_eq!(csv_cell("\t =1+2"),"\"'\t =1+2\"");
        assert_eq!(csv_cell("@SUM(A1)"),"\"'@SUM(A1)\"");
        assert_eq!(csv_cell("Фото \"кот\"\nдетали"),"\"Фото \"\"кот\"\"\nдетали\"");
        assert_eq!(csv_cell("кот + собака"),"\"кот + собака\"");
    }
    #[test]
    fn fts_retains_full_unicode_description_and_tracks_effective_updates(){
        let db=Connection::open_in_memory().unwrap();
        for migration in INDEX_MIGRATIONS.iter().take(9){db.execute_batch(migration.sql).unwrap();}
        let description=format!("{} На заднем плане оранжевый карабин рядом с рюкзаком.","Подробное описание окружающей обстановки. ".repeat(700));
        db.execute("INSERT INTO images(id,identity_key,file_name,description,status,created_at_unix,updated_at_unix) VALUES('a','a','фото.png',?1,'processed',1,1)",[&description]).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'карабин'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        db.execute("UPDATE images SET description='Исправлено: обезьяна на дереве' WHERE id='a'",[]).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'карабин'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search WHERE image_search MATCH 'ОБЕЗЬЯНА'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        db.execute("DELETE FROM images WHERE id='a'",[]).unwrap();
        assert_eq!(db.query_row("SELECT COUNT(*) FROM image_search",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
