#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod service_manager;

use service_manager::{ServiceManager, AppMode, ModeConfig};
use std::sync::Mutex;
use tauri::{State, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem};
use std::path::PathBuf;
use chrono::Local;
use std::fs;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;
use std::io::Write;
use std::process::{Command, Child};

struct AppState {
    config: Mutex<ModeConfig>,
    minimize_to_tray: Mutex<bool>,
    auto_backup: Mutex<bool>,
    backup_interval_hours: Mutex<u32>,
    backup_dir: Mutex<String>,
    hub_process: Mutex<Option<Child>>,
}

#[derive(serde::Serialize)]
struct AppSettings {
    minimize_to_tray: bool,
    auto_backup: bool,
    backup_interval_hours: u32,
    backup_dir: String,
    autostart_enabled: bool,
    data_dir: String,
    version: String,
}

#[tauri::command]
fn get_app_settings(state: State<AppState>) -> AppSettings {
    let config = state.config.lock().unwrap();
    let minimize_to_tray = *state.minimize_to_tray.lock().unwrap();
    let auto_backup = *state.auto_backup.lock().unwrap();
    let backup_interval_hours = *state.backup_interval_hours.lock().unwrap();
    let backup_dir = state.backup_dir.lock().unwrap().clone();

    AppSettings {
        minimize_to_tray,
        auto_backup,
        backup_interval_hours,
        backup_dir,
        autostart_enabled: config.autostart_enabled,
        data_dir: config.data_dir.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
fn set_minimize_to_tray(state: State<AppState>, value: bool) -> Result<(), String> {
    {
        let mut setting = state.minimize_to_tray.lock().unwrap();
        *setting = value;
    }
    save_settings_to_file(&state)
}

#[tauri::command]
fn set_auto_backup(state: State<AppState>, enabled: bool) -> Result<(), String> {
    {
        let mut setting = state.auto_backup.lock().unwrap();
        *setting = enabled;
    }
    save_settings_to_file(&state)
}

#[tauri::command]
fn set_backup_interval(state: State<AppState>, hours: u32) -> Result<(), String> {
    {
        let mut setting = state.backup_interval_hours.lock().unwrap();
        *setting = hours;
    }
    save_settings_to_file(&state)
}

#[tauri::command]
fn set_backup_dir(state: State<AppState>, dir: String) -> Result<(), String> {
    {
        let mut setting = state.backup_dir.lock().unwrap();
        *setting = dir;
    }
    save_settings_to_file(&state)
}

#[tauri::command]
fn set_autostart(state: State<AppState>, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let (key, _) = hkcu.create_subkey(path).map_err(|e| e.to_string())?;
        
        if enabled {
            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
            key.set_value("KnowledgeHUB", &exe_path.to_string_lossy().to_string())
                .map_err(|e| e.to_string())?;
        } else {
            key.delete_value("KnowledgeHUB").ok();
        }
    }
    
    {
        let mut config = state.config.lock().unwrap();
        config.autostart_enabled = enabled;
    }
    save_settings_to_file(&state)
}

#[tauri::command]
fn create_backup(state: State<AppState>) -> Result<String, String> {
    let (data_dir, backup_dir) = {
        let config = state.config.lock().unwrap();
        let backup_dir = state.backup_dir.lock().unwrap().clone();
        (PathBuf::from(&config.data_dir), backup_dir)
    };
    
    let backup_path = if backup_dir.is_empty() {
        data_dir.parent().unwrap_or(&data_dir).join("backups")
    } else {
        PathBuf::from(&backup_dir)
    };
    
    fs::create_dir_all(&backup_path).map_err(|e| e.to_string())?;
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let zip_filename = format!("khub_backup_{}.zip", timestamp);
    let zip_path = backup_path.join(&zip_filename);
    
    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    let db_path = data_dir.join("khub.db");
    if db_path.exists() {
        zip.start_file("khub.db", options).map_err(|e| e.to_string())?;
        let db_content = fs::read(&db_path).map_err(|e| e.to_string())?;
        zip.write_all(&db_content).map_err(|e| e.to_string())?;
    }
    
    let settings_path = data_dir.join("settings.json");
    if settings_path.exists() {
        zip.start_file("settings.json", options).map_err(|e| e.to_string())?;
        let settings_content = fs::read(&settings_path).map_err(|e| e.to_string())?;
        zip.write_all(&settings_content).map_err(|e| e.to_string())?;
    }
    
    let app_settings_path = get_settings_path();
    if app_settings_path.exists() {
        zip.start_file("app_settings.json", options).map_err(|e| e.to_string())?;
        let content = fs::read(&app_settings_path).map_err(|e| e.to_string())?;
        zip.write_all(&content).map_err(|e| e.to_string())?;
    }
    
    // Backup embeddings directory
    let embeddings_dir = data_dir.join("embeddings");
    if embeddings_dir.exists() {
        for entry in WalkDir::new(&embeddings_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let relative = path.strip_prefix(&embeddings_dir).unwrap_or(path);
                let zip_name = format!("embeddings/{}", relative.to_string_lossy().replace('\\', "/"));
                zip.start_file(&zip_name, options).map_err(|e| e.to_string())?;
                let content = fs::read(path).map_err(|e| e.to_string())?;
                zip.write_all(&content).map_err(|e| e.to_string())?;
            }
        }
    }
    
    zip.finish().map_err(|e| e.to_string())?;
    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_backup_list(state: State<AppState>) -> Vec<String> {
    let (data_dir, backup_dir) = {
        let config = state.config.lock().unwrap();
        let backup_dir = state.backup_dir.lock().unwrap().clone();
        (PathBuf::from(&config.data_dir), backup_dir)
    };
    
    let backup_path = if backup_dir.is_empty() {
        data_dir.parent().unwrap_or(&data_dir).join("backups")
    } else {
        PathBuf::from(&backup_dir)
    };
    
    if !backup_path.exists() {
        return Vec::new();
    }
    
    let mut backups: Vec<String> = fs::read_dir(&backup_path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .map(|ext| ext == "zip")
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.path().to_str().map(|s| s.to_string()))
        .collect();
    
    backups.sort();
    backups.reverse();
    backups
}

#[tauri::command]
fn restore_backup(state: State<AppState>, path: String) -> Result<serde_json::Value, String> {
    let data_dir = {
        let config = state.config.lock().unwrap();
        PathBuf::from(&config.data_dir)
    };
    
    let zip_file = fs::File::open(&path).map_err(|e| format!("Cannot open backup file: {}", e))?;
    let mut zip = zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip: {}", e))?;
    
    let mut restored = Vec::new();
    let mut merged = Vec::new();
    
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        
        if name.ends_with('/') { continue; }
        
        // Determine target path
        let target_path = if name == "khub.db" {
            data_dir.join("khub.db")
        } else if name == "settings.json" {
            data_dir.join("settings.json")
        } else if name == "app_settings.json" {
            get_settings_path()
        } else if name.starts_with("embeddings/") {
            data_dir.join(&name)
        } else {
            continue;
        };
        
        // Merge strategy for JSON files
        if (name == "settings.json" || name == "app_settings.json") && target_path.exists() {
            let mut current_content = String::new();
            if let Ok(mut current_file) = fs::File::open(&target_path) {
                use std::io::Read;
                let _ = current_file.read_to_string(&mut current_content);
            }
            
            let mut backup_content = String::new();
            use std::io::Read;
            file.read_to_string(&mut backup_content).map_err(|e| e.to_string())?;
            
            if let (Ok(mut current_json), Ok(backup_json)) = (
                serde_json::from_str::<serde_json::Value>(&current_content),
                serde_json::from_str::<serde_json::Value>(&backup_content),
            ) {
                if let (Some(current_obj), Some(backup_obj)) = (current_json.as_object_mut(), backup_json.as_object()) {
                    let mut added_keys = Vec::new();
                    for (key, value) in backup_obj {
                        if !current_obj.contains_key(key) {
                            current_obj.insert(key.clone(), value.clone());
                            added_keys.push(key.clone());
                        }
                    }
                    if !added_keys.is_empty() {
                        let merged_content = serde_json::to_string_pretty(current_obj).map_err(|e| e.to_string())?;
                        fs::write(&target_path, merged_content).map_err(|e| e.to_string())?;
                        merged.push(format!("{} (added: {:?})", name, added_keys));
                    }
                }
            }
            continue;
        }
        
        // Direct copy for db and embeddings
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        
        let mut target_file = fs::File::create(&target_path).map_err(|e| e.to_string())?;
        use std::io::copy;
        copy(&mut file, &mut target_file).map_err(|e| e.to_string())?;
        restored.push(name);
    }
    
    Ok(serde_json::json!({
        "restored": restored,
        "merged": merged,
    }))
}

