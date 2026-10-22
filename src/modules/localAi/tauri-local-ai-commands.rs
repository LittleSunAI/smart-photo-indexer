#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalRuntimePrepareResultDto {
    runtime_status: String,
    status_text: String,
    error_message: Option<String>,
    executable_path: Option<String>,
    install_was_attempted: bool,
    start_was_attempted: bool,
}

fn ollama_api_ready() -> bool {
    let socket = match std::net::ToSocketAddrs::to_socket_addrs("127.0.0.1:11434")
        .ok()
        .and_then(|mut items| items.next())
    {
        Some(value) => value,
        None => return false,
    };

    let mut stream = match std::net::TcpStream::connect_timeout(&socket, std::time::Duration::from_millis(800)) {
        Ok(value) => value,
        Err(_) => return false,
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(900)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(900)));

    if std::io::Write::write_all(
        &mut stream,
        b"GET /api/version HTTP/1.1\r\nHost: 127.0.0.1:11434\r\nConnection: close\r\n\r\n",
    )
    .is_err()
    {
        return false;
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok()
        && (response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"))
}

fn wait_for_ollama_api(max_attempts: usize, delay_ms: u64) -> bool {
    for _ in 0..max_attempts {
        if ollama_api_ready() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
    false
}

fn managed_ollama_directory() -> Result<PathBuf,String> {
    let root = if let Some(profile) = std::env::var_os("SMART_PHOTO_DATA_DIR") {
        PathBuf::from(profile)
    } else {
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("Не найдена папка локальных компонентов Windows.")?).join("com.smartphotoindexer.app")
    };
    if !root.is_absolute() { return Err("Папка локальных компонентов должна быть абсолютной.".into()); }
    Ok(root.join("runtime").join("ollama-0.40.3"))
}

fn candidate_ollama_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(directory) = managed_ollama_directory() {
        paths.push(directory.join("ollama.exe"));
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(&local_app_data).join("Programs").join("Ollama").join("bin").join("ollama.exe"));
        paths.push(PathBuf::from(&local_app_data).join("Programs").join("Ollama").join("ollama.exe"));
        paths.push(PathBuf::from(&local_app_data).join("Ollama").join("ollama.exe"));
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        paths.push(PathBuf::from(&user_profile).join("AppData").join("Local").join("Programs").join("Ollama").join("ollama.exe"));
    }

    if let Ok(program_files) = std::env::var("ProgramFiles") {
        paths.push(PathBuf::from(&program_files).join("Ollama").join("ollama.exe"));
        paths.push(PathBuf::from(&program_files).join("Ollama").join("Ollama.exe"));
    }

    paths
}

fn find_ollama_executable() -> Option<PathBuf> {
    for path in candidate_ollama_paths() {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut command=std::process::Command::new("where");
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let output = command.arg("ollama")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn start_ollama_runtime(executable_path: &Path) -> Result<(), String> {
    let mut command=std::process::Command::new(executable_path);
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    command.arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not start Ollama runtime: {error}"))
}

#[tauri::command]
async fn prepare_ollama_runtime(endpoint:Option<String>) -> Result<LocalRuntimePrepareResultDto, String> {
    let url=provider_transport::local_endpoint(endpoint.as_deref())?;
    let client=reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(2)).build().map_err(|_|"Не удалось проверить локальный сервис.")?;
    if client.get(url.join("/api/version").map_err(|e|e.to_string())?).send().await.is_ok_and(|response|response.status().is_success()){
        return Ok(LocalRuntimePrepareResultDto{runtime_status:"running".into(),status_text:"Локальный сервис готов.".into(),error_message:None,executable_path:None,install_was_attempted:false,start_was_attempted:false});
    }
    if url.port_or_known_default()!=Some(11434) || ![Some("127.0.0.1"),Some("localhost")].contains(&url.host_str()) || !["","/"].contains(&url.path()) {
        return Err("Сервис по выбранному локальному адресу не отвечает. Проверьте адрес и запустите свой исполнитель.".into());
    }
    tauri::async_runtime::spawn_blocking(|| {
        static START:std::sync::Mutex<()>=std::sync::Mutex::new(());
        let _guard=START.lock().map_err(|_|"Не удалось подготовить локальный сервис.".to_string())?;
        prepare_ollama_runtime_blocking()
    }).await.map_err(|e|e.to_string())?
}
fn prepare_ollama_runtime_blocking() -> Result<LocalRuntimePrepareResultDto, String> {
    if ollama_api_ready() {
        return Ok(LocalRuntimePrepareResultDto {
            runtime_status: "running".to_string(),
            status_text: "Ollama уже запущена.".to_string(),
            error_message: None,
            executable_path: find_ollama_executable().map(|path| path.to_string_lossy().to_string()),
            install_was_attempted: false,
            start_was_attempted: false,
        });
    }

    if let Some(executable_path) = find_ollama_executable() {
        start_ollama_runtime(&executable_path)?;

        if wait_for_ollama_api(45, 1000) {
            return Ok(LocalRuntimePrepareResultDto {
                runtime_status: "running".to_string(),
                status_text: "Ollama найдена и запущена.".to_string(),
                error_message: None,
                executable_path: Some(executable_path.to_string_lossy().to_string()),
                install_was_attempted: false,
                start_was_attempted: true,
            });
        }

        return Ok(LocalRuntimePrepareResultDto {
            runtime_status: "notRunning".to_string(),
            status_text: "Ollama найдена, но локальный API не поднялся.".to_string(),
            error_message: Some("Ollama executable was started, but /api/version did not answer in time.".to_string()),
            executable_path: Some(executable_path.to_string_lossy().to_string()),
            install_was_attempted: false,
            start_was_attempted: true,
        });
    }

    Err("Локальный исполнитель Ollama не установлен. Установите его; приложение затем подготовит сервис автоматически.".into())
}
