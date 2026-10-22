DROP TRIGGER fts_image_paths_update;
CREATE TRIGGER fts_image_paths_update AFTER UPDATE OF image_id,path,is_current ON image_paths
WHEN old.image_id <> new.image_id OR old.path <> new.path OR old.is_current <> new.is_current
BEGIN
    DELETE FROM image_search WHERE rowid IN (SELECT rowid FROM images WHERE id IN(old.image_id,new.image_id));
    INSERT INTO image_search(rowid,image_id,file_name,paths,description,tags)
    SELECT i.rowid,i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id IN(old.image_id,new.image_id);
END;
-- Correct old alias text left behind by reassignments performed before this migration.
DELETE FROM image_search;
INSERT INTO image_search(rowid,image_id,file_name,paths,description,tags)
SELECT i.rowid,i.id,i.file_name,
    COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
    COALESCE(i.description,''),
    COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
FROM images i;
