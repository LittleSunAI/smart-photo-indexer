CREATE TABLE image_tag_layers (
    image_id TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    source TEXT NOT NULL CHECK(source IN ('user','ai','legacy')),
    created_at_unix INTEGER NOT NULL,
    PRIMARY KEY(image_id,tag_id,source)
);
CREATE TABLE image_tag_suppressions (
    image_id TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at_unix INTEGER NOT NULL,
    PRIMARY KEY(image_id,tag_id)
);
INSERT INTO image_tag_layers(image_id,tag_id,source,created_at_unix)
 SELECT it.image_id,it.tag_id,CASE WHEN it.source='ai' THEN 'ai'
   WHEN it.source='user' AND i.manual_tags_set=1 THEN 'user' ELSE 'legacy' END,it.created_at_unix
 FROM image_tags it JOIN images i ON i.id=it.image_id;
