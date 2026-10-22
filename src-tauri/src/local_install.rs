//! Explicit installation only. Opening a folder or choosing a provider never downloads packages.
use crate::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tauri::Emitter;

// Official standalone CLI distribution: the GUI Setup always starts its app
// after installation, even with /VERYSILENT. Never launch that installer here.
const OLLAMA_URL: &str = "https://github.com/ollama/ollama/releases/download/v0.40.3/ollama-windows-amd64.zip";
const OLLAMA_SHA: &str = "e5ecff02dfc7cae8ddf6d6b7a7ac5f727ffeedf4491f44ae75ff5b0820dbbbd5";
const OLLAMA_BYTES: u64 = 1_468_053_171;
#[derive(Default)]
pub struct LocalInstaller { active: Mutex<Option<Arc<AtomicBool>>> }
#[tauri::command]
pub async fn preview_local_installation(app:tauri::AppHandle,model:String,endpoint:Option<String>)->Result<Value,String>{tauri::async_runtime::spawn_blocking(move||{
 let url=provider_transport::local_endpoint(endpoint.as_deref())?;let catalog:Value=serde_json::from_str(include_str!("../../src/shared/ai/model-catalog.json")).map_err(|e|e.to_string())?;let selected=catalog["local"].as_array().and_then(|rows|rows.iter().find(|row|row["id"]==model)).ok_or("Неизвестная локальная модель")?;let runtime=find_ollama_executable();let runtime_path=runtime.clone().unwrap_or(managed_ollama_directory()?.join("ollama.exe"));let models=std::env::var_os("OLLAMA_MODELS").map(PathBuf::from).unwrap_or_else(||PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join(".ollama/models"));let default=url.port_or_known_default()==Some(11434)&&[Some("localhost"),Some("127.0.0.1")].contains(&url.host_str())&&["","/"].contains(&url.path());Ok(json!({"runtimeInstalled":runtime.is_some(),"runtimePath":path_to_string(&runtime_path),"modelsPath":path_to_string(&models),"installerCache":path_to_string(&application_data_directory(&app)?.join("installers")),"runtimeDownloadBytes":if runtime.is_some(){0}else{OLLAMA_BYTES},"modelDownloadBytes":selected["downloadBytes"],"allowed":default,"message":if default{"Ollama и выбранная модель подготовятся автоматически в фоне. Отдельное окно Ollama и ввод команд не нужны."}else{"Для нестандартного адреса установка не запускается. Проверьте адрес и существующий сервис."}}))
}).await.map_err(|e|e.to_string())?}

fn progress(app: &tauri::AppHandle, model: &str, endpoint: &str, message: &str, completed: u64, total: u64, cancellable: bool) {
    let _ = app.emit("local-install-progress", json!({"model":model,"endpoint":endpoint,"message":message,"completed":completed,"total":total,"cancellable":cancellable}));
}
fn cancelled(flag: &AtomicBool) -> Result<(), String> {
    if flag.load(Ordering::Acquire) { Err("Установка отменена. Скачанные компоненты сохранены; локальный сервис может завершить текущий слой модели.".into()) } else { Ok(()) }
}
async fn until_cancelled(flag: Arc<AtomicBool>) {
    while !flag.load(Ordering::Acquire) { tokio::time::sleep(Duration::from_millis(200)).await; }
}
use std::time::Duration;

#[cfg(windows)]
fn other_ollama_process() -> Result<bool, String> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var_os("SystemRoot").ok_or("Не удалось определить системную папку Windows.")?;
    let output = std::process::Command::new(PathBuf::from(root).join("System32/tasklist.exe"))
        .creation_flags(0x08000000).args(["/FI","IMAGENAME eq ollama.exe","/FO","CSV","/NH"])
        .stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().map_err(|_|"Не удалось проверить запущенный локальный сервис.".to_string())?;
    if !output.status.success() { return Err("Не удалось проверить запущенный локальный сервис.".into()); }
    Ok(String::from_utf8_lossy(&output.stdout).to_ascii_lowercase().contains("\"ollama.exe\""))
}
#[cfg(not(windows))]
fn other_ollama_process() -> Result<bool, String> { Ok(false) }

#[cfg(windows)]
fn check_space(path: &Path, required: u64) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut existing = path;
    while !existing.exists() { existing = existing.parent().ok_or("Не удалось определить диск для установки.")?; }
    let wide: Vec<u16> = existing.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0;
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) } == 0 { return Err("Не удалось проверить свободное место на диске.".into()); }
    if free < required { return Err(format!("Недостаточно места на диске {}: требуется не менее {:.1} ГБ, доступно {:.1} ГБ.", existing.display(), required as f64 / 1e9, free as f64 / 1e9)); }
    Ok(())
}
#[cfg(not(windows))]
fn check_space(_path: &Path, _required: u64) -> Result<(), String> { Err("Установка локального сервиса поддерживается в Windows.".into()) }

