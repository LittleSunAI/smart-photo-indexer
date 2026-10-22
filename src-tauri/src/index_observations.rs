use crate::*;
use std::{collections::{BTreeMap, HashMap}, sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};

#[derive(Clone, PartialEq, Eq,Serialize,Deserialize)]
pub(crate) struct Stamp { size: u64, modified: SystemTime, created: Option<SystemTime>, identity: (u32, u64), changed: i64 }

fn stamp(path: &Path) -> Option<Stamp> {
    #[cfg(windows)] {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::*;
        // Attribute-only handle, shared with editors/deletion. No photo bytes are read.
        let file=fs::OpenOptions::new().access_mode(FILE_READ_ATTRIBUTES).share_mode(FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE).open(path).ok()?;
        let metadata=file.metadata().ok()?;if !metadata.is_file(){return None;}
        let handle=file.as_raw_handle();
        let mut info:BY_HANDLE_FILE_INFORMATION=unsafe{std::mem::zeroed()};
        let mut basic:FILE_BASIC_INFO=unsafe{std::mem::zeroed()};
        if unsafe{GetFileInformationByHandle(handle,&mut info)}==0 || unsafe{GetFileInformationByHandleEx(handle,FileBasicInfo,&mut basic as *mut _ as *mut _,std::mem::size_of::<FILE_BASIC_INFO>() as u32)}==0{return None;}
        if basic.FileAttributes&FILE_ATTRIBUTE_REPARSE_POINT!=0{return None;}
        Some(Stamp{size:metadata.len(),modified:metadata.modified().ok()?,created:metadata.created().ok(),identity:(info.dwVolumeSerialNumber,(u64::from(info.nFileIndexHigh)<<32)|u64::from(info.nFileIndexLow)),changed:basic.ChangeTime})
    }
    #[cfg(not(windows))] { let _=path;None }
}

// Include file identity and change time even when an editor restores the exact mtime.
// Filesystems without a trustworthy Windows stamp use a full content fingerprint.
pub fn content_revision(path: &Path) -> Result<String, String> {
    if let Some(value) = stamp(path) {
        Ok(revision_from_stamp(&value))
    } else {
        compute_file_sha256(path)
    }
}
fn revision_from_stamp(value:&Stamp)->String{
        let time = |value: SystemTime| value.duration_since(UNIX_EPOCH).map(|v| format!("+{}", v.as_nanos())).unwrap_or_else(|e| format!("-{}", e.duration().as_nanos()));
        let raw = format!("{}:{}:{}:{}:{}:{}", value.identity.0, value.identity.1, value.changed, value.size, time(value.modified), value.created.map(time).unwrap_or_default());
        format!("{:x}", Sha256::digest(raw.as_bytes()))
}
pub(crate) fn observed_revision(value:&Option<Stamp>,sha:&str)->String{value.as_ref().map(revision_from_stamp).unwrap_or_else(||sha.to_string())}

struct Entry { stamp: Stamp, sha: String, metadata: image_metadata::ImageMetadata, order: u64, bytes: usize, checked: Instant }
#[derive(Default)]
struct Cache { entries: HashMap<String,Entry>, order: BTreeMap<u64,String>, sequence: u64, bytes: usize }
const MAX_ENTRIES:usize=4096;
const MAX_BYTES:usize=8*1024*1024;
const TTL:Duration=Duration::from_secs(600);
static CACHE:OnceLock<Mutex<Cache>>=OnceLock::new();
static FULL_FILES:AtomicU64=AtomicU64::new(0);
static HASH_BYTES:AtomicU64=AtomicU64::new(0);
static HITS:AtomicU64=AtomicU64::new(0);
fn cache()->&'static Mutex<Cache>{CACHE.get_or_init(Default::default)}
impl Cache {
    fn remove(&mut self,path:&str) {
        if let Some(old)=self.entries.remove(path){self.order.remove(&old.order);self.bytes=self.bytes.saturating_sub(old.bytes);}
    }
    fn next(&mut self)->u64 {
        self.sequence=self.sequence.wrapping_add(1);
        if self.sequence==0{self.entries.clear();self.order.clear();self.bytes=0;self.sequence=1;}
        self.sequence
    }
    fn get(&mut self,path:&str,stamp:&Stamp)->Option<(String,image_metadata::ImageMetadata)> {
        if self.entries.get(path).is_some_and(|entry|entry.stamp!=*stamp || entry.checked.elapsed()>TTL){self.remove(path);}
        let order=self.next();let entry=self.entries.get_mut(path)?;
        self.order.remove(&entry.order);entry.order=order;self.order.insert(order,path.to_string());
        Some((entry.sha.clone(),entry.metadata.clone()))
    }
    fn put(&mut self,path:String,stamp:Stamp,sha:String,metadata:image_metadata::ImageMetadata) {
        self.remove(&path);
        // Account for both path copies, retained string buffers and fixed fields.
        let bytes=path.capacity()*2+sha.capacity()+serde_json::to_vec(&metadata).map(|v|v.len()*2).unwrap_or(4096)+1024;
        if bytes>MAX_BYTES{return;}
        while self.entries.len()>=MAX_ENTRIES || self.bytes+bytes>MAX_BYTES {
            let Some((_,old_path))=self.order.pop_first()else{break;};self.remove(&old_path);
        }
        let order=self.next();self.bytes+=bytes;self.order.insert(order,path.clone());
        self.entries.insert(path,Entry{stamp,sha,metadata,order,bytes,checked:Instant::now()});
    }
}

