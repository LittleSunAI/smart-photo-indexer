use super::*;

struct Fixture { root:PathBuf, db:Connection }
impl Fixture {
    fn new()->Self{
        let root=crate::test_support::temp_dir("smart-photo-file-actions");
        let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();Self{root,db}
    }
    fn file(&self,name:&str,bytes:&[u8])->String{let path=self.root.join(name);if let Some(parent)=path.parent(){fs::create_dir_all(parent).unwrap();}fs::write(&path,bytes).unwrap();path_to_string(&path)}
    fn payload(&self,kind:Kind,source:&str,target:Option<&str>)->Payload{
        Payload{version:1,batch_id:nonce("test-file-action"),ordinal:0,kind,phase:"prepared".into(),source_path:source.into(),target_path:target.map(str::to_string),
            source_fingerprint:io::at(Path::new(source)).unwrap().unwrap(),target_before:target.and_then(|t|io::at(Path::new(t)).unwrap()),stage_path:None,stage_identity:None,
            backup_path:None,published_fingerprint:None,source_removed:false,editor_database:None,project_bindings:Vec::new(),paused_task_ids:Vec::new(),trash_receipt:None,
            image_id:None,filesystem_applied:false,index_applied:false,editor_applied:false,outcome:None}
    }
}
impl Drop for Fixture{fn drop(&mut self){
    if let (Ok(root),Ok(base))=(self.root.canonicalize(),std::env::temp_dir().canonicalize()){
        if root.starts_with(base)&&root.file_name().is_some_and(|n|n.to_string_lossy().starts_with("smart-photo-file-actions-")){
            let _=fs::remove_dir_all(root);
        }
    }
}}

