use std::sync::Mutex;

static CURRENT_MODE: Mutex<Option<String>> = Mutex::new(None);

pub fn get_current_mode() -> String {
    let mode = CURRENT_MODE.lock().unwrap();
    mode.clone().unwrap_or_else(|| "standalone".to_string())
}

pub fn set_mode(mode: &str) -> Result<(), String> {
    match mode {
        "standalone" | "hub" | "client" => {
            let mut current = CURRENT_MODE.lock().map_err(|e| e.to_string())?;
            *current = Some(mode.to_string());
            Ok(())
        }
        _ => Err(format!("Invalid mode: {}", mode)),
    }
}
