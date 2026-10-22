use crate::*;
use std::collections::HashSet;

// image_tags remains the effective searchable projection. Layers and suppressions retain intent.
pub fn apply(tx:&rusqlite::Transaction<'_>,image_id:&str,source:&str,wanted:&[String],now:u64)->Result<(),String>{
    let wanted:HashSet<String>=wanted.iter().cloned().collect();
    if source=="manual"{
        let old=read_tags_for_image(tx,image_id)?;
        for removed in old.iter().filter(|tag|!wanted.contains(*tag)){
            let tag_id:i64=tx.query_row("SELECT id FROM tags WHERE name=?1",[removed],|r|r.get(0)).map_err(|e|e.to_string())?;
            tx.execute("DELETE FROM image_tag_layers WHERE image_id=?1 AND tag_id=?2 AND source='user'",params![image_id,tag_id]).map_err(|e|e.to_string())?;
            tx.execute("INSERT OR IGNORE INTO image_tag_suppressions(image_id,tag_id,created_at_unix) VALUES(?1,?2,?3)",params![image_id,tag_id,now]).map_err(|e|e.to_string())?;
        }
        let old:HashSet<_>=old.into_iter().collect();
        for added in wanted.iter().filter(|tag|!old.contains(*tag)){add_layer(tx,image_id,added,"user",now)?;}
    }else{
        tx.execute("DELETE FROM image_tag_layers WHERE image_id=?1 AND source='ai'",[image_id]).map_err(|e|e.to_string())?;
        for tag in &wanted{add_layer(tx,image_id,tag,"ai",now)?;}
    }
    rebuild(tx,image_id)
}
pub fn rebuild(tx:&rusqlite::Transaction<'_>,image_id:&str)->Result<(),String>{
    tx.execute("DELETE FROM image_tags WHERE image_id=?1",[image_id]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO image_tags(image_id,tag_id,source,created_at_unix) SELECT l.image_id,l.tag_id,CASE WHEN MAX(l.source='user')=1 THEN 'user' WHEN MAX(l.source='legacy')=1 THEN 'legacy' ELSE 'ai' END,MIN(l.created_at_unix) FROM image_tag_layers l WHERE l.image_id=?1 AND (l.source='user' OR NOT EXISTS(SELECT 1 FROM image_tag_suppressions s WHERE s.image_id=l.image_id AND s.tag_id=l.tag_id)) GROUP BY l.image_id,l.tag_id",[image_id]).map_err(|e|e.to_string())?;
    Ok(())
}
pub(crate) fn add_layer(tx:&rusqlite::Transaction<'_>,image_id:&str,tag:&str,source:&str,now:u64)->Result<(),String>{
    tx.execute("INSERT INTO tags(name,kind,created_at_unix) VALUES(?1,?2,?3) ON CONFLICT(name) DO NOTHING",params![tag,source,now]).map_err(|e|e.to_string())?;
    let id:i64=tx.query_row("SELECT id FROM tags WHERE name=?1",[tag],|r|r.get(0)).map_err(|e|e.to_string())?;
    if source=="user"{tx.execute("DELETE FROM image_tag_suppressions WHERE image_id=?1 AND tag_id=?2",params![image_id,id]).map_err(|e|e.to_string())?;}
    tx.execute("INSERT OR IGNORE INTO image_tag_layers(image_id,tag_id,source,created_at_unix) VALUES(?1,?2,?3,?4)",params![image_id,id,source,now]).map_err(|e|e.to_string())?;Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn manual_removal_survives_ai_refresh_and_unchanged_tags_keep_their_origin(){
        let mut db=Connection::open_in_memory().unwrap();
        for migration in INDEX_MIGRATIONS{db.execute_batch(migration.sql).unwrap();}
        db.execute("INSERT INTO images(id,identity_key,file_name,status,created_at_unix,updated_at_unix) VALUES('photo','photo','photo.png','notProcessed',1,1)",[]).unwrap();
        let tx=db.transaction().unwrap();
        apply(&tx,"photo","ai",&["кошка".into(),"пульт".into()],1).unwrap();
        apply(&tx,"photo","manual",&["пульт".into(),"семья".into()],2).unwrap();
        apply(&tx,"photo","ai",&["кошка".into(),"пульт".into(),"диван".into()],3).unwrap();
        let visible=read_tags_for_image(&tx,"photo").unwrap();
        assert!(!visible.contains(&"кошка".into()));
        assert!(visible.contains(&"семья".into()));
        assert!(visible.contains(&"диван".into()));
        let promoted:i64=tx.query_row("SELECT COUNT(*) FROM image_tag_layers l JOIN tags t ON t.id=l.tag_id WHERE t.name='пульт' AND l.source='user'",[],|r|r.get(0)).unwrap();
        assert_eq!(promoted,0);
        let mut restored=visible;restored.push("кошка".into());
        apply(&tx,"photo","manual",&restored,4).unwrap();
        assert!(read_tags_for_image(&tx,"photo").unwrap().contains(&"кошка".into()));
        tx.commit().unwrap();
    }
}
