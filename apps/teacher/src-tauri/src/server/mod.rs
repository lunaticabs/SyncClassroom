// ============================================================
// 服务器组装（替换 server.js 整文件）
// ============================================================
pub mod routes;
pub mod socket;
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};
use socketioxide::SocketIo;
use tower_http::{cors::CorsLayer, services::ServeDir};

use routes::{api, proxy};
use state::SharedState;

// ServerState 作为 axum 的共享状态，同时携带 SocketIo 句柄
#[derive(Clone)]
pub struct ServerState {
    pub app_state: SharedState,
    pub io:        SocketIo,
}

pub fn build_router(app_state: SharedState) -> (Router<()>, SocketIo) {
    let (socket_layer, io) = SocketIo::builder().build_layer();

    socket::register(&io, app_state.clone());

    let state = ServerState {
        app_state: app_state.clone(),
        io: io.clone(),
    };

    let public_dir = app_state.read().unwrap().public_dir.clone();

    // socket.io 路由单独挂 socket_layer，不污染静态文件路由
    let socket_router = Router::new()
        .layer(socket_layer);

    let router = Router::new()
        .route("/api/courses",         get(api::get_courses))
        .route("/api/course-status",   get(api::get_course_status))
        .route("/api/refresh-courses", post(api::refresh_courses))
        .route("/api/students",        get(api::get_students))
        .route("/api/student-log",     get(api::get_student_log))
        .route("/api/download-skill",  get(api::download_skill))
        .route("/api/course-guide",    get(api::course_guide))
        .route("/lib/{filename}",      get(proxy::lib_proxy))
        .route("/webfonts/{filename}", get(proxy::webfonts_proxy))
        .route("/weights/{filename}",  get(proxy::weights_proxy))
        .route("/images/proxy",        get(proxy::image_proxy))
        .merge(socket_router)
        .fallback_service(ServeDir::new(&public_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    (router, io)
}
