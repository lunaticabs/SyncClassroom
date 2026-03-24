// ============================================================
// 教师端主入口
// 职责：启动内嵌 axum+socketioxide 服务器，创建窗口和托盘
// ============================================================
mod commands;
mod config;
mod server;

use server::state::{AppState, SharedState};
use std::sync::{Arc, RwLock};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tokio::net::TcpListener;
use uuid::Uuid;

const PORT: u16 = 3000;

// ── 服务器启动 ────────────────────────────────────────────

async fn start_server(shared: SharedState) {
    let (router, _io) = server::build_router(shared);
    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}"))
        .await
        .expect("Failed to bind port");
    log::info!("[server] listening on port {PORT}");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("Server error");
}

// ── 托盘菜单 ──────────────────────────────────────────────

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show   = MenuItemBuilder::with_id("show",     "打开控制台").build(app)?;
    let logs   = MenuItemBuilder::with_id("logs",     "打开日志目录").build(app)?;
    let quit   = MenuItemBuilder::with_id("quit",     "退出").build(app)?;
    let sep    = tauri::menu::PredefinedMenuItem::separator(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &sep, &logs, &sep, &quit])
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("SyncClassroom 教师端")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "logs" => {
                let _ = commands::open_log_dir(app.clone());
            }
            "quit" => app.exit(0),
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
    // 生成一次性 host token，注入到 WebView 让教师端认证
    let host_token = Uuid::new_v4().to_string();
    let host_token_js = host_token.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // ── 确定 public/ 目录路径 ─────────────────────────────
            let public_dir = {
                #[cfg(debug_assertions)]
                {
                    // 开发模式：CARGO_MANIFEST_DIR = apps/teacher/src-tauri/
                    // 往上三级就是项目根目录下的 public/
                    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../../public");
                    p.canonicalize().unwrap_or(p)
                }
                #[cfg(not(debug_assertions))]
                {
                    // 生产模式：资源已通过 bundle.resources 打包进去
                    app_handle
                        .path()
                        .resource_dir()
                        .unwrap_or_default()
                        .join("public")
                }
            };
            log::info!("[setup] public_dir = {:?} (exists={})", public_dir, public_dir.exists());

            // ── 初始化共享状态 ────────────────────────────
            let shared: SharedState = Arc::new(RwLock::new(AppState::new(
                public_dir,
                host_token.clone(),
            )));

            // ── 在 Tauri 的 tokio 运行时内启动服务器 ─────
            // 完全替代 electron 里的 fork(serverPath)
            let shared_for_server = shared.clone();
            tauri::async_runtime::spawn(async move {
                start_server(shared_for_server).await;
            });

            // ── 创建主窗口 ────────────────────────────────
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(
                    format!("http://localhost:{PORT}").parse().unwrap(),
                ),
            )
            .title("SyncClassroom 教师端")
            .inner_size(1280.0, 800.0)
            .min_inner_size(900.0, 600.0)
            .decorations(false)
            .visible(false)
            // 注入 host token 和 Tauri 桥接脚本
            .initialization_script(&format!(
                r#"
                window.__SYNCCLASSROOM_HOST_TOKEN__ = "{host_token_js}";
                window.__SYNCCLASSROOM_ROLE__ = "teacher";
                "#
            ))
            .build()?;

            // 等服务器就绪后显示窗口
            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::new();
                for _ in 0..40 {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    if client
                        .get(format!("http://localhost:{PORT}"))
                        .send()
                        .await
                        .is_ok()
                    {
                        break;
                    }
                }
                let _ = window_clone.show();
            });

            // ── 系统托盘 ──────────────────────────────────
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings_cmd,
            commands::get_role,
            commands::toggle_fullscreen,
            commands::open_log_dir,
            commands::import_course,
        ])
        // 关闭窗口时隐藏而非退出，保持服务器运行
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