#[tauri::command]
fn open_logs_folder(state: State<AppState>) -> Result<(), String> {
    let logs_dir = {
        let config = state.config.lock().unwrap();
        PathBuf::from(&config.data_dir).join("logs")
    };
    
    fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(logs_dir.to_string_lossy().to_string())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn get_settings_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join("KnowledgeHub")
        .join("app_settings.json")
}

#[tauri::command]
fn cleanup_old_version() -> serde_json::Value {
    // Old per-user install paths that may exist
    let local_app_data = dirs::data_local_dir().unwrap_or_default();
    let old_paths = vec![
        local_app_data.join("Programs").join("KnowledgeHUB"),
        local_app_data.join("Programs").join("K-HUB"),
    ];

    let current_exe = std::env::current_exe().unwrap_or_default();
    let current_dir = current_exe.parent().unwrap_or(std::path::Path::new(""));

    let mut cleaned = Vec::new();
    let mut errors = Vec::new();

    for old_path in &old_paths {
        if !old_path.exists() {
            continue;
        }
        // Don't delete ourselves
        if old_path == current_dir {
            continue;
        }
        match fs::remove_dir_all(old_path) {
            Ok(()) => cleaned.push(old_path.to_string_lossy().to_string()),
            Err(e) => errors.push(format!("{}: {}", old_path.display(), e)),
        }
    }

    serde_json::json!({
        "cleaned": cleaned,
        "errors": errors,
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistedSettings {
    minimize_to_tray: bool,
    auto_backup: bool,
    backup_interval_hours: u32,
    backup_dir: String,
    autostart_enabled: bool,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            auto_backup: false,
            backup_interval_hours: 24,
            backup_dir: String::new(),
            autostart_enabled: false,
        }
    }
}

fn load_settings_from_file() -> PersistedSettings {
    let path = get_settings_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        PersistedSettings::default()
    }
}

