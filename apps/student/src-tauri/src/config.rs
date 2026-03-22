// ============================================================
// 学生端配置（替换 electron/config.js）
// ============================================================
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

const DEFAULT_PASSWORD_HASH: &str =
    "240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9"; // "admin123"

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentConfig {
    pub teacher_ip:          String,
    pub port:                u16,
    pub admin_password_hash: String,
}

impl Default for StudentConfig {
    fn default() -> Self {
        Self {
            teacher_ip:          "192.168.1.100".to_string(),
            port:                3000,
            admin_password_hash: DEFAULT_PASSWORD_HASH.to_string(),
        }
    }
}

impl StudentConfig {
    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.teacher_ip, self.port)
    }

    pub fn verify_password(&self, pwd: &str) -> bool {
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(pwd.as_bytes()));
        let expected = if self.admin_password_hash.is_empty() {
            DEFAULT_PASSWORD_HASH
        } else {
            &self.admin_password_hash
        };
        hash == expected
    }
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("config.json")
}

pub fn load_config(app: &AppHandle) -> StudentConfig {
    let path = config_path(app);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(app: &AppHandle, cfg: &StudentConfig) -> bool {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    serde_json::to_string_pretty(cfg)
        .ok()
        .and_then(|s| std::fs::write(&path, s).ok())
        .is_some()
}
