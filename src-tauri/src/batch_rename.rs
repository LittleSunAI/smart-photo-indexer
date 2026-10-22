//! Standalone, byte-preserving rename. The journal records intent before every move.
//! Planned batches never run on startup; recovery requires an explicit user command.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::Emitter;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rules {
    pub common_name: Option<String>, pub prefix: String, pub suffix: String,
    pub numbering: bool, pub start: u64, pub digits: usize, pub step: u64,
    pub position: String, pub order: String, pub per_folder: bool,
    // Missing in historic journals: retain their literal prefix/number/suffix assembly.
    #[serde(default)] pub separator: Option<String>,
}
impl Default for Rules {
    fn default() -> Self { Self { common_name: None, prefix: String::new(), suffix: String::new(), numbering: true,
        start: 1, digits: 3, step: 1, position: "after".into(), order: "current".into(), per_folder: false, separator:Some("_".into()) } }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Fingerprint { size: u64, modified_ns: String, identity: String, sha256: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub old_path: String, pub new_path: String, pub old_name: String, pub new_name: String,
    pub status: String, pub message: String, pub image_id: Option<String>,
    pub size_bytes: u64, pub modified_at_unix: u64,
    fingerprint: Fingerprint, temporary: String, projects: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub id: String, pub state: String, pub created_at_unix: u64, pub rules: Rules,
    pub rows: Vec<Row>, pub error: Option<String>,
    #[serde(default)] reverse: bool,
    #[serde(default)] editor_database: String,
    #[serde(default)] pub completed_count: usize,
    #[serde(default)] pub remaining_count: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewInput { pub paths: Vec<String>, pub include_subfolders: bool, pub rules: Rules }

fn linked(m: &fs::Metadata) -> bool {
    if m.file_type().is_symlink() { return true; }
    #[cfg(windows)] { use std::os::windows::fs::MetadataExt; if m.file_attributes() & 0x400 != 0 { return true; } }
    false
}
fn key(path: &str) -> String { path.replace('/', "\\").to_lowercase() }
fn identity(path: &Path, m: &fs::Metadata) -> Result<String, String> {
    #[cfg(windows)] {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
        let f = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(f.as_raw_handle() as _, &mut info) } == 0 { return Err(std::io::Error::last_os_error().to_string()); }
        let _ = m;
        Ok(format!("{}:{}:{}", info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow))
    }
    #[cfg(unix)] { use std::os::unix::fs::MetadataExt; let _ = path; Ok(format!("{}:{}", m.dev(), m.ino())) }
    #[cfg(not(any(windows, unix)))] { let _ = path; Ok(format!("{:?}", m.created().ok())) }
}
fn fingerprint(path: &Path) -> Result<Fingerprint, String> {
    let before = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !before.is_file() || linked(&before) { return Err("Разрешены обычные файлы; ссылки и перенаправления исключены.".into()); }
    let id = identity(path, &before)?;
    let modified = before.modified().map_err(|e| e.to_string())?.duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos().to_string();
    let sha256 = compute_file_sha256(path)?;
    let after = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if after.len() != before.len() || after.modified().ok() != before.modified().ok() || identity(path, &after)? != id {
        return Err("Файл изменился во время проверки. Обновите предпросмотр.".into());
    }
    Ok(Fingerprint { size: before.len(), modified_ns: modified, identity: id, sha256 })
}
fn matches(path: &str, expected: &Fingerprint) -> bool { fingerprint(Path::new(path)).is_ok_and(|actual| actual == *expected) }
fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.ends_with([' ', '.']) || name.encode_utf16().count() > 255
        || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c)) { return Err("Недопустимое имя Windows: символы, длина или окончание.".into()); }
    let stem = name.split('.').next().unwrap_or("").trim_end().to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || ["COM", "LPT"].iter().any(|prefix| stem.strip_prefix(prefix).is_some_and(|tail| ["1","2","3","4","5","6","7","8","9","¹","²","³"].contains(&tail))) {
        return Err("Имя зарезервировано Windows.".into());
    }
    Ok(())
}
fn validate_rules(r: &Rules) -> Result<(), String> {
    if r.separator.as_ref().is_some_and(|v| !["_","-"," ",""].contains(&v.as_str())){return Err("Выберите допустимый разделитель.".into());}
    if r.digits > 12 || r.step == 0 || r.start > 999_999_999_999 || !["before", "after"].contains(&r.position.as_str())
        || !["current", "name", "modified"].contains(&r.order.as_str()) { return Err("Проверьте параметры счётчика и порядок.".into()); }
    for fragment in [&r.prefix, &r.suffix] { if fragment.encode_utf16().count() > 255 { return Err("Слишком длинный префикс или суффикс.".into()); } }
    if r.common_name.as_ref().is_some_and(|name| name.trim().is_empty()) { return Err("Введите общее имя.".into()); }
    Ok(())
}
fn make_name(old: &str, rules: &Rules, index: u64) -> Result<String, String> {
    let (stem, extension) = match old.rfind('.') { Some(i) if i > 0 => (&old[..i], &old[i..]), _ => (old, "") };
    let base = rules.common_name.as_deref().unwrap_or(stem);
    if let Some(separator)=&rules.separator{
        let mut parts=vec![rules.prefix.as_str(),base,rules.suffix.as_str()].into_iter().filter(|v|!v.is_empty()).map(str::to_string).collect::<Vec<_>>();
        if rules.numbering{
            let count=rules.step.checked_mul(index).and_then(|v|v.checked_add(rules.start)).filter(|n|*n<=999_999_999_999).ok_or("Счётчик вышел за допустимый диапазон.")?;
            let number=format!("{:0width$}",count,width=rules.digits);if rules.position=="before"{parts.insert(0,number);}else{parts.push(number);}
        }
        let name=format!("{}{extension}",parts.join(separator));valid_name(&name)?;return Ok(name);
    }
    let numbered = if rules.numbering {
        let count = rules.step.checked_mul(index).and_then(|v| v.checked_add(rules.start)).filter(|n| *n <= 999_999_999_999).ok_or("Счётчик вышел за допустимый диапазон.")?;
        let n = format!("{:0width$}", count, width = rules.digits);
        if rules.position == "before" { format!("{n}_{base}") } else { format!("{base}_{n}") }
    } else { base.into() };
    let name = format!("{}{numbered}{}{extension}", rules.prefix, rules.suffix);
    valid_name(&name)?; Ok(name)
}
fn collect(paths: &[String], recursive: bool) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() || paths.len() > 20_000 { return Err("Выберите файлы или папку (до 20 000 элементов).".into()); }
    let mut result = Vec::new(); let mut seen = HashSet::new(); let mut visited = HashSet::new();
    fn visit(path: &Path, recursive: bool, result: &mut Vec<PathBuf>, seen: &mut HashSet<String>, visited: &mut HashSet<String>) -> Result<(), String> {
        let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if linked(&meta) { return Err(format!("Ссылка исключена: {}", path.display())); }
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            if !visited.insert(key(&path_to_string(&canonical))) { return Ok(()); }
            let mut children = fs::read_dir(&canonical).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            children.sort_by_key(|child| child.file_name());
            for child in children {
                let m = fs::symlink_metadata(child.path()).map_err(|e| e.to_string())?;
                if linked(&m) { continue; }
                if m.is_file() || (recursive && m.is_dir()) { visit(&child.path(), recursive, result, seen, visited)?; }
            }
        } else if meta.is_file() && seen.insert(key(&path_to_string(&canonical))) { result.push(canonical); }
        if result.len() > 20_000 { return Err("В наборе больше 20 000 файлов. Разделите его на части.".into()); }
        Ok(())
    }
    for path in paths { let path = Path::new(path); if !path.is_absolute() { return Err("Нужен абсолютный путь.".into()); } visit(path, recursive, &mut result, &mut seen, &mut visited)?; }
    if result.is_empty() { return Err("В выбранном наборе нет обычных файлов.".into()); }
    Ok(result)
}
fn journal(app: &tauri::AppHandle) -> Result<Connection, String> {
    let directory = application_data_directory(app)?; fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let db = Connection::open(directory.join("rename-journal.sqlite")).map_err(|e| e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS batches(id TEXT PRIMARY KEY,state TEXT NOT NULL,payload TEXT NOT NULL,updated INTEGER NOT NULL);").map_err(|e| e.to_string())?;
    Ok(db)
}
fn persist(db: &Connection, plan: &Plan) -> Result<(), String> {
    db.execute("INSERT INTO batches(id,state,payload,updated) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET state=excluded.state,payload=excluded.payload,updated=excluded.updated",
        params![plan.id,plan.state,serde_json::to_string(plan).map_err(|e|e.to_string())?,unix_now()]).map_err(|e| e.to_string())?; Ok(())
}
fn load(db: &Connection, id: &str) -> Result<Plan, String> {
    let value: String = db.query_row("SELECT payload FROM batches WHERE id=?1", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
    serde_json::from_str(&value).map_err(|e| e.to_string())
}
fn ensure_no_other_pending(db: &Connection, id: &str) -> Result<(), String> {
    let pending: i64 = db.query_row("SELECT COUNT(*) FROM batches WHERE id<>?1 AND state NOT IN ('planned','completed','rolledBack')", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if pending > 0 { return Err("Сначала завершите или откатите прерванное переименование в журнале.".into()); } Ok(())
}
fn validate_targets(rows: &mut [Row]) {
    let sources: HashSet<String> = rows.iter().map(|r| key(&r.old_path)).collect();
    let mut counts = HashMap::new(); for row in rows.iter() { *counts.entry(key(&row.new_path)).or_insert(0usize) += 1; }
    for row in rows {
        if row.status == "error" { continue; }
        if counts.get(&key(&row.new_path)).copied().unwrap_or(0) > 1 { row.status = "conflict".into(); row.message = "Повтор нового имени внутри набора.".into(); }
        else if fs::symlink_metadata(&row.new_path).is_ok() && !sources.contains(&key(&row.new_path)) { row.status = "conflict".into(); row.message = "Новое имя уже занято другим файлом.".into(); }
    }
}
fn prepare(app: &tauri::AppHandle, input: PreviewInput) -> Result<Plan, String> {
    validate_rules(&input.rules)?;
    let _guard = file_operations::OPERATIONS_LOCK.lock().map_err(|_| "File operation lock failed.")?;
    let db = journal(app)?; ensure_no_other_pending(&db, "")?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let id = format!("batch-{}-{nonce}", std::process::id());
    let index = open_index_connection(app)?;
    let editor = photo_editor::store::connection(app)?;
    let editor_database = path_to_string(&photo_editor::store::root(app)?.join("editor.sqlite"));
    let mut sources = collect(&input.paths, input.include_subfolders)?.into_iter().map(|path| {
        let f = fingerprint(&path)?; Ok((path, f))
    }).collect::<Result<Vec<_>, String>>()?;
    if input.rules.order == "name" { sources.sort_by_key(|(path, _)| (file_name(path).to_lowercase(), path_to_string(path))); }
    if input.rules.order == "modified" { sources.sort_by(|a,b| a.1.modified_ns.parse::<u128>().unwrap_or(0).cmp(&b.1.modified_ns.parse::<u128>().unwrap_or(0)).then_with(|| a.0.cmp(&b.0))); }
    let mut counters = HashMap::new(); let mut rows = Vec::new();
    for (i, (path, f)) in sources.into_iter().enumerate() {
        let old_path = path_to_string(&path); let old_name = file_name(&path);
        let parent = path.parent().ok_or("Нет папки файла")?;
        let number = if input.rules.per_folder { let counter = counters.entry(parent.to_path_buf()).or_insert(0u64); let value = *counter; *counter += 1; value } else { i as u64 };
        let (new_name, status, message) = match make_name(&old_name, &input.rules, number) {
            Ok(name) => { let state = if name == old_name { "unchanged" } else { "ready" }; (name, state.into(), String::new()) },
            Err(error) => (old_name.clone(), "error".into(), error),
        };
        let image_id = index.query_row("SELECT image_id FROM image_paths WHERE path=?1 AND is_current=1", [&old_path], |r| r.get::<_,String>(0)).optional().map_err(|e| e.to_string())?;
        if let Some(image_id) = &image_id { if read_indexed_sha256(&index, image_id)?.is_some_and(|sha| sha != f.sha256) { return Err(format!("Файл изменён после индексирования: {old_name}. Обновите его индекс перед переименованием.")); } }
        let mut statement = editor.prepare("SELECT id FROM editor_projects WHERE source_path=?1").map_err(|e| e.to_string())?;
        let projects = statement.query_map([&old_path], |r| r.get(0)).map_err(|e| e.to_string())?.collect::<Result<Vec<String>,_>>().map_err(|e| e.to_string())?;
        let temporary = path_to_string(&parent.join(format!(".smart-photo-{id}-{i}.rename")));
        if Path::new(&temporary).exists() { return Err("Временное имя уже занято. Создайте новый предпросмотр.".into()); }
        rows.push(Row { new_path: path_to_string(&parent.join(&new_name)), old_path, old_name, new_name, status, message,
            size_bytes: f.size, modified_at_unix: (f.modified_ns.parse::<u128>().unwrap_or(0) / 1_000_000_000) as u64,
            fingerprint: f, temporary, image_id, projects });
    }
    validate_targets(&mut rows);
    let remaining_count=rows.iter().filter(|r|r.status=="ready").count();
    let plan = Plan { id, state: "planned".into(), created_at_unix: unix_now(), rules: input.rules, rows, error: None, reverse: false, editor_database,completed_count:0,remaining_count };
    persist(&db, &plan)?; Ok(plan)
}

// Phases, rather than a per-file guess, distinguish originals from targets in cycles.
fn move_checked(from: &str, to: &str, f: &Fingerprint) -> Result<(), String> {
    if !matches(from, f) { return Err(format!("Файл исчез или изменился: {from}")); }
    if fs::symlink_metadata(to).is_ok() { return Err(format!("Имя занято, перезапись запрещена: {to}")); }
    let parent = Path::new(to).parent().ok_or("Нет папки цели")?;
    let actual = parent.canonicalize().map_err(|e| e.to_string())?;
    if key(&path_to_string(&actual)) != key(&path_to_string(parent)) { return Err("Папка перенаправлена после предпросмотра.".into()); }
    file_safety::move_no_replace(Path::new(from), Path::new(to))?;
    if !matches(to, f) || Path::new(from).exists() { return Err("Перемещение требует проверки журнала.".into()); } Ok(())
}
fn changed(plan: &Plan) -> impl Iterator<Item=&Row> { plan.rows.iter().filter(|r| r.status != "unchanged") }
fn run_files(db: &Connection, plan: &mut Plan) -> Result<(), String> {
    if plan.state == "planned" {
        if plan.rows.iter().any(|r| !["ready", "unchanged"].contains(&r.status.as_str())) { return Err("Сначала устраните ошибки и совпадения имён.".into()); }
        for row in &plan.rows { if !matches(&row.old_path, &row.fingerprint) { return Err(format!("Предпросмотр устарел: {}", row.old_name)); } }
        let mut checked = plan.rows.clone(); validate_targets(&mut checked);
        if checked.iter().any(|r| r.status == "conflict") { return Err("Новое имя занято после предпросмотра. Обновите план.".into()); }
        plan.state = "staging".into(); persist(db, plan)?;
    }
    if plan.state == "staging" {
        for row in changed(plan) {
            // A crash may occur after the OS move and before our next journal write.
            if matches(&row.temporary, &row.fingerprint) {
                if matches(&row.old_path, &row.fingerprint) { return Err("Файл обнаружен в двух местах; требуется ручная сверка.".into()); }
            } else { move_checked(&row.old_path, &row.temporary, &row.fingerprint)?; }
        }
        plan.state = "publishing".into(); persist(db, plan)?;
    }
    if plan.state == "publishing" {
        for row in changed(plan) {
            if matches(&row.new_path, &row.fingerprint) {
                if Path::new(&row.temporary).exists() { return Err("Цель и временный файл существуют одновременно; перезапись запрещена.".into()); }
            } else { move_checked(&row.temporary, &row.new_path, &row.fingerprint)?; }
        }
        plan.state = "metadata".into(); persist(db, plan)?;
    }
    Ok(())
}
fn metadata(app: &tauri::AppHandle, journal_db: &Connection, plan: &mut Plan) -> Result<(), String> {
    for row in &plan.rows { if !matches(&row.new_path, &row.fingerprint) { return Err("Результат изменён до обновления библиотеки. Сверьте журнал.".into()); } }
    let mut index = open_index_connection(app)?;
    let marker = format!("batch-rename:{}:{}", plan.id, if plan.reverse { "reverse" } else { "forward" });
    let tx = index.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let committed: i64 = tx.query_row("SELECT COUNT(*) FROM action_logs WHERE action_type='batch_renamed' AND message=?1", [&marker], |r| r.get(0)).map_err(|e| e.to_string())?;
    let forward_committed: i64 = tx.query_row("SELECT COUNT(*) FROM action_logs WHERE action_type='batch_renamed' AND message=?1", [format!("batch-rename:{}:forward",plan.id)], |r| r.get(0)).map_err(|e|e.to_string())?;
    if committed == 0 && (!plan.reverse || forward_committed > 0) {
        let now = unix_now();
        // Current paths participating in a cycle can change owners atomically. Tags,
        // collection membership and image IDs stay with the content, not the filename.
        for row in changed(plan) {
            if let Some(id) = &row.image_id {
                let owner: Option<String> = tx.query_row("SELECT image_id FROM image_paths WHERE path=?1 AND is_current=1", [&row.old_path], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
                if owner.as_ref() != Some(id) { return Err("Идентичность исходника в библиотеке изменилась.".into()); }
            }
        }
        for row in changed(plan) { tx.execute("UPDATE image_paths SET is_current=0,missing_at_unix=?2,last_seen_at_unix=?2 WHERE path=?1", params![row.old_path,now]).map_err(|e| e.to_string())?; }
        for row in changed(plan) {
            if let Some(id) = &row.image_id {
                tx.execute("INSERT INTO image_paths(image_id,path,is_current,first_seen_at_unix,last_seen_at_unix,size_bytes,modified_at_unix) VALUES(?1,?2,1,?3,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET image_id=excluded.image_id,is_current=1,last_seen_at_unix=excluded.last_seen_at_unix,missing_at_unix=NULL,size_bytes=excluded.size_bytes,modified_at_unix=excluded.modified_at_unix", params![id,row.new_path,now,row.size_bytes,row.modified_at_unix]).map_err(|e| e.to_string())?;
                tx.execute("UPDATE images SET file_name=?2,updated_at_unix=?3 WHERE id=?1",params![id,row.new_name,now]).map_err(|e| e.to_string())?;
            } else {
                tx.execute("UPDATE image_paths SET is_current=0,missing_at_unix=?2,last_seen_at_unix=?2 WHERE path=?1",params![row.new_path,now]).map_err(|e|e.to_string())?;
            }
            tx.execute("INSERT INTO action_logs(action_type,image_id,old_value,new_value,message,created_at_unix) VALUES('file_renamed',?1,?2,?3,?4,?5)",params![row.image_id,row.old_path,row.new_path,marker,now]).map_err(|e| e.to_string())?;
        }
        // Paused/prepared work keeps its identity and remains paused/prepared.
        // Move path keys through the registered names to support UNIQUE job/path cycles.
        for row in changed(plan) {if let Some(id)=&row.image_id{
            tx.execute("UPDATE processing_tasks SET source_path=?2 WHERE source_path=?1 AND image_id=?3 AND status NOT IN ('succeeded','completed','cancelled','failed')",params![row.old_path,row.temporary,id]).map_err(|e|e.to_string())?;
            tx.execute("UPDATE file_operations SET source_path=?2 WHERE source_path=?1 AND image_id=?3 AND kind='rename' AND status='planned'",params![row.old_path,row.temporary,id]).map_err(|e|e.to_string())?;
        }}
        for row in changed(plan) {if let Some(id)=&row.image_id{
            tx.execute("UPDATE processing_tasks SET source_path=?2,snapshot=CASE WHEN json_type(snapshot,'$.sourceSnapshot')='object' THEN json_set(snapshot,'$.targetPath',?2,'$.fileName',?4,'$.sourceSnapshot.path',?2,'$.sourceSnapshot.name',?4) ELSE json_set(snapshot,'$.targetPath',?2,'$.fileName',?4) END WHERE source_path=?1 AND image_id=?3 AND status NOT IN ('succeeded','completed','cancelled','failed')",params![row.temporary,row.new_path,id,row.new_name]).map_err(|e|e.to_string())?;
            tx.execute("UPDATE file_operations SET source_path=?2 WHERE source_path=?1 AND image_id=?3 AND kind='rename' AND status='planned'",params![row.temporary,row.new_path,id]).map_err(|e|e.to_string())?;
        }}
        tx.execute("INSERT INTO action_logs(action_type,message,created_at_unix) VALUES('batch_renamed',?1,?2)",params![marker,now]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    // Capture the database and project IDs in the plan: cycles and retries cannot
    // accidentally update another project that now has the old path.
    let expected_database = path_to_string(&photo_editor::store::root(app)?.join("editor.sqlite"));
    if plan.editor_database != expected_database { return Err("Хранилище проектов сменилось. Верните прежнее хранилище для завершения журнала.".into()); }
    let mut editor = photo_editor::store::connection(app)?;
    let tx = editor.transaction().map_err(|e| e.to_string())?;
    for row in changed(plan) { for id in &row.projects {
        tx.execute("UPDATE editor_projects SET source_path=?2,source_name=?3 WHERE id=?1",params![id,row.new_path,row.new_name]).map_err(|e| e.to_string())?;
    } }
    tx.commit().map_err(|e| e.to_string())?;
    plan.state = if plan.reverse { "rolledBack" } else { "completed" }.into(); plan.error = None;
    for row in &mut plan.rows { row.message.clear(); }
    persist(journal_db, plan)?;
    let mappings: Vec<(String,String)> = changed(plan).map(|r| (r.old_path.clone(),r.new_path.clone())).collect();
    viewer_sessions::rename_paths(app, &mappings)?;
    app.emit("smart-photo-indexer:batch-renamed", serde_json::json!({"id":plan.id,"reverse":plan.reverse,"mappings":mappings})).map_err(|e| e.to_string())?;
    Ok(())
}
fn assert_not_processing(app: &tauri::AppHandle, plan: &Plan) -> Result<(), String> {
    let db = open_index_connection(app)?;
    for row in changed(plan) {
        let active: i64 = db.query_row("SELECT COUNT(*) FROM processing_tasks t JOIN processing_jobs j ON j.id=t.job_id WHERE t.source_path=?1 AND (j.status IN ('queued','running') OR t.status IN ('preparing','running','committing')) AND t.status NOT IN ('succeeded','completed','cancelled','failed')", [&row.old_path], |r| r.get(0)).map_err(|e| e.to_string())?;
        if active > 0 { return Err(format!("Файл сейчас в очереди распознавания: {}. Завершите или отмените эту задачу.", row.old_name)); }
    } Ok(())
}
fn execute_with_rules(app: &tauri::AppHandle, id: &str, action: &str,rules:Option<Rules>) -> Result<Plan, String> {
    if action == "inspect" { return load(&journal(app)?,id); }
    processing::cloud_fence(app, || {
        // Same lock order as recognition's run_rename: coordinator, then files.
        let _file_guard = file_operations::OPERATIONS_LOCK.lock().map_err(|_| "File operation lock failed.")?;
        let _index_guard = INDEX_SYNC_LOCK.lock().map_err(|_| "Index sync lock failed.")?;
        let db = journal(app)?; let mut plan = load(&db, id)?; ensure_no_other_pending(&db, id)?;
        if let Some(rules)=rules{
            if action!="apply"||plan.state!="planned"{return Err("Правила можно изменить только перед новым применением.".into());}
            plan=preview_names(plan,rules)?;persist(&db,&plan)?;
        }
        if action == "rollback" {
            if ["planned", "rolledBack"].contains(&plan.state.as_str()) { return Err("Для этого плана откат не требуется.".into()); }
            if !plan.reverse {
                // Put all currently owned files in their registered temporary paths
                // before publishing any original name. All checks precede any move.
                let mut locations = Vec::new();
                for row in changed(&plan) {
                    let candidates = if plan.state == "staging" { [&row.old_path, &row.temporary] } else { [&row.new_path, &row.temporary] };
                    let found: Vec<String> = candidates.into_iter().filter(|path| matches(path, &row.fingerprint)).cloned().collect();
                    if found.len() != 1 { return Err(format!("Безопасный откат невозможен: {} изменён или недоступен.", row.new_name)); }
                    locations.push(found[0].clone());
                }
                let owned_locations:HashSet<String>=locations.iter().map(|path|key(path)).collect();
                for row in changed(&plan) { if fs::symlink_metadata(&row.old_path).is_ok()&&!owned_locations.contains(&key(&row.old_path)) { return Err(format!("Прежнее имя занято другим файлом: {}. Откат не начат.",row.old_name)); } }
                // Persist rollback intent, including locations, before moving anything.
                // Swap logical direction; temporary paths are already registered.
                let mut cursor = 0;
                for row in plan.rows.iter_mut().filter(|r| r.status != "unchanged") {
                    let was_untouched=plan.state=="staging"&&locations[cursor]==row.old_path;
                    std::mem::swap(&mut row.old_path, &mut row.new_path);
                    std::mem::swap(&mut row.old_name, &mut row.new_name);
                    row.message = locations[cursor].clone(); cursor += 1;
                    if was_untouched { row.status="unchanged".into(); }
                }
                plan.reverse = true; plan.state = "rollbackStaging".into(); persist(&db, &plan)?;
            }
        } else if !["apply", "continue"].contains(&action) { return Err("Неизвестная команда переименования.".into()); }
        if ["completed", "rolledBack"].contains(&plan.state.as_str()) { return Ok(plan); }
        if plan.state == "planned" && action != "apply" { return Err("Новый план запускается только кнопкой применения.".into()); }
        assert_not_processing(app, &plan)?;
        if plan.state == "planned" {
            let index=open_index_connection(app)?;
            let editor = photo_editor::store::connection(app)?;
            if path_to_string(&photo_editor::store::root(app)?.join("editor.sqlite")) != plan.editor_database { return Err("Хранилище проектов сменилось; обновите предпросмотр.".into()); }
            for row in &mut plan.rows {
                let current:Option<String>=index.query_row("SELECT image_id FROM image_paths WHERE path=?1 AND is_current=1",[&row.old_path],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
                if row.image_id.is_some()&&row.image_id!=current{return Err("Идентичность файла в библиотеке изменилась. Обновите предпросмотр.".into());}
                if let Some(id)=&current{if read_indexed_sha256(&index,id)?.is_some_and(|sha|sha!=row.fingerprint.sha256){return Err("Индекс исходника изменился. Обновите предпросмотр.".into());}}
                row.image_id=current;
                let mut statement=editor.prepare("SELECT id FROM editor_projects WHERE source_path=?1").map_err(|e|e.to_string())?;
                row.projects=statement.query_map([&row.old_path],|r|r.get(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
            }
            persist(&db,&plan)?;
        }
        let operation = (|| {
            if plan.state == "rollbackStaging" {
                for row in changed(&plan) {
                    if !matches(&row.temporary, &row.fingerprint) { move_checked(&row.message, &row.temporary, &row.fingerprint)?; }
                }
                plan.state = "publishing".into(); persist(&db, &plan)?;
            }
            run_files(&db, &mut plan)?;
            if plan.state == "metadata" { metadata(app, &db, &mut plan)?; }
            Ok::<(),String>(())
        })();
        if let Err(error) = operation { plan.error = Some(error); persist(&db, &plan)?; }
        plan.completed_count=changed(&plan).filter(|r|matches(&r.new_path,&r.fingerprint)).count();
        plan.remaining_count=changed(&plan).count().saturating_sub(plan.completed_count);
        persist(&db,&plan)?;
        Ok(plan)
    })
}

#[tauri::command]
pub async fn preview_batch_rename(app: tauri::AppHandle, input: PreviewInput) -> Result<Plan, String> {
    tauri::async_runtime::spawn_blocking(move || prepare(&app, input)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn batch_rename_command(app: tauri::AppHandle, id: String, action: String,rules:Option<Rules>) -> Result<Plan, String> {
    tauri::async_runtime::spawn_blocking(move || execute_with_rules(&app, &id, &action,rules)).await.map_err(|e| e.to_string())?
}
// No traversal, hash, journal write or mutation: reuse the captured immutable source set.
fn preview_names(mut plan:Plan,rules:Rules)->Result<Plan,String>{
    validate_rules(&rules)?;if plan.state!="planned"{return Err("Операция уже начата; её правила зафиксированы.".into());}
    if rules.order=="name"{plan.rows.sort_by_key(|r|(r.old_name.to_lowercase(),r.old_path.clone()));}
    if rules.order=="modified"{plan.rows.sort_by(|a,b|a.fingerprint.modified_ns.parse::<u128>().unwrap_or(0).cmp(&b.fingerprint.modified_ns.parse::<u128>().unwrap_or(0)).then_with(||a.old_path.cmp(&b.old_path)));}
    let mut counters=HashMap::<String,u64>::new();
    for (index,row) in plan.rows.iter_mut().enumerate(){
        let parent=Path::new(&row.old_path).parent().ok_or("Нет папки исходника")?;
        let number=if rules.per_folder{let counter=counters.entry(key(&path_to_string(parent))).or_default();let value=*counter;*counter+=1;value}else{index as u64};
        match make_name(&row.old_name,&rules,number){Ok(name)=>{row.new_path=path_to_string(&parent.join(&name));row.status=if row.new_path==row.old_path{"unchanged"}else{"ready"}.into();row.new_name=name;row.message.clear();},Err(error)=>{row.new_path=row.old_path.clone();row.new_name=row.old_name.clone();row.status="error".into();row.message=error;}}
    }
    validate_targets(&mut plan.rows);plan.remaining_count=plan.rows.iter().filter(|r|r.status=="ready").count();plan.rules=rules;Ok(plan)
}
#[tauri::command]
pub async fn preview_batch_rename_names(app:tauri::AppHandle,id:String,rules:Rules)->Result<Plan,String>{
    tauri::async_runtime::spawn_blocking(move||preview_names(load(&journal(&app)?,&id)?,rules)).await.map_err(|e|e.to_string())?
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct HistoryItem { id: String, state: String, created_at_unix: u64, count: usize, error: Option<String> }
#[tauri::command]
pub async fn get_batch_rename_history(app: tauri::AppHandle) -> Result<Vec<HistoryItem>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = journal(&app)?;
        let mut statement = db.prepare("SELECT payload FROM batches WHERE state<>'planned' ORDER BY updated DESC LIMIT 30").map_err(|e| e.to_string())?;
        let values = statement.query_map([], |r| r.get::<_,String>(0)).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
        values.into_iter().map(|v| {let p:Plan=serde_json::from_str(&v).map_err(|e|e.to_string())?;let count=changed(&p).count();Ok(HistoryItem{id:p.id,state:p.state,created_at_unix:p.created_at_unix,count,error:p.error})}).collect::<Result<Vec<_>,String>>()
    }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn get_batch_rename_selection_paths(app:tauri::AppHandle,query_id:String,excluded_paths:Vec<String>)->Result<Vec<String>,String>{
    if excluded_paths.len()>20_000{return Err("Слишком много исключений. Выберите меньший набор.".into());}
    tauri::async_runtime::spawn_blocking(move||{
        let excluded:HashSet<String>=excluded_paths.into_iter().map(|path|key(&path)).collect();
        let paths=search::limited_path_context(&app,&query_id,20_001+excluded.len())?.into_iter().filter(|path|!excluded.contains(&key(path))).collect::<Vec<_>>();
        if paths.len()>20_000{return Err("Выберите не более 20 000 файлов. Выбор не был обрезан.".into());}
        Ok(paths)
    }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn names_preserve_extensions_and_literal_fragments() {
        let mut r = Rules::default(); r.common_name = Some("Товар".into());
        assert_eq!(make_name("DSC.JPG", &r, 1).unwrap(), "Товар_002.JPG");
        // Old journals without separator must retain their byte-for-byte name assembly.
        r.separator=None;r.common_name = None; r.position = "before".into(); r.prefix = "Каталог_".into(); r.suffix = "_a".into();
        assert_eq!(make_name("DSC.JPG", &r, 0).unwrap(), "Каталог_001_DSC_a.JPG");
        r.numbering = false; assert_eq!(make_name("без расширения", &r, 0).unwrap(), "Каталог_без расширения_a");
        assert_eq!(make_name(".gitignore", &r, 0).unwrap(), "Каталог_.gitignore_a");
    }
    #[test] fn joined_fragments_number_whole_name_and_preserve_extension(){
        let mut r=Rules::default();r.prefix="Фото".into();r.common_name=Some("Отпуск".into());r.suffix="Москва".into();
        assert_eq!(make_name("a.jpg",&r,0).unwrap(),"Фото_Отпуск_Москва_001.jpg");
        r.position="before".into();assert_eq!(make_name("a.jpg",&r,0).unwrap(),"001_Фото_Отпуск_Москва.jpg");
        r.numbering=false;assert_eq!(make_name("a.jpg",&r,0).unwrap(),"Фото_Отпуск_Москва.jpg");
        r.prefix.clear();r.suffix.clear();r.numbering=true;r.position="after".into();assert_eq!(make_name("a.jpg",&r,0).unwrap(),"Отпуск_001.jpg");
        r.prefix="Фото".into();r.suffix="Москва".into();r.separator=Some(String::new());assert_eq!(make_name("a.jpg",&r,0).unwrap(),"ФотоОтпускМосква001.jpg");
    }
    #[test] fn windows_names_and_overflow_are_rejected() {
        for name in ["CON.jpg", "LPT1", "COM¹.txt", "NUL", "a.", "a ", "a:b", "a/b", ".."] { assert!(valid_name(name).is_err(), "{name}"); }
        let mut r = Rules::default(); r.start = 999_999_999_999; assert!(make_name("a.jpg", &r, 1).is_err());
        assert!(valid_name(&"я".repeat(256)).is_err());
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self { let path=crate::test_support::temp_dir("smart-photo-batch-rename-test");Self(path.canonicalize().unwrap()) }
        fn plan(&self,names:&[(&str,&str)]) -> Plan {
            let rows=names.iter().enumerate().map(|(i,(old,new))|{
                let path=self.0.join(old);fs::write(&path,format!("owned distinct content {i}")).unwrap();let f=fingerprint(&path).unwrap();
                Row{old_path:path_to_string(&path),new_path:path_to_string(&self.0.join(new)),old_name:(*old).into(),new_name:(*new).into(),status:if old==new{"unchanged"}else{"ready"}.into(),message:String::new(),image_id:None,size_bytes:f.size,modified_at_unix:0,fingerprint:f,temporary:path_to_string(&self.0.join(format!("owned-temp-{i}"))),projects:vec![]}
            }).collect();
            Plan{id:"owned".into(),state:"planned".into(),created_at_unix:0,rules:Rules::default(),rows,error:None,reverse:false,editor_database:String::new(),completed_count:0,remaining_count:names.len()}
        }
        fn db(&self)->Connection{let db=Connection::open(self.0.join("journal.sqlite")).unwrap();db.execute_batch("PRAGMA synchronous=FULL;CREATE TABLE batches(id TEXT PRIMARY KEY,state TEXT,payload TEXT,updated INTEGER)").unwrap();db}
    }
    impl Drop for Fixture{fn drop(&mut self){fs::remove_dir_all(&self.0).unwrap();}}
    #[test] fn cycles_preserve_content_and_file_identity() {
        let f=Fixture::new();let db=f.db();let mut p=f.plan(&[("a.JPG","b.JPG"),("b.JPG","c.JPG"),("c.JPG","a.JPG")]);persist(&db,&p).unwrap();run_files(&db,&mut p).unwrap();
        assert_eq!(p.state,"metadata");for r in &p.rows{assert!(matches(&r.new_path,&r.fingerprint));assert!(!Path::new(&r.temporary).exists());}
    }
    #[test] fn staging_restart_does_not_move_the_same_file_twice() {
        let f=Fixture::new();let db=f.db();let mut p=f.plan(&[("a","x"),("b","y")]);p.state="staging".into();persist(&db,&p).unwrap();
        move_checked(&p.rows[0].old_path,&p.rows[0].temporary,&p.rows[0].fingerprint).unwrap();drop(p);
        let mut restored=load(&db,"owned").unwrap();run_files(&db,&mut restored).unwrap();assert_eq!(restored.state,"metadata");for r in restored.rows{assert!(matches(&r.new_path,&r.fingerprint));}
    }
    #[test] fn publishing_restart_and_external_conflict_do_not_overwrite() {
        let f=Fixture::new();let db=f.db();let mut p=f.plan(&[("a","x"),("b","y")]);
        p.state="publishing".into();persist(&db,&p).unwrap();for row in &p.rows{move_checked(&row.old_path,&row.temporary,&row.fingerprint).unwrap();}
        move_checked(&p.rows[0].temporary,&p.rows[0].new_path,&p.rows[0].fingerprint).unwrap();
        fs::write(&p.rows[1].new_path,"foreign").unwrap();assert!(run_files(&db,&mut p).is_err());assert_eq!(fs::read_to_string(&p.rows[1].new_path).unwrap(),"foreign");assert!(matches(&p.rows[1].temporary,&p.rows[1].fingerprint));
        fs::remove_file(&p.rows[1].new_path).unwrap();let mut restored=load(&db,"owned").unwrap();run_files(&db,&mut restored).unwrap();assert_eq!(restored.state,"metadata");
    }
    #[test] fn stale_preview_is_rejected_before_any_move() {
        let f=Fixture::new();let db=f.db();let mut p=f.plan(&[("a","x"),("b","y")]);fs::write(&p.rows[1].old_path,"changed").unwrap();assert!(run_files(&db,&mut p).is_err());assert_eq!(p.state,"planned");assert!(Path::new(&p.rows[0].old_path).exists());assert!(!Path::new(&p.rows[0].temporary).exists());
    }
    #[test] fn folder_recursion_and_mixed_selection_are_explicit_and_deduplicated() {
        let f=Fixture::new();fs::create_dir(f.0.join("sub")).unwrap();fs::write(f.0.join("one.txt"),"one").unwrap();fs::write(f.0.join("sub/two.txt"),"two").unwrap();
        let input=vec![path_to_string(&f.0),path_to_string(&f.0.join("one.txt"))];assert_eq!(collect(&input,false).unwrap().len(),1);assert_eq!(collect(&input,true).unwrap().len(),2);
    }
    #[test] fn unchanged_rows_and_case_collisions_are_checked() {
        let f=Fixture::new();let mut p=f.plan(&[("a.JPG","same.JPG"),("b.JPG","SAME.JPG")]);validate_targets(&mut p.rows);assert!(p.rows.iter().all(|r|r.status=="conflict"));
        let f=Fixture::new();let db=f.db();let mut p=f.plan(&[("a.JPG","a.JPG"),("b.JPG","c.JPG")]);run_files(&db,&mut p).unwrap();assert!(matches(&p.rows[0].old_path,&p.rows[0].fingerprint));assert!(matches(&p.rows[1].new_path,&p.rows[1].fingerprint));
    }
}
