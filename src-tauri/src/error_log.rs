use std::path::PathBuf;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use std::sync::Arc;
use tauri::State;

/// Write error to error.log file
pub fn log_error(cwd: &PathBuf, error: &str) {
    let log_path = cwd.join("error.log");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_entry = format!("[{}] ERROR: {}\n", timestamp, error);
    
    // Append to log file
    if let Err(e) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
    {
        eprintln!("Failed to write to error log: {}", e);
    }
}

/// Write info to error.log file
pub fn log_info(cwd: &PathBuf, info: &str) {
    let log_path = cwd.join("error.log");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_entry = format!("[{}] INFO: {}\n", timestamp, info);
    
    if let Err(e) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
    {
        eprintln!("Failed to write to error log: {}", e);
    }
}

/// Write warning to error.log file
pub fn log_warning(cwd: &PathBuf, warning: &str) {
    let log_path = cwd.join("error.log");
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_entry = format!("[{}] WARNING: {}\n", timestamp, warning);
    
    if let Err(e) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut file| file.write_all(log_entry.as_bytes()))
    {
        eprintln!("Failed to write to error log: {}", e);
    }
}

#[tauri::command]
pub async fn load_error_log(cwd: State<'_, Arc<PathBuf>>) -> Result<String, String> {
    let log_path = cwd.join("error.log");
    
    if !log_path.exists() {
        return Ok(String::new());
    }
    
    let content = tokio::fs::read_to_string(&log_path)
        .await
        .map_err(|e| format!("Failed to read error log: {}", e))?;
    
    Ok(content)
}

#[tauri::command]
pub async fn clear_error_log(cwd: State<'_, Arc<PathBuf>>) -> Result<(), String> {
    let log_path = cwd.join("error.log");
    
    if log_path.exists() {
        tokio::fs::write(&log_path, "")
            .await
            .map_err(|e| format!("Failed to clear error log: {}", e))?;
    }
    
    Ok(())
}