#[test]
fn migration_preserves_old_intents_and_is_idempotent(){
    let mut db=Connection::open_in_memory().unwrap();run_index_migrations(&mut db).unwrap();
    db.execute("INSERT INTO file_operations(id,kind,source_path,sha256,size_bytes,status,created_at_unix,updated_at_unix) VALUES('old','trash','old.png','sha',1,'committed',1,1)",[]).unwrap();
    assert_eq!(run_index_migrations(&mut db).unwrap(),0);
    assert_eq!(db.query_row("SELECT payload FROM file_operations WHERE id='old'",[],|r|r.get::<_,Option<String>>(0)).unwrap(),None);
    assert_eq!(current_schema_version(&db).unwrap(),28);
}
#[test]
fn copy_preserves_bytes_source_alias_and_metadata(){
    let mut f=Fixture::new();let source=f.file("source.png",b"unchanged pixels");let target=path_to_string(&f.root.join("copy.png"));
    let mut p=f.payload(Kind::Copy,&source,Some(&target));io::persist(&f.db,&p).unwrap();
    let mut stage=io::stage(&f.db,&mut p).unwrap();io::publish(&f.db,&mut p,&mut stage).unwrap();io::finish_move(&f.db,&mut p,stage).unwrap();
    metadata::commit_index(&mut f.db,&mut p).unwrap();
    assert_eq!(fs::read(&source).unwrap(),fs::read(&target).unwrap());
    assert_eq!(f.db.query_row("SELECT COUNT(*) FROM image_paths WHERE is_current=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    assert_eq!(f.db.query_row("SELECT COUNT(*) FROM images",[],|r|r.get::<_,i64>(0)).unwrap(),1);
}
#[test]
fn move_and_replace_keep_backup_until_commit(){
    let mut f=Fixture::new();let source=f.file("source.png",b"new pixels");let target=f.file("target.png",b"old pixels");
    let mut p=f.payload(Kind::Move,&source,Some(&target));io::persist(&f.db,&p).unwrap();
    let mut stage=io::stage(&f.db,&mut p).unwrap();io::publish(&f.db,&mut p,&mut stage).unwrap();io::finish_move(&f.db,&mut p,stage).unwrap();
    assert!(!Path::new(&source).exists());assert_eq!(fs::read(&target).unwrap(),b"new pixels");
    assert_eq!(fs::read(p.backup_path.as_ref().unwrap()).unwrap(),b"old pixels");
    metadata::commit_index(&mut f.db,&mut p).unwrap();let warnings=io::cleanup(&p);assert!(warnings.is_empty());
    assert!(!Path::new(p.backup_path.as_ref().unwrap()).exists());
}
#[test]
fn changed_target_is_never_overwritten_after_decision(){
    let f=Fixture::new();let source=f.file("source.png",b"new pixels");let target=f.file("target.png",b"old pixels");
    let mut p=f.payload(Kind::Copy,&source,Some(&target));io::persist(&f.db,&p).unwrap();let mut stage=io::stage(&f.db,&mut p).unwrap();
    fs::write(&target,b"a different photo now").unwrap();assert!(io::publish(&f.db,&mut p,&mut stage).is_err());drop(stage);
    assert_eq!(fs::read(&target).unwrap(),b"a different photo now");assert_eq!(fs::read(&source).unwrap(),b"new pixels");io::cleanup(&p);
}
#[test]
fn source_change_after_capture_is_detected(){
    let f=Fixture::new();let source=f.file("source.png",b"old");let captured=io::capture(&source).unwrap();
    fs::write(&source,b"changed content").unwrap();let actual=io::at(Path::new(&source)).unwrap().unwrap();
    assert!(captured.modified_ns!=actual.modified_ns||captured.size_bytes!=actual.size_bytes);
}
#[test]
fn source_alias_metadata_stays_with_content_when_target_is_replaced(){
    let mut f=Fixture::new();let source=f.file("source.png",b"new");let keep=f.file("kept.png",b"new");let target=f.file("target.png",b"old");
    let hash=compute_file_sha256(Path::new(&source)).unwrap();
    for (id,sha,description) in [("new",hash.as_str(),"new description"),("old","old-sha","old description")]{
        f.db.execute("INSERT INTO images(id,identity_key,file_name,sha256,description,status,is_favorite,created_at_unix,updated_at_unix) VALUES(?1,?1,'photo.png',?2,?3,'processed',1,1,1)",params![id,sha,description]).unwrap();}
    for (id,path) in [("new",&source),("new",&keep),("old",&target)]{f.db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES(?1,?2,1,1,1)",params![id,path]).unwrap();}
    let mut p=f.payload(Kind::Move,&source,Some(&target));io::persist(&f.db,&p).unwrap();let mut stage=io::stage(&f.db,&mut p).unwrap();io::publish(&f.db,&mut p,&mut stage).unwrap();io::finish_move(&f.db,&mut p,stage).unwrap();metadata::commit_index(&mut f.db,&mut p).unwrap();
    assert_eq!(f.db.query_row("SELECT COUNT(*) FROM image_paths WHERE image_id='new' AND is_current=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    assert_eq!(f.db.query_row("SELECT description FROM images WHERE id='old'",[],|r|r.get::<_,String>(0)).unwrap(),"old description");
    assert_eq!(f.db.query_row("SELECT status FROM images WHERE id='old'",[],|r|r.get::<_,String>(0)).unwrap(),"missing");io::cleanup(&p);
}
#[test]
fn busy_checks_are_path_specific_and_history_does_not_block(){
    let f=Fixture::new();f.db.execute_batch("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('i','i','photo.png','sha',1,1);INSERT INTO processing_jobs(id,request_key,status,config,created_at_unix,updated_at_unix) VALUES('j','j','paused','{\"providerMode\":\"local\"}',1,1);INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','source.png','sha','{}','pending');").unwrap();
    assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_ok());
    assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::from(["t".into()])).is_err());
    assert!(metadata::check_recognition_busy(&f.db,&["other-alias.png".into()],&HashSet::from(["t".into()])).is_ok());
    f.db.execute("UPDATE processing_jobs SET status='running'",[]).unwrap();assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_err());
    f.db.execute("UPDATE processing_tasks SET status='failed'",[]).unwrap();assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_ok());
}
#[test]
fn publication_invalidation_keeps_generation_records_and_stops_old_intent(){
    let db=Connection::open_in_memory().unwrap();db.execute_batch("CREATE TABLE editor_publications(id TEXT,state TEXT,target_path TEXT,error TEXT);INSERT INTO editor_publications VALUES('p','planned','source.png',NULL),('q','completed','source.png',NULL),('r','planned','other.png',NULL);").unwrap();
    metadata::invalidate_publications(&db,&["source.png".into()]).unwrap();
    assert_eq!(db.query_row("SELECT state FROM editor_publications WHERE id='p'",[],|r|r.get::<_,String>(0)).unwrap(),"cancelled");
    assert_eq!(db.query_row("SELECT state FROM editor_publications WHERE id='q'",[],|r|r.get::<_,String>(0)).unwrap(),"completed");
    assert_eq!(db.query_row("SELECT state FROM editor_publications WHERE id='r'",[],|r|r.get::<_,String>(0)).unwrap(),"planned");
    assert!(photo_editor::publication::require_live_intent(&db,"p").is_err());
    assert!(photo_editor::publication::require_live_intent(&db,"r").is_ok());
}
#[test]
fn filename_indices_preserve_extension_and_source_identity_protects_hardlinks(){
    let f=Fixture::new();let source=f.file("снимок.png",b"pixels");let alias=f.root.join("alias.png");fs::hard_link(&source,&alias).unwrap();
    assert_eq!(io::identity_at(Path::new(&source)).unwrap(),io::identity_at(&alias).unwrap());
    assert_eq!(io::numbered(Path::new(&source),2).file_name().unwrap(),"снимок (2).png");
    // Windows may treat rename onto another hardlink as a successful no-op.
    // The executor therefore checks identity before asking it to rename.
    assert!(Path::new(&source).exists());assert_eq!(fs::read(&alias).unwrap(),b"pixels");
}

