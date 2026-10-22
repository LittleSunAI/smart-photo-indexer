ALTER TABLE processing_jobs ADD COLUMN task_revision INTEGER NOT NULL DEFAULT 0;
CREATE TABLE processing_task_changes (
    task_id TEXT PRIMARY KEY REFERENCES processing_tasks(id) ON DELETE CASCADE,
    job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL
);
CREATE INDEX idx_processing_changes_job ON processing_task_changes(job_id,revision);
CREATE TRIGGER processing_task_insert AFTER INSERT ON processing_tasks BEGIN
    UPDATE processing_jobs SET task_revision=task_revision+1 WHERE id=NEW.job_id;
    INSERT INTO processing_task_changes(task_id,job_id,revision)
      VALUES(NEW.id,NEW.job_id,(SELECT task_revision FROM processing_jobs WHERE id=NEW.job_id));
END;
CREATE TRIGGER processing_task_update AFTER UPDATE ON processing_tasks BEGIN
    UPDATE processing_jobs SET task_revision=task_revision+1 WHERE id=NEW.job_id;
    INSERT INTO processing_task_changes(task_id,job_id,revision)
      VALUES(NEW.id,NEW.job_id,(SELECT task_revision FROM processing_jobs WHERE id=NEW.job_id))
      ON CONFLICT(task_id) DO UPDATE SET revision=excluded.revision;
END;
INSERT INTO processing_task_changes(task_id,job_id,revision)
  SELECT id,job_id,rowid FROM processing_tasks;
UPDATE processing_jobs SET task_revision=COALESCE((SELECT MAX(revision) FROM processing_task_changes WHERE job_id=processing_jobs.id),0);
CREATE TABLE processing_service_cooldowns(provider TEXT PRIMARY KEY,not_before_unix INTEGER NOT NULL);