fn save_settings_to_file(state: &State<AppState>) -> Result<(), String> {
    let settings = PersistedSettings {
        minimize_to_tray: *state.minimize_to_tray.lock().unwrap(),
        auto_backup: *state.auto_backup.lock().unwrap(),
        backup_interval_hours: *state.backup_interval_hours.lock().unwrap(),
        backup_dir: state.backup_dir.lock().unwrap().clone(),
        autostart_enabled: state.config.lock().unwrap().autostart_enabled,
    };
    
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn start_hub_service(app_handle: &tauri::AppHandle) -> Option<Child> {
    // 获取 sidecar 路径
    let resource_dir = app_handle.path_resolver().resource_dir().unwrap_or_default();
    let sidecar_path = resource_dir.join("hub-service.exe");
    
    // 如果 sidecar 不存在，尝试从当前目录查找
    let exe_path = if sidecar_path.exists() {
        sidecar_path
    } else {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let current_dir = current_exe.parent().unwrap_or(std::path::Path::new("."));
        current_dir.join("hub-service.exe")
    };
    
    if !exe_path.exists() {
        eprintln!("hub-service.exe not found at {:?}", exe_path);
        return None;
    }
    
    // 启动 hub-service
    match Command::new(&exe_path)
        .env("DATABASE_URL", format!("sqlite:{}/khub.db", 
            dirs::data_dir().unwrap_or_default().join("KnowledgeHub").to_string_lossy()))
        .env("KH_MODE", "standalone")
        .spawn() 
    {
        Ok(child) => {
            println!("hub-service started with PID: {}", child.id());
            Some(child)
        }
        Err(e) => {
            eprintln!("Failed to start hub-service: {}", e);
            None
        }
    }
}

fn main() {
    let data_dir = dirs::data_dir()
        .unwrap_or_default()
        .join("KnowledgeHub")
        .to_string_lossy()
        .to_string();

    // 确保数据目录存在
    fs::create_dir_all(&data_dir).ok();

    let persisted = load_settings_from_file();

    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", "显示窗口"))
        .add_item(CustomMenuItem::new("quit", "退出"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    match id.as_str() {
                        "show" => {
                            let window = app.get_window("main").unwrap();
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                        "quit" => {
                            // 停止 hub-service
                            let state: State<AppState> = app.state();
                            let mut process = state.hub_process.lock().unwrap();
                            if let Some(mut child) = process.take() {
                                child.kill().ok();
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                }
                SystemTrayEvent::LeftClick { .. } => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                _ => {}
            }
        })
        .on_window_event(|event| {
            match event.event() {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let app_handle = event.window().app_handle();
                    let state: State<AppState> = app_handle.state();
                    let minimize_to_tray = *state.minimize_to_tray.lock().unwrap();
                    
                    if minimize_to_tray {
                        event.window().hide().unwrap();
                        api.prevent_close();
                    }
                }
                _ => {}
            }
        })
        .setup(|app| {
            // 启动 hub-service
            let child = start_hub_service(&app.handle());
            
            // 保存进程引用
            let state: State<AppState> = app.state();
            let mut process = state.hub_process.lock().unwrap();
            *process = child;
            
            Ok(())
        })
        .manage(AppState {
            config: Mutex::new(ModeConfig {
                mode: AppMode::Standalone,
                hub_url: None,
                data_dir: data_dir.clone(),
                autostart_enabled: persisted.autostart_enabled,
            }),
            minimize_to_tray: Mutex::new(persisted.minimize_to_tray),
            auto_backup: Mutex::new(persisted.auto_backup),
            backup_interval_hours: Mutex::new(persisted.backup_interval_hours),
            backup_dir: Mutex::new(persisted.backup_dir),
            hub_process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            set_minimize_to_tray,
            set_auto_backup,
            set_backup_interval,
            set_backup_dir,
            set_autostart,
            create_backup,
            get_backup_list,
            open_logs_folder,
            restore_backup,
            cleanup_old_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