#[test]
fn crash_before_publish_never_creates_the_target_on_recovery(){
    let f=Fixture::new();let source=f.file("source.png",b"original");let target=path_to_string(&f.root.join("target.png"));
    let mut p=f.payload(Kind::Copy,&source,Some(&target));let stage=io::stage(&f.db,&mut p).unwrap();drop(stage);
    assert!(!io::inspect_recovery(&mut p).unwrap());
    assert_eq!(fs::read(&source).unwrap(),b"original");assert!(!Path::new(&target).exists());
    assert!(!Path::new(p.stage_path.as_ref().unwrap()).exists());
}
#[test]
fn crash_after_backup_restores_old_target_without_publishing_new_copy(){
    let f=Fixture::new();let source=f.file("source.png",b"new");let target=f.file("target.png",b"old");
    let mut p=f.payload(Kind::Copy,&source,Some(&target));let stage=io::stage(&f.db,&mut p).unwrap();drop(stage);
    let backup=f.root.join("owned.backup");p.backup_path=Some(path_to_string(&backup));io::phase(&f.db,&mut p,"backupIntent").unwrap();
    crate::guarded_file::GuardedFile::open(Path::new(&target),true).unwrap().rename(&backup).unwrap();
    assert!(!io::inspect_recovery(&mut p).unwrap());assert_eq!(fs::read(&target).unwrap(),b"old");assert_eq!(fs::read(&source).unwrap(),b"new");
}
#[test]
fn crash_after_cross_publish_keeps_source_and_records_partial(){
    let f=Fixture::new();let source=f.file("source.png",b"original");let target=path_to_string(&f.root.join("target.png"));
    let mut p=f.payload(Kind::Copy,&source,Some(&target));let mut stage=io::stage(&f.db,&mut p).unwrap();
    // Force the copy/publish/remove branch without requiring another physical volume in this unit test.
    // The native acceptance separately verifies a real D -> C move.
    p.kind=Kind::Move;io::publish(&f.db,&mut p,&mut stage).unwrap();drop(stage);
    assert!(io::inspect_recovery(&mut p).unwrap());assert!(!p.source_removed);
    assert_eq!(fs::read(&source).unwrap(),fs::read(&target).unwrap());
}
#[test]
fn reused_source_path_after_recorded_move_is_never_retired_as_old_content(){
    let mut f=Fixture::new();let source=f.file("source.png",b"original");let target=path_to_string(&f.root.join("target.png"));
    let mut p=f.payload(Kind::Move,&source,Some(&target));let mut stage=io::stage(&f.db,&mut p).unwrap();io::publish(&f.db,&mut p,&mut stage).unwrap();io::finish_move(&f.db,&mut p,stage).unwrap();
    fs::write(&source,b"a completely different photo").unwrap();assert!(io::inspect_recovery(&mut p).unwrap());assert!(p.source_removed);
    f.db.execute("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('new-owner','new-owner','source.png','different-sha',1,1)",[]).unwrap();
    f.db.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix) VALUES('new-owner',?1,1,1,1)",[&source]).unwrap();
    metadata::commit_index(&mut f.db,&mut p).unwrap();
    assert_eq!(f.db.query_row("SELECT is_current FROM image_paths WHERE path=?1 AND image_id='new-owner'",[&source],|r|r.get::<_,i64>(0)).unwrap(),1);
    assert_eq!(fs::read(&source).unwrap(),b"a completely different photo");
}
#[test]
fn missing_source_without_trash_receipt_is_never_a_confirmed_delete(){
    let f=Fixture::new();let source=f.file("source.png",b"original");let mut p=f.payload(Kind::Trash,&source,None);
    fs::rename(&source,f.root.join("externally-moved.png")).unwrap();
    assert!(io::inspect_recovery(&mut p).is_err());assert!(!p.filesystem_applied);assert!(!p.source_removed);
}
#[test]
fn recovery_busy_protects_only_its_paths_and_not_its_own_active_row(){
    let f=Fixture::new();let source=f.file("source.png",b"original");let p=f.payload(Kind::Trash,&source,None);io::persist(&f.db,&p).unwrap();
    assert!(metadata::check_recovery_busy(&f.db,std::slice::from_ref(&source),&p.id()).is_ok());
    assert!(metadata::check_recovery_busy(&f.db,std::slice::from_ref(&source),"other").is_err());
    assert!(metadata::check_recovery_busy(&f.db,&["unrelated.png".into()],"other").is_ok());
}
#[test]
fn cloud_unknown_blocks_even_paused_but_settled_and_stale_local_do_not(){
    let f=Fixture::new();f.db.execute_batch("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('i','i','photo.png','sha',1,1);INSERT INTO processing_jobs(id,request_key,status,config,created_at_unix,updated_at_unix) VALUES('j','j','paused','{\"providerMode\":\"fal\"}',1,1);INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('t','j','i','source.png','sha','{}','failed');INSERT INTO processing_attempts(id,task_id,pass_number,generation,status,started_at_unix,reserved_micros) VALUES('a','t',1,0,'unknown',1,100);").unwrap();
    assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_err());
    f.db.execute("UPDATE processing_jobs SET config='{\"providerMode\":\"local\"}'",[]).unwrap();
    assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_ok());
    f.db.execute("UPDATE processing_jobs SET config='{\"providerMode\":\"fal\"}'",[]).unwrap();f.db.execute("UPDATE processing_attempts SET status='remoteFailed'",[]).unwrap();
    assert!(metadata::check_recognition_busy(&f.db,&["source.png".into()],&HashSet::new()).is_ok());
}
#[test]
fn editor_binding_commit_is_idempotent_and_keeps_project_revision(){
    let f=Fixture::new();let source=f.file("source.png",b"original");let target=path_to_string(&f.root.join("target.png"));let mut p=f.payload(Kind::Move,&source,Some(&target));p.source_removed=true;
    let mut editor=Connection::open_in_memory().unwrap();editor.execute_batch("CREATE TABLE editor_projects(id TEXT PRIMARY KEY,source_path TEXT,source_name TEXT,source_sha256 TEXT,revision INTEGER);").unwrap();
    editor.execute("INSERT INTO editor_projects VALUES('project',?1,'source.png','original-sha',42)",[&source]).unwrap();p.project_bindings=metadata::bindings(&editor,&source).unwrap();
    metadata::commit_editor(&mut editor,&mut p).unwrap();metadata::commit_editor(&mut editor,&mut p).unwrap();
    assert_eq!(editor.query_row("SELECT source_path FROM editor_projects",[],|r|r.get::<_,String>(0)).unwrap(),target);
    assert_eq!(editor.query_row("SELECT revision FROM editor_projects",[],|r|r.get::<_,i64>(0)).unwrap(),42);
}

