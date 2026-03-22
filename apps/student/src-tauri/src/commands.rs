// ============================================================
// 学生端 Tauri Commands（替换 ipcMain 全部处理器）
// ============================================================
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Runtime, State, Window, WebviewUrl, WebviewWindowBuilder};

use crate::config::{load_config, save_config, StudentConfig};
use crate::autostart::windows as autostart;

// ── 全局可变状态（学生端不需要 server，状态简单）────────

pub struct StudentState {
    pub config: StudentConfig,
    pub is_class_active: bool,
    pub force_fullscreen: bool,
}

pub type SharedStudentState = Arc<Mutex<StudentState>>;

// ── Config ────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<SharedStudentState>) -> serde_json::Value {
    let st = state.lock().unwrap();
    serde_json::json!({
        "teacherIp": st.config.teacher_ip,
        "port":      st.config.port,
    })
}

#[tauri::command]
pub fn save_config_cmd(
    app: AppHandle,
    state: State<SharedStudentState>,
    config: serde_json::Value,
) -> bool {
    // 特殊退出标志
    if config.get("_quit").is_some() {
        app.exit(0);
        return true;
    }

    let mut st = state.lock().unwrap();
    if let Some(ip) = config.get("teacherIp").and_then(|v| v.as_str()) {
        st.config.teacher_ip = ip.to_string();
    }
    if let Some(port) = config.get("port").and_then(|v| v.as_u64()) {
        st.config.port = port as u16;
    }
    if let Some(hash) = config.get("adminPasswordHash").and_then(|v| v.as_str()) {
        st.config.admin_password_hash = hash.to_string();
    }
    save_config(&app, &st.config)
}

#[tauri::command]
pub fn verify_password(state: State<SharedStudentState>, pwd: String) -> serde_json::Value {
    let st = state.lock().unwrap();
    serde_json::json!({ "ok": st.config.verify_password(&pwd) })
}

#[tauri::command]
pub fn get_role() -> &'static str {
    "student"
}

// ── 开机自启动 ────────────────────────────────────────────

#[tauri::command]
pub fn get_autostart() -> bool {
    autostart::get_autostart()
}

#[derive(Serialize)]
pub struct AutostartResult {
    pub success: bool,
    pub error:   Option<String>,
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enable: bool) -> AutostartResult {
    let exe = app.path().current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    match autostart::set_autostart(enable, &exe) {
        Ok(_)    => AutostartResult { success: true, error: None },
        Err(msg) => AutostartResult { success: false, error: Some(msg) },
    }
}

// ── 课堂控制 ──────────────────────────────────────────────

#[tauri::command]
pub fn class_started<R: Runtime>(
    window: Window<R>,
    state: State<SharedStudentState>,
    opts: serde_json::Value,
) {
    let force = opts.get("forceFullscreen")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut st = state.lock().unwrap();
    st.is_class_active = true;
    st.force_fullscreen = force;
    drop(st);

    if force {
        let _ = window.set_fullscreen(true);
        let _ = window.set_always_on_top(true);
    }
}

#[tauri::command]
pub fn class_ended<R: Runtime>(
    window: Window<R>,
    state: State<SharedStudentState>,
) {
    let mut st = state.lock().unwrap();
    st.is_class_active = false;
    drop(st);

    let _ = window.set_fullscreen(false);
    let _ = window.set_always_on_top(false);
}

#[tauri::command]
pub fn set_fullscreen<R: Runtime>(
    window: Window<R>,
    state: State<SharedStudentState>,
    enable: bool,
) {
    let mut st = state.lock().unwrap();
    st.force_fullscreen = enable;
    let is_active = st.is_class_active;
    drop(st);

    if is_active {
        let _ = window.set_fullscreen(enable);
        let _ = window.set_always_on_top(enable);
    }
}

#[tauri::command]
pub fn set_admin_password(
    app: AppHandle,
    state: State<SharedStudentState>,
    hash: String,
) {
    let mut st = state.lock().unwrap();
    st.config.admin_password_hash = hash;
    save_config(&app, &st.config);
}

#[tauri::command]
pub fn manual_retry(
    app: AppHandle,
    state: State<SharedStudentState>,
) {
    let url = state.lock().unwrap().config.server_url();
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.navigate(url.parse().unwrap());
    }
}

// ── 管理员窗口（替换 electron/admin.html 的独立窗口）────

#[tauri::command]
pub fn open_admin_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("admin") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }

    // 找到 admin.html 资源路径
    let admin_path = app.path()
        .resource_dir()
        .unwrap_or_default()
        .join("frontend")
        .join("admin.html");

    WebviewWindowBuilder::new(
        &app,
        "admin",
        WebviewUrl::App(format!("frontend/admin.html").into()),
    )
    .title("管理员设置")
    .inner_size(420.0, 560.0)
    .resizable(false)
    .center()
    .build()
    .map(|_| ())
    .map_err(|e| e.to_string())
}

// 学生端不使用课堂设置（教师端专用），返回 null 避免报错
#[tauri::command]
pub fn get_settings() -> serde_json::Value { serde_json::Value::Null }
#[tauri::command]
pub fn save_settings(_settings: serde_json::Value) -> bool { true }
