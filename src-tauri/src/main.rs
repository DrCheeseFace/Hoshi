#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader, Cursor, Write};
use std::process::{Command, Stdio, Child};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::env::consts::{ARCH, OS};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

use sysinfo::System;
use tauri::{Emitter, Manager, State, Window};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    katago_process: Mutex<Option<Child>>,
    cold_boot_sgf: Mutex<Option<String>>,
    abort_download: AtomicBool,
}

#[tauri::command]
async fn check_for_updates() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

    // Fetch the latest release data from KataGo's GitHub
    let res = client.get("https://api.github.com/repos/lightvector/KataGo/releases/latest")
        .header("User-Agent", "Hoshi-Go-App")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let json: serde_json::Value = res.json().await.map_err(|e| format!("JSON Parse error: {}", e))?;

    // Extract the version tag and the list of available asset downloads
    let version_tag = json["tag_name"].as_str().unwrap_or("Unknown").to_string();

    let mut download_urls = Vec::new();
    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            if let Some(url) = asset["browser_download_url"].as_str() {
                download_urls.push(url.to_string());
            }
        }
    }

    Ok(serde_json::json!({
        "latest_version": version_tag,
        "assets": download_urls
    }))
}

#[tauri::command]
fn get_system_profile() -> serde_json::Value {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_brand = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();
    let memory_gb = (sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0).round() as u64;

    // Attempt to detect the GPU via native OS commands
    let mut gpu_info = String::from("Unknown GPU");

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("wmic")
            .args(&["path", "win32_VideoController", "get", "name"])
            .creation_flags(0x08000000)
            .output() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            gpu_info = out_str.lines().skip(1).collect::<Vec<&str>>().join(" ").trim().to_string();
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("system_profiler").args(&["SPDisplaysDataType"]).output() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = out_str.lines().find(|l| l.contains("Chipset Model:")) {
                gpu_info = line.replace("Chipset Model:", "").trim().to_string();
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lspci").output() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = out_str.lines().find(|l| l.contains("VGA compatible controller") || l.contains("3D controller")) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 2 { gpu_info = parts[2].trim().to_string(); }
            }
        }
    }

    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpu": cpu_brand,
        "ram_gb": memory_gb,
        "gpu": gpu_info
    })
}

// Lets the user pick an installation folder
#[tauri::command]
async fn native_folder_dialog(app: tauri::AppHandle, title: String) -> Result<Option<String>, String> {
    let path = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_title(title).blocking_pick_folder()
    }).await.map_err(|e| e.to_string())?;
    Ok(path.map(|p| p.to_string()))
}

// OS Detection, Download, and Extraction
#[tauri::command]
fn get_app_base_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""))
            .to_string_lossy()
            .to_string()
    }

    #[cfg(target_family = "unix")]
    {
        // Safely point Mac and Linux to a hidden folder in the User's Home directory
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
        format!("{}/.hoshi", home)
    }
}

#[tauri::command]
fn resolve_destination(base_path: String) -> String {
    let mut path = std::path::PathBuf::from(base_path);
    path.push("KataGo");
    path.to_string_lossy().to_string()
}

// Create the new abort command right above the download command
#[tauri::command]
fn cancel_download(state: tauri::State<'_, AppState>) {
    state.abort_download.store(true, Ordering::SeqCst);
}

