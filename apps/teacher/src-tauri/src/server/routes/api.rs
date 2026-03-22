// ============================================================
// REST API 路由（替换 server.js 中 🛠️ API 路由 段落）
// ============================================================
use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tokio::fs;

use crate::server::ServerState;

// GET /api/courses
pub async fn get_courses(State(state): State<ServerState>) -> impl IntoResponse {
    let st = state.app_state.read().unwrap();
    Json(json!({
        "courses":           st.course_catalog,
        "currentCourseId":   st.current_course_id,
        "currentSlideIndex": st.current_slide_index,
    }))
}

// GET /api/course-status
pub async fn get_course_status(State(state): State<ServerState>) -> impl IntoResponse {
    let st = state.app_state.read().unwrap();
    Json(json!({
        "currentCourseId":   st.current_course_id,
        "currentSlideIndex": st.current_slide_index,
    }))
}

// POST /api/refresh-courses
pub async fn refresh_courses(State(state): State<ServerState>) -> impl IntoResponse {
    let courses = {
        let mut st = state.app_state.write().unwrap();
        st.course_catalog = st.scan_courses();
        st.course_catalog.clone()
    };
    // 通知所有已连接的 host
    let _ = state.io.to("hosts").emit(
        "course-catalog-updated",
        json!({ "courses": courses }),
    );
    Json(json!({ "success": true, "courses": courses }))
}

// GET /api/students
pub async fn get_students(State(state): State<ServerState>) -> impl IntoResponse {
    let st = state.app_state.read().unwrap();
    let students: Vec<String> = st.student_ips.keys().cloned().collect();
    Json(json!({ "students": students }))
}

// GET /api/student-log
pub async fn get_student_log(State(state): State<ServerState>) -> impl IntoResponse {
    let st = state.app_state.read().unwrap();
    let log: Vec<_> = st.student_log.iter().cloned().collect();
    Json(json!({ "log": log }))
}

// GET /api/download-skill
pub async fn download_skill(State(state): State<ServerState>) -> impl IntoResponse {
    let path = state.app_state.read().unwrap().public_dir
        .parent().unwrap_or(std::path::Path::new("."))
        .join("create-course.md");
    serve_file_download(path, "create-course.md", "text/markdown; charset=utf-8").await
}

// GET /api/course-guide
pub async fn course_guide(State(state): State<ServerState>) -> impl IntoResponse {
    let path = state.app_state.read().unwrap().public_dir
        .parent().unwrap_or(std::path::Path::new("."))
        .join("course-template.md");
    serve_file_inline(path, "text/plain; charset=utf-8").await
}

// ── 辅助函数 ─────────────────────────────────────────────

async fn serve_file_download(
    path: std::path::PathBuf,
    filename: &'static str,
    content_type: &'static str,
) -> Response {
    match fs::read(&path).await {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            )
            .body(Body::from(bytes))
            .unwrap(),
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}

async fn serve_file_inline(path: std::path::PathBuf, content_type: &'static str) -> Response {
    match fs::read(&path).await {
        Ok(bytes) => Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(bytes))
            .unwrap(),
        Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
    }
}
