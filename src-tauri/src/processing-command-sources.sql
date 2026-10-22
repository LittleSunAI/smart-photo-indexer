ALTER TABLE processing_sources RENAME TO processing_sources_legacy;
CREATE TABLE processing_sources(
 job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
 command_id TEXT NOT NULL DEFAULT '',
 ordinal INTEGER NOT NULL,
 path TEXT NOT NULL COLLATE NOCASE,
 PRIMARY KEY(job_id,command_id,ordinal),UNIQUE(job_id,command_id,path)
);
INSERT INTO processing_sources(job_id,command_id,ordinal,path) SELECT job_id,'',ordinal,path FROM processing_sources_legacy;
DROP TABLE processing_sources_legacy;
