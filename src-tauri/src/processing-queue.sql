-- Extend the existing queue. Legacy jobs retain their IDs and manual semantics.
ALTER TABLE processing_jobs ADD COLUMN queue_mode TEXT NOT NULL DEFAULT 'manual' CHECK(queue_mode IN ('manual','direct'));
CREATE UNIQUE INDEX idx_processing_single_direct ON processing_jobs(queue_mode) WHERE queue_mode='direct';
ALTER TABLE processing_tasks ADD COLUMN run_revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE processing_tasks ADD COLUMN work_visible INTEGER NOT NULL DEFAULT 1 CHECK(work_visible IN (0,1));
CREATE INDEX idx_processing_work ON processing_tasks(job_id,work_visible,status);
ALTER TABLE processing_attempts ADD COLUMN run_revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE processing_attempts ADD COLUMN execution_context TEXT;
ALTER TABLE processing_attempts ADD COLUMN normalized_result TEXT;

CREATE TABLE processing_commands (
 id TEXT PRIMARY KEY,
 request_key TEXT NOT NULL UNIQUE,
 job_id TEXT NOT NULL REFERENCES processing_jobs(id),
 status TEXT NOT NULL,
 execution_context TEXT NOT NULL,
 accepted_count INTEGER NOT NULL DEFAULT 0,
 skipped_count INTEGER NOT NULL DEFAULT 0,
 error TEXT,
 created_at_unix INTEGER NOT NULL,
 updated_at_unix INTEGER NOT NULL
);
CREATE INDEX idx_processing_commands_job ON processing_commands(job_id);

-- Preserve the old provider/key before any later task-key replacement.
UPDATE processing_attempts SET execution_context=(
 SELECT json_object('config',json(j.config),'credentialRef',j.credential_ref,'receiptId',NULL)
 FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.id=processing_attempts.task_id
), normalized_result=(SELECT p.result FROM processing_passes p WHERE p.attempt_id=processing_attempts.id);

-- One definition of effective ownership, shared by all native consumers. The
-- explicit object test preserves an intentional null key in a frozen context.
CREATE VIEW processing_task_context AS
 SELECT t.*,t.rowid AS task_ordinal,
 COALESCE(json_extract(t.snapshot,'$._execution.config'),j.config) AS effective_config,
 CASE WHEN json_type(t.snapshot,'$._execution')='object'
 THEN json_extract(t.snapshot,'$._execution.credentialRef') ELSE j.credential_ref END AS effective_credential_ref,
 json_extract(t.snapshot,'$._execution.receiptId') AS receipt_id
 FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id;

CREATE VIEW processing_attempt_context AS
 SELECT a.*,a.rowid AS attempt_ordinal,t.job_id,t.image_id,t.source_path,t.sha256,
 COALESCE(json_extract(a.execution_context,'$.config'),j.config) AS effective_config,
 CASE WHEN json_type(a.execution_context)='object'
 THEN json_extract(a.execution_context,'$.credentialRef') ELSE j.credential_ref END AS effective_credential_ref,
 json_extract(a.execution_context,'$.receiptId') AS receipt_id
 FROM processing_attempts a JOIN processing_tasks t ON t.id=a.task_id JOIN processing_jobs j ON j.id=t.job_id;
