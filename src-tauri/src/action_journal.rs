use crate::*;
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct JournalRename { image_id:Option<String>,old_path:String,new_path:String }
/** Page the combined journal in SQLite; old offsets must not be truncated by an intermediate 500-row fetch. */
pub(crate) fn read(connection:&Connection,input:ActionJournalInputDto)->Result<ActionJournalResultDto,String>{
 let limit=input.limit.unwrap_or(80).clamp(1,200);let offset=input.offset.unwrap_or(0);let image_id=input.image_id.filter(|v|!v.trim().is_empty());let include_search=input.include_search_history.unwrap_or(true)&&image_id.is_none();
 let scope="(?1 IS NULL OR image_id=?1 OR (action_type='batch_renamed' AND message LIKE 'batch-rename:%' AND EXISTS(SELECT 1 FROM action_logs child WHERE child.action_type='file_renamed' AND child.message=action_logs.message AND child.image_id=?1)) OR (action_type IN ('batch_tags_committed','batch_tags_undone','batch_tags_applied_event','batch_tags_undo_event') AND EXISTS(SELECT 1 FROM json_each(CASE WHEN json_valid(old_value) THEN old_value ELSE '{}' END,'$.images') member WHERE json_extract(member.value,'$.imageId')=?1)))";
 let cte=format!("WITH events AS (SELECT 'action' kind,id,created_at_unix FROM action_logs WHERE {scope} AND NOT(action_type='file_renamed' AND message LIKE 'batch-rename:%' AND EXISTS(SELECT 1 FROM action_logs parent WHERE parent.action_type='batch_renamed' AND parent.message=action_logs.message)) AND (action_type NOT IN ('batch_tags_committed','batch_tags_undone') OR NOT EXISTS(SELECT 1 FROM action_logs event WHERE event.action_type IN('batch_tags_applied_event','batch_tags_undo_event') AND json_extract(event.new_value,'$.planId')=action_logs.id)) UNION ALL SELECT 'search' kind,id,created_at_unix FROM search_history WHERE ?2)");
 let total:i64=connection.query_row(&format!("{cte} SELECT COUNT(*) FROM events"),params![image_id,include_search],|r|r.get(0)).map_err(|e|e.to_string())?;
 let mut statement=connection.prepare(&format!("{cte} SELECT kind,id FROM events ORDER BY created_at_unix DESC,kind DESC,id DESC LIMIT ?3 OFFSET ?4")).map_err(|e|e.to_string())?;
 let ids=statement.query_map(params![image_id,include_search,limit,offset],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
 let mut entries=Vec::new();
 for (kind,id) in ids{if kind=="action"{
  let mut entry=connection.query_row("SELECT al.id,al.action_type,al.image_id,i.file_name,p.path,al.old_value,al.new_value,al.message,al.created_at_unix FROM action_logs al LEFT JOIN images i ON i.id=al.image_id LEFT JOIN image_paths p ON p.rowid=(SELECT rowid FROM image_paths WHERE image_id=al.image_id ORDER BY is_current DESC,last_seen_at_unix DESC,path COLLATE NOCASE ASC,path ASC LIMIT 1) WHERE al.id=?1",[id],action_log_row_to_journal_entry).map_err(|e|e.to_string())?;
  if entry.action_type=="batch_renamed" && entry.message.starts_with("batch-rename:") {
   let mut children=connection.prepare("SELECT image_id,old_value,new_value FROM action_logs WHERE action_type='file_renamed' AND message=?1 AND (?2 IS NULL OR image_id=?2) AND old_value IS NOT NULL AND new_value IS NOT NULL ORDER BY id").map_err(|e|e.to_string())?;
   entry.related_renames=Some(children.query_map(params![entry.message,image_id],|r|Ok(JournalRename{image_id:r.get(0)?,old_path:r.get(1)?,new_path:r.get(2)?})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?);
  }
  entries.push(entry);
 }else{entries.push(connection.query_row("SELECT query,result_count,created_at_unix FROM search_history WHERE id=?1",[id],|r|{let query:String=r.get(0)?;let count:i64=r.get(1)?;Ok(ActionJournalEntryDto{id:format!("search:{id}"),source:"search_history".into(),action_type:"search_performed".into(),image_id:None,image_file_name:None,image_path:None,old_value:None,new_value:Some(serde_json::json!({"query":query,"resultCount":count}).to_string()),message:format!("Поиск «{query}» · найдено {count}"),created_at_unix:r.get(2)?,related_renames:None})}).map_err(|e|e.to_string())?);}}
 Ok(ActionJournalResultDto{total_count:total.max(0) as usize,returned_count:entries.len(),entries,loaded_at_unix:unix_now()})
}

#[cfg(test)]mod tests{use super::*;
#[test]fn rename_groups_use_exact_marker_frozen_paths_and_photo_scope(){
 let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
 for id in ["first","second"]{db.execute("INSERT INTO images(id,identity_key,file_name,status,created_at_unix,updated_at_unix) VALUES(?1,?1,'later-current-name.png','notProcessed',1,1)",[id]).unwrap();}
 for marker in ["batch-rename:one:forward","batch-rename:one:reverse","batch-rename:two:forward"]{
  for id in ["first","second"]{db.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) VALUES('file_renamed',?1,?2,?3,?4,1)",params![id,format!("D:\\before\\{id}.png"),format!("D:\\after\\{id}-renamed.png"),marker]).unwrap();}
  db.execute("INSERT INTO action_logs(action_type,message,created_at_unix) VALUES('batch_renamed',?1,1)",[marker]).unwrap();
 }
 db.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) VALUES('file_renamed','first','D:\\standalone-old.png','D:\\standalone-new.png','batch-rename:incomplete:forward',1)",[]).unwrap();
 let read_all=|scope|read(&db,ActionJournalInputDto{image_id:scope,limit:Some(2),offset:None,include_search_history:Some(false)}).unwrap();
 let all=read_all(None);assert_eq!(all.total_count,4);assert_eq!(all.entries.len(),2);assert_eq!(all.entries[0].action_type,"file_renamed");assert!(all.entries[0].related_renames.is_none());
 let grouped=&all.entries[1];let frozen=grouped.related_renames.as_ref().unwrap();assert_eq!(frozen.len(),2);assert_eq!(frozen[0].old_path,"D:\\before\\first.png");assert_eq!(frozen[0].new_path,"D:\\after\\first-renamed.png");
 let scoped=read_all(Some("second".into()));assert_eq!(scoped.total_count,3);for event in scoped.entries{let members=event.related_renames.unwrap();assert_eq!(members.len(),1);assert_eq!(members[0].image_id.as_deref(),Some("second"));}
 assert_eq!(read_all(Some("absent".into())).total_count,0);
 assert_eq!(db.query_row("SELECT COUNT(*) FROM action_logs",[],|r|r.get::<_,i64>(0)).unwrap(),10,"Grouping must not delete or rewrite stored history");
 let later=read(&db,ActionJournalInputDto{image_id:None,limit:Some(2),offset:Some(2),include_search_history:Some(false)}).unwrap();assert_eq!(later.entries.len(),2);assert!(later.entries.iter().all(|e|e.related_renames.as_ref().unwrap().len()==2));
}
#[test]fn combined_pages_reach_beyond_500_and_do_not_collapse_distinct_events(){
 let db=Connection::open_in_memory().unwrap();for migration in INDEX_MIGRATIONS{db.execute_batch(migration.sql).unwrap();}
 for i in 0..620{db.execute("INSERT INTO action_logs(action_type,message,created_at_unix) VALUES('metadata_updated',?1,10)",[format!("event {i}")]).unwrap();}
 db.execute("INSERT INTO search_history(query,result_count,created_at_unix) VALUES('forest',1,11)",[]).unwrap();
 let read_page=|offset|read(&db,ActionJournalInputDto{image_id:None,limit:Some(80),offset:Some(offset),include_search_history:Some(true)}).unwrap();
 assert_eq!(read_page(0).total_count,621);assert_eq!(read_page(0).entries[0].source,"search_history");assert_eq!(read_page(560).entries.len(),61);
 let ids:std::collections::HashSet<String>=(0..8).flat_map(|i|read_page(i*80).entries.into_iter().map(|v|v.id)).collect();assert_eq!(ids.len(),621);
}}
