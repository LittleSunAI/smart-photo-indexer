CREATE VIRTUAL TABLE image_search USING fts5(image_id UNINDEXED,file_name,paths,description,tags,tokenize='unicode61 remove_diacritics 2',prefix='2 3 4');
INSERT INTO image_search(image_id,file_name,paths,description,tags)
SELECT i.id,i.file_name,
    COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
    COALESCE(i.description,''),
    COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
FROM images i;
CREATE TRIGGER fts_images_insert AFTER INSERT ON images BEGIN
    DELETE FROM image_search WHERE image_id = new.id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.id;
END;
CREATE TRIGGER fts_images_update AFTER UPDATE OF description,file_name ON images BEGIN
    DELETE FROM image_search WHERE image_id = new.id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.id;
END;
CREATE TRIGGER fts_images_delete AFTER DELETE ON images BEGIN
    DELETE FROM image_search WHERE image_id = old.id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=old.id;
END;
CREATE TRIGGER fts_image_paths_insert AFTER INSERT ON image_paths BEGIN
    DELETE FROM image_search WHERE image_id = new.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.image_id;
END;
CREATE TRIGGER fts_image_paths_update AFTER UPDATE ON image_paths BEGIN
    DELETE FROM image_search WHERE image_id = new.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.image_id;
END;
CREATE TRIGGER fts_image_paths_delete AFTER DELETE ON image_paths BEGIN
    DELETE FROM image_search WHERE image_id = old.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=old.image_id;
END;
CREATE TRIGGER fts_image_tags_insert AFTER INSERT ON image_tags BEGIN
    DELETE FROM image_search WHERE image_id = new.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.image_id;
END;
CREATE TRIGGER fts_image_tags_update AFTER UPDATE ON image_tags BEGIN
    DELETE FROM image_search WHERE image_id = new.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=new.image_id;
END;
CREATE TRIGGER fts_image_tags_delete AFTER DELETE ON image_tags BEGIN
    DELETE FROM image_search WHERE image_id = old.image_id;
    INSERT INTO image_search(image_id,file_name,paths,description,tags)
    SELECT i.id,i.file_name,
        COALESCE((SELECT group_concat(path,' ') FROM image_paths WHERE image_id=i.id AND is_current=1),''),
        COALESCE(i.description,''),
        COALESCE((SELECT group_concat(t.name,' ') FROM image_tags it JOIN tags t ON t.id=it.tag_id WHERE it.image_id=i.id),'')
    FROM images i WHERE i.id=old.image_id;
END;