// OS Detection, Download, and Extraction (Now with Progress Streaming & Cancellation)
#[tauri::command] // v--- ADD STATE PARAMETER HERE
async fn download_katago(window: tauri::Window, state: tauri::State<'_, AppState>, target_folder: String, engine_url: String, network_url: String) -> Result<serde_json::Value, String> {
    state.abort_download.store(false, Ordering::SeqCst); // Reset flag on start

    let target_dir = std::path::PathBuf::from(&target_folder);

    // Create the target directory if it doesn't exist yet
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create folder: {}", e))?;

    let _exe_name_str = if engine_url.contains("windows") { "katago.exe".to_string() } else { "katago".to_string() };
    let client = reqwest::Client::new();

    // --- 1. STREAM ENGINE ZIP ---
    let mut eng_resp = client.get(&engine_url).send().await.map_err(|e| format!("Engine network error: {}", e))?
        .error_for_status().map_err(|e| format!("Engine HTTP error: {}", e))?;

    let eng_total = eng_resp.content_length().unwrap_or(0);
    let mut eng_bytes = Vec::new();
    let start_time = std::time::Instant::now();

    while let Some(chunk) = eng_resp.chunk().await.map_err(|e| format!("Chunk error: {}", e))? {
        // ABORT CHECK: If user clicked cancel, delete folder and quit
        if state.abort_download.load(Ordering::SeqCst) {
            let _ = std::fs::remove_dir_all(&target_dir);
            return Err("Download cancelled by user".to_string());
        }

        eng_bytes.extend_from_slice(&chunk);
        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { (eng_bytes.len() as f64 / 1_048_576.0) / elapsed } else { 0.0 };

        // Emit progress to the frontend UI
        let _ = window.emit("download-progress", serde_json::json!({
            "file": "KataGo Engine",
            "downloaded": eng_bytes.len(),
            "total": eng_total,
            "speed": speed
        }));
    }

    let target_dir_clone = target_dir.clone();
    let extract_exe_name = _exe_name_str.clone();

    // Extract the Zip
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let reader = std::io::Cursor::new(eng_bytes);
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Invalid zip archive: {}", e))?;
        archive.extract(&target_dir_clone).map_err(|e| format!("Extraction failed: {}", e))?;

        #[cfg(target_family = "unix")]
        {
            let exe_path = target_dir_clone.join(&extract_exe_name);
            if let Ok(mut perms) = std::fs::metadata(&exe_path).map(|m| m.permissions()) {
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&exe_path, perms);
            }
        }
        Ok(())
    }).await.map_err(|e| e.to_string())??;

    // --- 2. STREAM NEURAL NETWORK ---
    let mut net_resp = client.get(&network_url).send().await.map_err(|e| format!("Network download error: {}", e))?
        .error_for_status().map_err(|e| format!("Network HTTP error: {}", e))?;

    let net_total = net_resp.content_length().unwrap_or(0);
    let mut net_bytes = Vec::new();
    let net_start_time = std::time::Instant::now();

    while let Some(chunk) = net_resp.chunk().await.map_err(|e| format!("Chunk error: {}", e))? {
        // ABORT CHECK: If user clicked cancel, delete folder and quit
        if state.abort_download.load(Ordering::SeqCst) {
            let _ = std::fs::remove_dir_all(&target_dir);
            return Err("Download cancelled by user".to_string());
        }

        net_bytes.extend_from_slice(&chunk);
        let elapsed = net_start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { (net_bytes.len() as f64 / 1_048_576.0) / elapsed } else { 0.0 };

        let _ = window.emit("download-progress", serde_json::json!({
            "file": "Neural Network",
            "downloaded": net_bytes.len(),
            "total": net_total,
            "speed": speed
        }));
    }

    let net_path = target_dir.join("default_model.bin.gz");
    std::fs::write(&net_path, net_bytes).map_err(|e| format!("Failed to save network: {}", e))?;

    Ok(serde_json::json!({
        "exePath": target_dir.join(_exe_name_str).to_string_lossy(),
        "modelPath": net_path.to_string_lossy(),
        "cfgPath": target_dir.join("analysis_example.cfg").to_string_lossy()
    }))
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_cold_boot_sgf(state: State<'_, AppState>) -> Option<String> {
    state.cold_boot_sgf.lock().unwrap().take()
}

#[tauri::command]
fn get_default_engine_paths() -> serde_json::Value {
    let base_path = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""))
        .to_path_buf();

    serde_json::json!({
        "exePath": base_path.join("KataGo").join("katago.exe").to_string_lossy(),
        "modelPath": base_path.join("KataGo").join("model.bin.gz").to_string_lossy(),
        "cfgPath": base_path.join("KataGo").join("analysis.cfg").to_string_lossy()
    })
}

