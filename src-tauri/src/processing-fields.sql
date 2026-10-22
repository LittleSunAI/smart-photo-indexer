ALTER TABLE images ADD COLUMN description_content_sha TEXT;
ALTER TABLE images ADD COLUMN tags_content_sha TEXT;
ALTER TABLE images ADD COLUMN tags_analysis_completed INTEGER NOT NULL DEFAULT 0;
-- Conservative legacy adoption: prove the bytes using a committed task, or
-- keep explicit manual values. Failed/cancelled rows are never evidence.
UPDATE images SET description_content_sha=sha256 WHERE sha256 IS NOT NULL AND trim(COALESCE(description,''))<>'' AND (
 manual_description_set=1 OR EXISTS(SELECT 1 FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.image_id=images.id AND t.sha256=images.sha256 AND t.metadata_committed=1 AND COALESCE(json_extract(j.config,'$.outputMode'),'both') IN ('both','description')));
UPDATE images SET tags_content_sha=sha256,tags_analysis_completed=CASE WHEN manual_tags_set=1 THEN 0 ELSE 1 END WHERE sha256 IS NOT NULL AND (
 (manual_tags_set=1 AND EXISTS(SELECT 1 FROM image_tags WHERE image_id=images.id)) OR EXISTS(SELECT 1 FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.image_id=images.id AND t.sha256=images.sha256 AND t.metadata_committed=1 AND COALESCE(json_extract(j.config,'$.outputMode'),'both') IN ('both','tags')));
