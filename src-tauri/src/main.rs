#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio, Child};
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use std::thread;
use tauri::{Emitter, Manager, State, Window};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    katago_process: Mutex<Option<Child>>,
    cold_boot_sgf: Mutex<Option<String>>,
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

    let mut child = Command::new(clean_exe)
        .args(args)
        .current_dir(working_dir) // Crucial for KataGo to find its weights
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
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
        .manage(AppState { katago_process: Mutex::new(None), cold_boot_sgf: Mutex::new(None) })
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
            file_exists
        ])
        .run(tauri::generate_context!())
        .expect("failed to run");
}
