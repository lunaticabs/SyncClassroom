// ============================================================
// 学生端主入口
// 职责：开机自启、托盘常驻、课堂开始时全屏置顶、连接教师端
// ============================================================
mod autostart;
mod commands;
mod config;

use commands::{SharedStudentState, StudentState};
use config::load_config;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

const RETRY_INTERVAL_MS: u64 = 5_000;

// ── 主窗口导航（连接到教师端 URL）────────────────────────

fn navigate_to_teacher(app: &AppHandle, url: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.navigate(url.parse().unwrap());
        let _ = w.show();
    }
}

fn show_offline(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
    }
}

// ── 后台重试任务 ──────────────────────────────────────────

fn start_retry_task(app: AppHandle, url: String) {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap();

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(RETRY_INTERVAL_MS)).await;
            if client.get(&url).send().await.is_ok() {
                navigate_to_teacher(&app, &url);
                break;
            }
        }
    });
}

// ── 托盘菜单 ──────────────────────────────────────────────

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show  = MenuItemBuilder::with_id("show",  "显示窗口").build(app)?;
    let admin = MenuItemBuilder::with_id("admin", "管理员设置...").build(app)?;
    let sep   = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit  = MenuItemBuilder::with_id("quit",  "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &admin, &sep, &quit])
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("SyncClassroom 学生端")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "admin" => {
                let _ = commands::open_admin_window(app.clone());
            }
            "quit" => {
                // 退出需要通过管理员窗口验证密码后触发 _quit 标志
                let _ = commands::open_admin_window(app.clone());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

// ── Tauri 入口 ────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let cfg = load_config(app.handle());
            let server_url = cfg.server_url();

            // ── 初始化共享状态 ─────────────────────────────
            let student_state: SharedStudentState = Arc::new(Mutex::new(StudentState {
                config:          cfg,
                is_class_active: false,
                force_fullscreen: true,
            }));
            app.manage(student_state);

            // ── 创建主窗口，初始加载离线页面 ──────────────
            // offline.html 内嵌在二进制资源中，确保无需网络即可显示
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("frontend/offline.html".into()),
            )
            .title("SyncClassroom 学生端")
            .inner_size(1280.0, 800.0)
            .decorations(false)
            .visible(false)
            .initialization_script(&format!(
                r#"
                window.__SYNCCLASSROOM_ROLE__ = "student";
                window.__SYNCCLASSROOM_SERVER__ = "{server_url}";
                "#
            ))
            .build()?;

            // ── 尝试连接教师端，失败则启动重试 ───────────
            let app_handle = app.handle().clone();
            let url_clone  = server_url.clone();
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(3))
                    .build()
                    .unwrap();

                if client.get(&url_clone).send().await.is_ok() {
                    navigate_to_teacher(&app_handle, &url_clone);
                } else {
                    // 显示离线页面，并启动后台重试
                    show_offline(&app_handle);
                    start_retry_task(app_handle, url_clone);
                }
            });

            // ── 系统托盘 ───────────────────────────────────
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config_cmd,
            commands::verify_password,
            commands::get_role,
            commands::get_autostart,
            commands::set_autostart,
            commands::class_started,
            commands::class_ended,
            commands::set_fullscreen,
            commands::set_admin_password,
            commands::manual_retry,
            commands::open_admin_window,
            commands::get_settings,
            commands::save_settings,
        ])
        // 关闭窗口 → 隐藏到托盘，阻止退出
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // admin 窗口可以直接关闭
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