#[test]
fn keep_both_reuses_verified_stage_if_another_process_claims_the_index(){
    let f=Fixture::new();let source=f.file("source.png",b"source pixels");let base=f.file("target.png",b"existing target");
    let target=io::next_free(Path::new(&base)).unwrap();let mut p=f.payload(Kind::Copy,&source,Some(&target));
    let mut staged=io::stage(&f.db,&mut p).unwrap();fs::write(&target,b"claimed by another process").unwrap();
    assert!(io::publish(&f.db,&mut p,&mut staged).is_err());assert!(!p.filesystem_applied);
    p.target_path=Some(io::next_free(Path::new(&base)).unwrap());assert!(p.target_path.as_ref().unwrap().ends_with("target (2).png"));
    io::publish(&f.db,&mut p,&mut staged).unwrap();drop(staged);
    assert_eq!(fs::read(&target).unwrap(),b"claimed by another process");assert_eq!(fs::read(&base).unwrap(),b"existing target");assert_eq!(fs::read(p.target_path.as_ref().unwrap()).unwrap(),b"source pixels");
}
#[test]
fn unavailable_parent_is_not_a_free_filename(){
    let f=Fixture::new();assert!(io::at(&f.root.join("disconnected-parent/photo.png")).is_err());
}
#[test]
fn paused_move_rebinds_only_paths_and_preserves_frozen_configuration(){
    let mut f=Fixture::new();let source=f.file("source.png",b"source pixels");let target=path_to_string(&f.root.join("target.png"));let mut p=f.payload(Kind::Move,&source,Some(&target));
    f.db.execute("INSERT INTO images(id,identity_key,file_name,sha256,created_at_unix,updated_at_unix) VALUES('i','i','source.png',?1,1,1)",[&p.source_fingerprint.sha256]).unwrap();
    let config=r#"{"providerMode":"fal","model":"frozen-model","frozenExecution":{"schema":"unchanged"}}"#;
    f.db.execute("INSERT INTO processing_jobs(id,request_key,status,config,created_at_unix,updated_at_unix) VALUES('job','job','paused',?1,1,1)",[config]).unwrap();
    let snapshot=serde_json::json!({"targetPath":source,"fileName":"source.png","sourceSnapshot":{"path":source,"name":"source.png","sourceRevision":"same-content"}}).to_string();
    f.db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('task','job','i',?1,?2,?3,'pending')",params![source,p.source_fingerprint.sha256,snapshot]).unwrap();
    p.paused_task_ids=metadata::paused_tasks(&f.db,&source).unwrap();metadata::check_rebind(&f.db,&p).unwrap();let mut staged=io::stage(&f.db,&mut p).unwrap();io::publish(&f.db,&mut p,&mut staged).unwrap();io::finish_move(&f.db,&mut p,staged).unwrap();metadata::commit_index(&mut f.db,&mut p).unwrap();
    let(path,raw):(String,String)=f.db.query_row("SELECT source_path,snapshot FROM processing_tasks WHERE id='task'",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();let saved:serde_json::Value=serde_json::from_str(&raw).unwrap();
    assert_eq!(path,target);assert_eq!(saved["sourceSnapshot"]["path"],target);assert_eq!(saved["sourceSnapshot"]["sourceRevision"],"same-content");
    assert_eq!(f.db.query_row("SELECT config FROM processing_jobs WHERE id='job'",[],|r|r.get::<_,String>(0)).unwrap(),config);
    f.db.execute("INSERT INTO processing_tasks(id,job_id,image_id,source_path,sha256,snapshot,status) VALUES('other','job','i',?1,'sha','{}','failed')",[&source]).unwrap();
    p.target_path=Some(source);assert!(metadata::check_rebind(&f.db,&p).is_err());
}
#[cfg(windows)]
#[test]
fn real_recycle_rejects_changed_source_without_removing_it(){
    let f=Fixture::new();let source=f.file("source.png",b"must remain unchanged");let mut expected=io::at(Path::new(&source)).unwrap().unwrap();expected.sha256="not the real hash".into();
    assert!(recycle::recycle(&source,&expected).is_err());assert_eq!(fs::read(source).unwrap(),b"must remain unchanged");
}
#[test]
fn source_observation_does_not_block_an_existing_file_rename(){
    let f=Fixture::new();let source=f.file("source.png",b"same content");let target=f.root.join("renamed.png");
    let observer=crate::guarded_file::GuardedFile::observe(Path::new(&source)).unwrap();let mover=crate::guarded_file::GuardedFile::open(Path::new(&source),true).unwrap();
    mover.rename(&target).unwrap();assert_eq!(observer.identity().unwrap(),mover.identity().unwrap());assert!(target.exists());assert!(!Path::new(&source).exists());
}
#[test]
fn file_batch_reservation_blocks_only_current_source_and_target_until_release(){
    let f=Fixture::new();let first=f.file("source.png",b"first");let second=f.file("second.png",b"second");
    let mut batch=Batch{id:"batch".into(),kind:Kind::Move,files:Arc::new(vec![io::capture(&first).unwrap(),io::capture(&second).unwrap()]),destination:Some(path_to_string(&f.root.join("target"))),source_identities:Arc::new(HashSet::new()),counts:Counts::default(),warnings:Vec::new(),clipboard_revision:None,next:0,revision:1,state:"running".into(),conflict:None,decision:None,all_choice:None,outcomes:Vec::new(),cancel:false};
    assert!(batch_reserves_path(&batch,&first.to_uppercase()));assert!(batch_reserves_path(&batch,&path_to_string(&f.root.join("target/source.png"))));assert!(!batch_reserves_path(&batch,&second));assert!(!batch_reserves_path(&batch,"different-alias.png"));
    batch.next=1;assert!(!batch_reserves_path(&batch,&first));assert!(batch_reserves_path(&batch,&second));batch.state="cancelled".into();assert!(!batch_reserves_path(&batch,&second));
}
#[test]
fn free_index_scan_does_not_open_or_hash_busy_existing_files(){
    let f=Fixture::new();let target=f.root.join("photo.png");let locked=f.root.join("photo (1).png");
    let _owner=crate::guarded_file::GuardedFile::create_new(&locked).unwrap();
    assert!(io::next_free(&target).unwrap().ends_with("photo (2).png"));
}