fn cache_key(db:&Connection,path:&Path)->String{format!("{}\n{}",db.path().unwrap_or("memory"),path_to_string(path))}
fn persisted(db:&Connection,path:&Path,stamp:&Stamp)->Option<(String,image_metadata::ImageMetadata)>{
    let (saved,sha,metadata):(String,String,String)=db.query_row("SELECT o.stamp,o.sha256,o.metadata FROM file_observations o JOIN image_paths p ON p.path=o.path AND p.image_id=o.image_id AND p.is_current=1 JOIN images i ON i.id=o.image_id AND i.sha256=o.sha256 WHERE o.path=?1 AND o.format_version=1",[path_to_string(path)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).ok()?;
    if serde_json::from_str::<Stamp>(&saved).ok().as_ref()!=Some(stamp)||sha.len()!=64{return None;}
    Some((sha,serde_json::from_str(&metadata).ok()?))
}
pub(crate) fn persist(db:&Connection,path:&Path,image_id:&str,observed:&Option<Stamp>,sha:&str,metadata:&image_metadata::ImageMetadata)->Result<(),String>{
    // Publish only together with the matching image/path transaction, after a stable read.
    if observed.is_none()||stamp(path)!=*observed{db.execute("DELETE FROM file_observations WHERE path=?1",[path_to_string(path)]).map_err(|e|e.to_string())?;return Ok(());}
    db.execute("INSERT INTO file_observations(path,image_id,format_version,stamp,sha256,metadata) VALUES(?1,?2,1,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET image_id=excluded.image_id,format_version=1,stamp=excluded.stamp,sha256=excluded.sha256,metadata=excluded.metadata",params![path_to_string(path),image_id,serde_json::to_string(observed.as_ref().unwrap()).map_err(|e|e.to_string())?,sha,serde_json::to_string(metadata).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;Ok(())
}
pub fn preview_metadata(db:Option<&Connection>,path:&Path)->image_metadata::ImageMetadata{
    // Reuse only a validated header from the bounded native observation cache;
    // cache misses read headers outside SQLite and never create an unverified SHA.
    if let (Some(db),Some(stamp))=(db,stamp(path)){if let Ok(mut cache)=cache().lock(){if let Some((_,metadata))=cache.get(&cache_key(db,path),&stamp){return metadata;}}if let Some((_,metadata))=persisted(db,path,&stamp){return metadata;}}
    image_metadata::read(path)
}

pub fn observe(db:&Connection,path:&Path,reuse:bool)->Result<(String,image_metadata::ImageMetadata,Option<Stamp>),String> {
    let key=cache_key(db,path);let before=stamp(path);
    if reuse {
        if let Some(stamp)=&before {
            if let Some(value)=cache().lock().map_err(|_|"Observation cache lock failed.")?.get(&key,stamp) {
                HITS.fetch_add(1,Ordering::Relaxed);return Ok((value.0,value.1,before));
            }
            if let Some((sha,metadata))=persisted(db,path,stamp){cache().lock().map_err(|_|"Observation cache lock failed.")?.put(key.clone(),stamp.clone(),sha.clone(),metadata.clone());HITS.fetch_add(1,Ordering::Relaxed);return Ok((sha,metadata,before));}
        }
    }
    // Watcher events, explicit verification, favorite/AI and old IPC always hash.
    cache().lock().map_err(|_|"Observation cache lock failed.")?.remove(&key);
    let sha=compute_file_sha256(path)?;let metadata=image_metadata::read(path);
    FULL_FILES.fetch_add(1,Ordering::Relaxed);
    HASH_BYTES.fetch_add(before.as_ref().map(|value|value.size).or(metadata.size_bytes).unwrap_or(0),Ordering::Relaxed);
    let after=stamp(path);
    if before.is_some() && before!=after{return Err("Файл изменился во время проверки; повторите обновление.".into());}
    if let Some(stamp)=after.as_ref() {cache().lock().map_err(|_|"Observation cache lock failed.")?.put(key,stamp.clone(),sha.clone(),metadata.clone());}
    Ok((sha,metadata,after))
}

#[tauri::command]
pub fn get_index_observation_statistics()->Result<serde_json::Value,String> {
    let cache=cache().lock().map_err(|_|"Observation cache lock failed.")?;
    Ok(serde_json::json!({"fullFiles":FULL_FILES.load(Ordering::Relaxed),"photoBytesScheduledForHash":HASH_BYTES.load(Ordering::Relaxed),"cacheHits":HITS.load(Ordering::Relaxed),"cacheEntries":cache.entries.len(),"cacheAccountedBytes":cache.bytes,"maxEntries":MAX_ENTRIES,"maxAccountedBytes":MAX_BYTES,"ttlSeconds":TTL.as_secs()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stamp() -> Stamp {
        Stamp { size: 24, modified: SystemTime::UNIX_EPOCH, created: None, identity: (1, 2), changed: 3 }
    }

    #[test]
    fn persistent_observation_requires_matching_active_path_sha_stamp_and_format(){
        let db=Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE images(id TEXT PRIMARY KEY,sha256 TEXT);CREATE TABLE image_paths(path TEXT PRIMARY KEY,image_id TEXT,is_current INTEGER);CREATE TABLE file_observations(path TEXT PRIMARY KEY,image_id TEXT,format_version INTEGER,stamp TEXT,sha256 TEXT,metadata TEXT);").unwrap();
        let sha="a".repeat(64);let saved=sample_stamp();let raw=serde_json::to_string(&saved).unwrap();let metadata=serde_json::to_string(&image_metadata::ImageMetadata{width:Some(640),..Default::default()}).unwrap();
        db.execute("INSERT INTO images VALUES('image',?1)",[&sha]).unwrap();db.execute("INSERT INTO image_paths VALUES('photo','image',1)",[]).unwrap();
        db.execute("INSERT INTO file_observations VALUES('photo','image',1,?1,?2,?3)",params![raw,sha,metadata]).unwrap();
        assert_eq!(persisted(&db,Path::new("photo"),&saved).unwrap().1.width,Some(640));
        let mut changed=saved.clone();changed.changed+=1;assert!(persisted(&db,Path::new("photo"),&changed).is_none());
        db.execute("UPDATE images SET sha256='other'",[]).unwrap();assert!(persisted(&db,Path::new("photo"),&saved).is_none());
        db.execute("UPDATE images SET sha256=?1",[&sha]).unwrap();db.execute("UPDATE image_paths SET is_current=0",[]).unwrap();assert!(persisted(&db,Path::new("photo"),&saved).is_none());
        db.execute("UPDATE image_paths SET is_current=1",[]).unwrap();db.execute("UPDATE file_observations SET format_version=2",[]).unwrap();assert!(persisted(&db,Path::new("photo"),&saved).is_none());
    }

    #[test]
    fn cache_expiration_and_file_identity_reject_stale_content() {
        let mut cache = Cache::default();
        let stamp = sample_stamp();
        cache.put("photo".into(), stamp.clone(), "sha".into(), Default::default());
        let mut replaced = stamp.clone();
        replaced.identity.1 += 1;
        assert!(cache.get("photo", &replaced).is_none());
        assert!(cache.entries.is_empty());
        cache.put("photo".into(), stamp.clone(), "sha".into(), Default::default());
        cache.entries.get_mut("photo").unwrap().checked = Instant::now() - TTL - Duration::from_secs(1);
        assert!(cache.get("photo", &stamp).is_none());
        assert!(cache.order.is_empty());
        assert_eq!(cache.bytes, 0);
    }

    #[test]
    fn least_recently_used_is_evicted_and_hot_entry_survives() {
        let mut cache = Cache::default();
        let stamp = sample_stamp();
        for i in 0..MAX_ENTRIES {
            cache.put(format!("photo-{i}"), stamp.clone(), "sha".into(), Default::default());
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert!(cache.get("photo-0", &stamp).is_some());
        cache.put("extra".into(), stamp.clone(), "sha".into(), Default::default());
        assert!(cache.get("photo-1", &stamp).is_none());
        assert!(cache.get("photo-0", &stamp).is_some());
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert_eq!(cache.entries.len(), cache.order.len());
        assert!(cache.bytes <= MAX_BYTES);
    }

    #[test]
    fn byte_budget_bounds_large_metadata_and_rejects_oversized_entry() {
        let mut cache = Cache::default();
        let stamp = sample_stamp();
        for i in 0..32 {
            let metadata = image_metadata::ImageMetadata { camera_model: Some("x".repeat(256 * 1024)), ..Default::default() };
            cache.put(format!("large-{i}"), stamp.clone(), "sha".into(), metadata);
            assert!(cache.bytes <= MAX_BYTES);
            assert_eq!(cache.entries.len(), cache.order.len());
        }
        assert!(cache.entries.len() < 32);
        assert!(cache.get("large-31", &stamp).is_some());
        let metadata = image_metadata::ImageMetadata { camera_model: Some("x".repeat(MAX_BYTES)), ..Default::default() };
        cache.put("oversized".into(), stamp.clone(), "sha".into(), metadata);
        assert!(cache.get("oversized", &stamp).is_none());
        assert_eq!(cache.bytes, cache.entries.values().map(|entry| entry.bytes).sum::<usize>());
    }
}
