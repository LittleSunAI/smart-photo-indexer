DROP TRIGGER fts_images_update;
CREATE TRIGGER fts_images_update AFTER UPDATE OF description,file_name ON images
WHEN old.description IS NOT new.description OR old.file_name IS NOT new.file_name
BEGIN
    DELETE FROM image_search WHERE rowid = new.rowid;
    INSERT INTO image_search(rowid,image_id,file_name,paths,description,tags)
    SELECT i.rowid,i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.id;
END;
