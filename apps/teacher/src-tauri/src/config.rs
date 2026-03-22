// ============================================================
// 教师端配置（替换 electron/config.js 中 Settings 相关部分）
// ============================================================
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub force_fullscreen:      bool,
    pub sync_follow:           bool,
    pub alert_join:            bool,
    pub alert_leave:           bool,
    pub alert_fullscreen_exit: bool,
    pub alert_tab_hidden:      bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            force_fullscreen:      true,
            sync_follow:           true,
            alert_join:            true,
            alert_leave:           true,
            alert_fullscreen_exit: true,
            alert_tab_hidden:      true,
        }
    }
}

fn settings_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("settings.json")
}

pub fn load_settings(app: &AppHandle) -> Settings {
    let path = settings_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> bool {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    serde_json::to_string_pretty(settings)
        .ok()
        .and_then(|s| std::fs::write(&path, s).ok())
        .is_some()
}
