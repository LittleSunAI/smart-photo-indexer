use crate::*;

#[tauri::command]
pub async fn rename_collection(app:tauri::AppHandle,collection_id:String,name:String)->Result<CollectionMutationResultDto,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let name=name.trim();if name.is_empty() || name.chars().count()>200{return Err("Название коллекции должно содержать от 1 до 200 символов.".into());}
        let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;
        let tx=db.transaction().map_err(|e|e.to_string())?;
        let previous=read_collection_by_id(&tx,&collection_id)?.ok_or("Коллекция не найдена.")?;
        let conflict:i64=tx.query_row("SELECT COUNT(*) FROM collections WHERE unicode_lower(name)=unicode_lower(?1) AND id<>?2",params![name,collection_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if conflict>0{return Err("Коллекция с таким названием уже существует.".into());}
        let now=unix_now();
        tx.execute("UPDATE collections SET name=?1,updated_at_unix=?2 WHERE id=?3",params![name,now,collection_id]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO action_logs(action_type,old_value,new_value,message,created_at_unix) VALUES('collection_renamed',?1,?2,?3,?4)",params![previous.name,name,collection_id,now]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(CollectionMutationResultDto{collection:read_collection_by_id(&db,&collection_id)?,requested_images:0,changed_images:0,skipped_images:0,action_log_entries:1,updated_at_unix:now})
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn delete_collection(app:tauri::AppHandle,collection_id:String)->Result<CollectionMutationResultDto,String>{
    tauri::async_runtime::spawn_blocking(move||{
        let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let tx=db.transaction().map_err(|e|e.to_string())?;
        let previous=read_collection_by_id(&tx,&collection_id)?.ok_or("Коллекция не найдена.")?;let now=unix_now();
        // Membership is removed explicitly; photographs, paths and metadata remain intact.
        tx.execute("DELETE FROM collection_images WHERE collection_id=?1",[&collection_id]).map_err(|e|e.to_string())?;
        tx.execute("DELETE FROM collections WHERE id=?1",[&collection_id]).map_err(|e|e.to_string())?;
        let before=serde_json::to_string(&previous).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO action_logs(action_type,old_value,message,created_at_unix) VALUES('collection_deleted',?1,?2,?3)",params![before,collection_id,now]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(CollectionMutationResultDto{collection:None,requested_images:0,changed_images:previous.image_count,skipped_images:0,action_log_entries:1,updated_at_unix:now})
    }).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn remove_images_from_collection(app:tauri::AppHandle,input:AddImagesToCollectionInputDto)->Result<CollectionMutationResultDto,String>{
    tauri::async_runtime::spawn_blocking(move||{
        if input.image_ids.is_empty() || input.image_ids.len()>5000{return Err("Выберите от 1 до 5000 фотографий.".into());}
        let mut db=open_index_connection(&app)?;run_index_migrations(&mut db)?;let tx=db.transaction().map_err(|e|e.to_string())?;
        read_collection_by_id(&tx,&input.collection_id)?.ok_or("Коллекция не найдена.")?;let now=unix_now();let mut changed=0;
        let ids:std::collections::HashSet<_>=input.image_ids.iter().collect();
        for id in ids{
            if tx.execute("DELETE FROM collection_images WHERE collection_id=?1 AND image_id=?2",params![input.collection_id,id]).map_err(|e|e.to_string())?>0{
                changed+=1;
                tx.execute("INSERT INTO action_logs(action_type,image_id,old_value,message,created_at_unix) VALUES('collection_removed',?1,?2,?3,?4)",params![id,input.collection_id,"Удалено из коллекции",now]).map_err(|e|e.to_string())?;
            }
        }
        tx.execute("UPDATE collections SET updated_at_unix=?1 WHERE id=?2",params![now,input.collection_id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(CollectionMutationResultDto{collection:read_collection_by_id(&db,&input.collection_id)?,requested_images:input.image_ids.len(),changed_images:changed,skipped_images:input.image_ids.len()-changed,action_log_entries:changed,updated_at_unix:now})
    }).await.map_err(|e|e.to_string())?
}