// ASYNC DIALOGS FIX: Uses spawn_blocking so it physically cannot freeze the UI thread
#[tauri::command]
async fn native_open_dialog(app: tauri::AppHandle, title: String, f_name: String, f_ext: String) -> Result<Option<String>, String> {
    let path = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().set_title(title).add_filter(f_name, &[&f_ext]).blocking_pick_file()
    }).await.map_err(|e| e.to_string())?;
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command]
async fn native_save_dialog(app: tauri::AppHandle, title: String, def_path: String, f_name: String, f_ext: String) -> Result<Option<String>, String> {
    let path = tauri::async_runtime::spawn_blocking(move || {
        let mut builder = app.dialog().file().set_title(title).add_filter(f_name, &[&f_ext]);
        if !def_path.is_empty() { builder = builder.set_file_name(def_path); }
        builder.blocking_save_file()
    }).await.map_err(|e| e.to_string())?;
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command]
fn start_katago(window: Window, state: State<'_, AppState>, exe_path: String, args: Vec<String>) -> Result<(), String> {
    // Strip any accidental quotes from OS copy-pasting
    let clean_exe = exe_path.trim_matches(|c| c == '"' || c == '\'');
    let exe_path_buf = std::path::PathBuf::from(clean_exe);

    let working_dir = exe_path_buf.parent().ok_or("Invalid path")?;

    #[cfg(target_os = "windows")]
    let mut child = Command::new(clean_exe)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("KataGo Spawn Error: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(clean_exe)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("KataGo Spawn Error: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let win_out = window.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line { let _ = win_out.emit("katago-stdout", l); }
        }
    });

    let stderr = child.stderr.take().unwrap();
    let win_err = window.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line { let _ = win_err.emit("katago-stderr", l); }
        }
    });

    *state.katago_process.lock().unwrap() = Some(child);
    Ok(())
}

#[tauri::command]
fn stop_katago(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(mut child) = state.katago_process.lock().unwrap().take() { let _ = child.kill(); }
    Ok(())
}

#[tauri::command]
fn send_katago_command(state: State<'_, AppState>, command: String) -> Result<(), String> {
    if let Some(child) = state.katago_process.lock().unwrap().as_mut() {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(command.as_bytes()).map_err(|e| e.to_string())?;
            stdin.write_all(b"\n").map_err(|e| e.to_string())?;
            stdin.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn file_exists(path: String) -> bool {
    // Strip accidental quotes just in case
    let clean_path = path.trim_matches(|c| c == '"' || c == '\'');
    std::path::Path::new(clean_path).exists()
}

fn main() {
    tauri::Builder::default()
        .manage(AppState { katago_process: Mutex::new(None), cold_boot_sgf: Mutex::new(None), abort_download: AtomicBool::new(false) })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if let Some(arg) = args.iter().find(|a| a.ends_with(".sgf")) {
                if let Ok(c) = std::fs::read_to_string(arg) { let _ = app.emit("sgf-data", c); }
            }
        }))
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            if let Some(arg) = args.iter().find(|a| a.ends_with(".sgf")) {
                if let Ok(c) = std::fs::read_to_string(arg) {
                    let state = app.state::<AppState>();
                    *state.cold_boot_sgf.lock().unwrap() = Some(c);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_file, write_file, get_cold_boot_sgf, get_default_engine_paths,
            native_open_dialog, native_save_dialog, start_katago, stop_katago, send_katago_command,
            file_exists,
            download_katago, check_for_updates, get_system_profile, native_folder_dialog,
            get_app_base_dir, resolve_destination, cancel_download
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                // 1. Fetch state and take the child process in one go.
                // The MutexGuard drops cleanly and instantly at the semicolon.
                let state = app_handle.state::<AppState>();
                let child_opt = state.katago_process.lock().unwrap().take();

                // 2. Safely kill the process now that the lock is gone.
                if let Some(mut child) = child_opt {
                    let _ = child.kill();
                }
            }
            _ => {}
        });
}
