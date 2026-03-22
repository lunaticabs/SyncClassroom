// ============================================================
// 教师端 Tauri Commands（替换 ipcMain.handle 全部处理器）
// ============================================================
use tauri::{AppHandle, Manager, Runtime, Window};

use crate::config::{load_settings, save_settings, Settings};

// ── 设置读写 ──────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    load_settings(&app)
}

#[tauri::command]
pub fn save_settings_cmd(app: AppHandle, settings: Settings) -> bool {
    save_settings(&app, &settings)
}

// ── 角色标识 ──────────────────────────────────────────────

#[tauri::command]
pub fn get_role() -> &'static str {
    "teacher"
}

// ── 全屏切换 ──────────────────────────────────────────────

#[tauri::command]
pub fn toggle_fullscreen<R: Runtime>(window: Window<R>) -> Result<(), String> {
    let is_full = window.is_fullscreen().map_err(|e| e.to_string())?;
    window.set_fullscreen(!is_full).map_err(|e| e.to_string())
}

// ── 日志目录 ──────────────────────────────────────────────

#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> Result<String, String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&log_dir);
    let path_str = log_dir.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(path_str)
}

// ── 导入课程文件 ──────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub success:  bool,
    pub imported: Vec<String>,
    pub skipped:  Vec<String>,
    pub canceled: bool,
}

#[tauri::command]
pub async fn import_course(app: AppHandle) -> ImportResult {
    use tauri_plugin_dialog::DialogExt;

    let result = app
        .dialog()
        .file()
        .set_title("导入课程文件")
        .add_filter("课程文件", &["tsx", "js", "ts"])
        .blocking_pick_files();

    let Some(paths) = result else {
        return ImportResult { success: false, imported: vec![], skipped: vec![], canceled: true };
    };

    // 目标目录：public/courses/
    let courses_dir = {
        let data_dir = app.path().resource_dir().unwrap_or_default();
        data_dir.join("public").join("courses")
    };
    let _ = std::fs::create_dir_all(&courses_dir);

    let mut imported = vec![];
    let mut skipped  = vec![];

    for path in paths {
        let src = path.as_path().unwrap();
        let name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest = courses_dir.join(&name);
        match std::fs::copy(src, &dest) {
            Ok(_)  => imported.push(name),
            Err(_) => skipped.push(name),
        }
    }

    ImportResult { success: true, imported, skipped, canceled: false }
}