fn verified_installer(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else { return false; };
    if file.metadata().map(|m|m.len()).ok() != Some(OLLAMA_BYTES) { return false; }
    let mut reader = BufReader::new(file);
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 256 * 1024];
    loop { match reader.read(&mut buffer) { Ok(0) => break, Ok(n) => hash.update(&buffer[..n]), Err(_) => return false } }
    format!("{:x}", hash.finalize()) == OLLAMA_SHA
}

#[cfg(windows)]
fn unpack_runtime(archive: PathBuf, runtime: PathBuf) -> Result<(),String> {
    let parent=runtime.parent().ok_or("Не найдена папка локального сервиса")?;
    fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    let parent=fs::canonicalize(parent).map_err(|e|e.to_string())?;
    let destination=parent.join(runtime.file_name().ok_or("Неверное имя папки сервиса")?);
    let staging=parent.join(format!(".ollama-{}-{}.part",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()));
    fs::create_dir(&staging).map_err(|e|e.to_string())?;
    let result=(||{
        // Native extraction handles Windows extended/long paths. Framework
        // ZipFile.ExtractToDirectory fails on directory entries under \\?\.
        // No PowerShell, archive utility, console or GUI is required.
        let mut zip=zip::ZipArchive::new(fs::File::open(&archive).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let mut total=0u64;
        for index in 0..zip.len(){
            let entry=zip.by_index(index).map_err(|e|e.to_string())?;
            if entry.enclosed_name().is_none() || entry.name().map_err(|e|e.to_string())?.contains(':') || entry.is_symlink(){return Err("Недопустимый путь в пакете локального сервиса.".into());}
            total=total.checked_add(entry.size()).ok_or("Неверный размер пакета")?;
            if total>8_000_000_000{return Err("Превышен размер распакованного пакета.".into());}
        }
        for index in 0..zip.len(){
            let mut entry=zip.by_index(index).map_err(|e|e.to_string())?;
            let target=staging.join(entry.enclosed_name().ok_or("Недопустимый путь в пакете")?);
            if entry.is_dir(){fs::create_dir_all(&target).map_err(|e|e.to_string())?;continue;}
            fs::create_dir_all(target.parent().ok_or("Неверный путь файла пакета")?).map_err(|e|e.to_string())?;
            let mut output=fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(|e|e.to_string())?;
            let expected=entry.size();
            let copied=std::io::copy(&mut entry.by_ref().take(expected+1),&mut output).map_err(|e|e.to_string())?;
            if copied!=expected{return Err("Размер распакованного файла не совпадает.".into());}
            output.sync_all().map_err(|e|e.to_string())?;
        }
        if !staging.join("ollama.exe").is_file(){return Err("В проверенном пакете не найден исполнитель Ollama.".into());}
        fs::write(staging.join("installation.sha256"),OLLAMA_SHA).map_err(|e|e.to_string())?;
        if destination.exists(){return Err("Папка локального сервиса уже существует. Повторная установка не выполнялась.".into());}
        fs::rename(&staging,&destination).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if result.is_err() && staging.parent()==Some(parent.as_path()) {let _=fs::remove_dir_all(&staging);}
    result
}
#[cfg(not(windows))]
fn unpack_runtime(_archive:PathBuf,_runtime:PathBuf)->Result<(),String>{Err("Установка локального сервиса поддерживается в Windows.".into())}
async fn bootstrap(app: &tauri::AppHandle, model: &str, endpoint: &str, cancel: Arc<AtomicBool>) -> Result<(), String> {
    if std::env::consts::ARCH != "x86_64" { return Err("Этот тестовый пакет локального сервиса предназначен для Windows x64.".into()); }
    let cache = application_data_directory(app)?.join("installers");
    fs::create_dir_all(&cache).map_err(|_|"Не удалось создать папку установочных компонентов.".to_string())?;
    let installer = cache.join("Ollama-0.40.3-windows-amd64.zip");
    let runtime = managed_ollama_directory()?;
    check_space(&cache, OLLAMA_BYTES + 512_000_000)?;
    check_space(&runtime,8_000_000_000+512_000_000)?;
    let verify_path = installer.clone();
    let verified = tauri::async_runtime::spawn_blocking(move || verified_installer(&verify_path)).await.map_err(|e|e.to_string())?;
    if !verified {
        let part = cache.join("Ollama-0.40.3-windows-amd64.part");
        progress(app,model,endpoint,"Скачивание локального сервиса Ollama…",0,OLLAMA_BYTES,true);
        let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::limited(5)).connect_timeout(Duration::from_secs(20)).read_timeout(Duration::from_secs(90)).timeout(Duration::from_secs(1800)).build().map_err(|e|e.to_string())?;
        let transfer = async {
            let mut response = client.get(OLLAMA_URL).send().await.map_err(|_|"Не удалось скачать Ollama. Проверьте интернет и повторите установку.".to_string())?;
            if !response.status().is_success() { return Err(format!("Сервер установочного пакета ответил {}.",response.status())); }
            let mut file = fs::File::create(&part).map_err(|_|"Не удалось записать установочный пакет.".to_string())?;
            let mut total = 0u64;
            let mut notified = 0u64;
            use std::io::Write;
            while let Some(bytes) = response.chunk().await.map_err(|_|"Скачивание сервиса прервано. Повторите установку.".to_string())? {
                cancelled(&cancel)?;
                total += bytes.len() as u64;
                if total > OLLAMA_BYTES { return Err("Установочный пакет превышает ожидаемый размер.".into()); }
                file.write_all(&bytes).map_err(|_|"Не удалось записать пакет. Проверьте свободное место.".to_string())?;
                if total >= notified + 4 * 1024 * 1024 { progress(app,model,endpoint,"Скачивание локального сервиса Ollama…",total,OLLAMA_BYTES,true); notified = total; }
            }
            file.sync_all().map_err(|e|e.to_string())?;
            drop(file);
            Ok::<(),String>(())
        };
        tokio::select! { result = transfer => result?, _ = until_cancelled(cancel.clone()) => return cancelled(&cancel) }
        progress(app,model,endpoint,"Проверяю целостность установочного пакета…",0,0,true);
        let verify_path = part.clone();
        if !tauri::async_runtime::spawn_blocking(move || verified_installer(&verify_path)).await.map_err(|e|e.to_string())? { return Err("Проверка SHA-256 установочного пакета не пройдена; запуск запрещён.".into()); }
        fs::rename(&part,&installer).map_err(|e|e.to_string())?;
    }
    cancelled(&cancel)?;
    // Never run an installer over an Ollama found or started by another application while downloading.
    if find_ollama_executable().is_some() || ollama_api_ready() { return Ok(()); }
    if tauri::async_runtime::spawn_blocking(other_ollama_process).await.map_err(|e|e.to_string())?? {
        return Err("Ollama уже запущена, но не отвечает на стандартном адресе. Проверьте её настройки; существующий процесс не будет заменён установщиком.".into());
    }
    progress(app,model,endpoint,"Подготовка фонового компонента Ollama…",0,0,false);
    tauri::async_runtime::spawn_blocking(move ||unpack_runtime(installer,runtime)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub async fn cancel_local_model_install(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<LocalInstaller>();
    if let Some(cancel) = state.active.lock().map_err(|_|"Состояние установки недоступно.")?.as_ref() { cancel.store(true,Ordering::Release); }
    Ok(())
}
#[tauri::command]
pub async fn install_local_model(app: tauri::AppHandle, model: String, endpoint: Option<String>) -> Result<Value,String> {
    let url = provider_transport::local_endpoint(endpoint.as_deref())?;
    let endpoint = url.to_string();
    let catalog: Value = serde_json::from_str(include_str!("../../src/shared/ai/model-catalog.json")).map_err(|e|e.to_string())?;
    let selected = catalog["local"].as_array().and_then(|rows|rows.iter().find(|row|row["id"]==model)).ok_or("Выбрана неизвестная локальная модель.")?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<LocalInstaller>();
        let mut active = state.active.lock().map_err(|_|"Состояние установки недоступно.")?;
        if active.is_some() { return Err("Установка другой локальной модели уже выполняется.".into()); }
        *active = Some(cancel.clone());
    }
    struct Guard(tauri::AppHandle);
    impl Drop for Guard { fn drop(&mut self) { if let Ok(mut active) = self.0.state::<LocalInstaller>().active.lock() { *active = None; } } }
    let _guard = Guard(app.clone());
    let operation = async {
        progress(&app,&model,&endpoint,"Проверяю локальные компоненты…",0,0,true);
        let ready = prepare_ollama_runtime(Some(endpoint.clone())).await;
        if ready.is_err() {
            let default_endpoint = url.port_or_known_default()==Some(11434) && [Some("localhost"),Some("127.0.0.1")].contains(&url.host_str()) && ["","/"].contains(&url.path());
            if !default_endpoint || find_ollama_executable().is_some() { return Err(ready.unwrap_err()); }
            bootstrap(&app,&model,&endpoint,cancel.clone()).await?;
            let prepared = prepare_ollama_runtime(Some(endpoint.clone())).await?;
            if let Some(error) = prepared.error_message { return Err(error); }
        } else if let Some(error) = ready.unwrap().error_message { return Err(error); }
        cancelled(&cancel)?;
        if let Ok(status) = provider_transport::get_ai_provider_status(app.clone(),"local".into(),model.clone(),Some(endpoint.clone()),None).await {
            if status["ready"]==true { return Ok(status); }
        }
        let models = std::env::var_os("OLLAMA_MODELS").map(PathBuf::from).unwrap_or_else(||PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join(".ollama/models"));
        check_space(&models,selected["downloadBytes"].as_u64().unwrap_or(5_000_000_000)+512_000_000)?;
        progress(&app,&model,&endpoint,"Скачивание выбранной модели…",0,0,true);
        tokio::select! { result = provider_transport::download_local_model(app.clone(),model.clone(),Some(endpoint.clone())) => result, _ = until_cancelled(cancel.clone()) => cancelled(&cancel).and_then(|_|Err("Установка отменена.".into())) }
    };
    operation.await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn untrusted_installer_never_passes() { assert!(!verified_installer(Path::new("Cargo.toml"))); assert!(!verified_installer(Path::new("missing-installer.exe"))); }
    #[test] fn cancellation_is_explicit() { let flag=AtomicBool::new(false);assert!(cancelled(&flag).is_ok());flag.store(true,Ordering::Release);assert!(cancelled(&flag).is_err()); }
    #[cfg(windows)]
    #[test]
    #[ignore = "Requires the exact downloaded and SHA-256 verified official archive"]
    fn official_runtime_archive_unpacks_with_native_long_paths() {
        let archive=PathBuf::from(std::env::var_os("SMART_PHOTO_TEST_OLLAMA_ARCHIVE").expect("Explicit official archive fixture required"));
        assert!(verified_installer(&archive));
        let root=crate::test_support::temp_dir("official-runtime-unpack");
        let runtime=root.join("long-path".repeat(22)).join("runtime");
        unpack_runtime(archive,runtime.clone()).unwrap();
        assert!(runtime.join("ollama.exe").is_file());
        assert!(runtime.join("lib/ollama/ggml-base.dll").is_file());
        assert!(runtime.join("lib/ollama/cuda_v12/cublas64_12.dll").is_file());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test] fn runtime_unpack_publishes_complete_directory_and_rejects_escape() {
        let root=crate::test_support::temp_dir("runtime-unpack");
        let archive=root.join("runtime.zip");
        let make=|entry:&str|{
            let mut zip=zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            let options=zip::write::SimpleFileOptions::default();
            zip.add_directory("lib/ollama/",options).unwrap();
            zip.start_file("lib/ollama/runtime.dll",options).unwrap();
            std::io::Write::write_all(&mut zip,b"nested fixture").unwrap();
            zip.start_file(entry,options).unwrap();
            std::io::Write::write_all(&mut zip,b"owned fixture").unwrap();
            zip.finish().unwrap();
        };
        make("ollama.exe");let runtime=root.join("complete");unpack_runtime(archive.clone(),runtime.clone()).unwrap();
        assert_eq!(fs::read(runtime.join("ollama.exe")).unwrap(),b"owned fixture");
        assert_eq!(fs::read_to_string(runtime.join("installation.sha256")).unwrap(),OLLAMA_SHA);
        assert_eq!(fs::read(runtime.join("lib/ollama/runtime.dll")).unwrap(),b"nested fixture");
        let long_runtime=root.join("long-path".repeat(22)).join("runtime");
        unpack_runtime(archive.clone(),long_runtime.clone()).unwrap();
        assert_eq!(fs::read(long_runtime.join("lib/ollama/runtime.dll")).unwrap(),b"nested fixture");
        assert!(unpack_runtime(archive.clone(),runtime.clone()).is_err());assert_eq!(fs::read(runtime.join("ollama.exe")).unwrap(),b"owned fixture");
        fs::remove_file(&archive).unwrap();make("../outside.exe");assert!(unpack_runtime(archive,root.join("rejected")).is_err());assert!(!root.join("outside.exe").exists());assert!(!root.join("rejected").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
