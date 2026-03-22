// ============================================================
// CDN 智能缓存代理（替换 server.js 中 ⚡️ 智能缓存代理中心 段落）
//
// 逻辑：本地有缓存 → 直接返回；无缓存 → 从 CDN 下载，
//       同时写盘 & 流式返回给请求方。
// ============================================================
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use serde::Deserialize;
use tokio::fs;

use crate::server::ServerState;

const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

// ── 已知文件 URL 映射表 ───────────────────────────────────

fn known_url(filename: &str) -> Option<&'static str> {
    match filename {
        "tailwindcss.js"           => Some("https://cdn.tailwindcss.com"),
        "fontawesome.all.min.css"  => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"),
        "fa-solid-900.woff2"       => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-solid-900.woff2"),
        "fa-solid-900.ttf"         => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-solid-900.ttf"),
        "fa-regular-400.woff2"     => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-regular-400.woff2"),
        "fa-regular-400.ttf"       => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-regular-400.ttf"),
        "fa-brands-400.woff2"      => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-brands-400.woff2"),
        "fa-brands-400.ttf"        => Some("https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/fa-brands-400.ttf"),
        "react.development.js"     => Some("https://unpkg.com/react@18/umd/react.development.js"),
        "react-dom.development.js" => Some("https://unpkg.com/react-dom@18/umd/react-dom.development.js"),
        "babel.min.js"             => Some("https://unpkg.com/@babel/standalone/babel.min.js"),
        "face-api.min.js"          => Some("https://fastly.jsdelivr.net/npm/face-api.js@0.22.2/dist/face-api.min.js"),
        "socket.io.min.js"         => Some("https://cdn.socket.io/4.7.5/socket.io.min.js"),
        _                          => None,
    }
}

// ── 核心下载函数 ──────────────────────────────────────────

async fn download(urls: &[String]) -> Option<(Bytes, String)> {
    let client = reqwest::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .ok()?;

    for url in urls {
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        if let Ok(bytes) = resp.bytes().await {
            log::info!("[proxy] downloaded {} bytes from {url}", bytes.len());
            return Some((bytes, ct));
        }
    }
    None
}

fn candidate_urls(filename: &str, registered: Option<String>) -> Vec<String> {
    if let Some(url) = known_url(filename) {
        return vec![url.to_string()];
    }
    if let Some(url) = registered {
        let fastly = url.replace("cdn.jsdelivr.net", "fastly.jsdelivr.net");
        return vec![fastly, url];
    }

    // 从文件名猜测 npm 包名
    let pkg = filename
        .trim_end_matches(".umd.min.js")
        .trim_end_matches(".min.js")
        .trim_end_matches(".js")
        .trim_end_matches(".css");

    let pkg_mapped = match pkg {
        "chart" => "chart.js",
        other   => other,
    };

    vec![
        format!("https://fastly.jsdelivr.net/npm/{pkg_mapped}@latest/dist/{filename}"),
        format!("https://fastly.jsdelivr.net/npm/{pkg_mapped}@latest/{filename}"),
        format!("https://cdn.jsdelivr.net/npm/{pkg_mapped}@latest/dist/{filename}"),
        format!("https://cdn.jsdelivr.net/npm/{pkg_mapped}@latest/{filename}"),
        format!("https://unpkg.com/{pkg_mapped}@latest/dist/{filename}"),
        format!("https://unpkg.com/{pkg_mapped}@latest/{filename}"),
    ]
}

fn build_response(bytes: Bytes, content_type: &str, cache: bool) -> Response {
    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, content_type);
    if cache {
        builder = builder.header(
            header::CACHE_CONTROL,
            "public, max-age=31536000",
        );
    } else {
        builder = builder.header(
            header::CACHE_CONTROL,
            "no-cache, no-store, must-revalidate",
        );
    }
    builder.body(Body::from(bytes)).unwrap()
}

// ── GET /lib/:filename ────────────────────────────────────

pub async fn lib_proxy(
    Path(filename): Path<String>,
    State(state): State<ServerState>,
) -> Response {
    let lib_dir = state.app_state.read().unwrap().public_dir.join("lib");
    let local   = lib_dir.join(&filename);

    // 缓存命中
    if local.exists() {
        let ct = mime_from_filename(&filename);
        if let Ok(bytes) = fs::read(&local).await {
            return build_response(bytes.into(), ct, false);
        }
    }

    // 确定下载 URL
    let registered = state
        .app_state
        .read()
        .unwrap()
        .dependency_map
        .get(&filename)
        .cloned();
    let urls = candidate_urls(&filename, registered);

    log::info!("[proxy] fetching {filename}...");
    match download(&urls).await {
        Some((bytes, ct)) => {
            let _ = fs::create_dir_all(&lib_dir).await;
            let _ = fs::write(&local, &bytes).await;
            build_response(bytes, &ct, true)
        }
        None => {
            log::warn!("[proxy] not found: {filename}");
            (StatusCode::NOT_FOUND, format!("not found: {filename}")).into_response()
        }
    }
}

// ── GET /webfonts/:filename （FontAwesome CSS 内嵌路径）────

pub async fn webfonts_proxy(
    Path(filename): Path<String>,
    State(state): State<ServerState>,
) -> Response {
    // FontAwesome 字体文件缓存在 lib/ 目录
    let lib_dir = state.app_state.read().unwrap().public_dir.join("lib");
    let local   = lib_dir.join(&filename);

    if local.exists() {
        if let Ok(bytes) = fs::read(&local).await {
            return build_response(bytes.into(), mime_from_filename(&filename), false);
        }
    }

    if let Some(url) = known_url(&filename) {
        log::info!("[proxy] fetching font {filename}...");
        if let Some((bytes, ct)) = download(&[url.to_string()]).await {
            let _ = fs::create_dir_all(&lib_dir).await;
            let _ = fs::write(&local, &bytes).await;
            return build_response(bytes, &ct, true);
        }
    }

    (StatusCode::NOT_FOUND, "font not found").into_response()
}

// ── GET /weights/:filename （AI 模型权重）────────────────

pub async fn weights_proxy(
    Path(filename): Path<String>,
    State(state): State<ServerState>,
) -> Response {
    let weights_dir = state.app_state.read().unwrap().public_dir.join("weights");
    let local       = weights_dir.join(&filename);

    if local.exists() {
        if let Ok(bytes) = fs::read(&local).await {
            return build_response(bytes.into(), "application/octet-stream", false);
        }
    }

    let url = format!(
        "https://fastly.jsdelivr.net/gh/justadudewhohacks/face-api.js@master/weights/{filename}"
    );
    log::info!("[proxy] fetching model {filename}...");
    match download(&[url]).await {
        Some((bytes, ct)) => {
            let _ = fs::create_dir_all(&weights_dir).await;
            let _ = fs::write(&local, &bytes).await;
            build_response(bytes, &ct, true)
        }
        None => (StatusCode::NOT_FOUND, "model not found").into_response(),
    }
}

// ── GET /images/proxy?url=... ─────────────────────────────

#[derive(Deserialize)]
pub struct ImageProxyParams {
    url: Option<String>,
}

pub async fn image_proxy(
    Query(params): Query<ImageProxyParams>,
    State(state): State<ServerState>,
) -> Response {
    let Some(image_url) = params.url else {
        return (StatusCode::BAD_REQUEST, "missing url param").into_response();
    };

    let url_obj = match reqwest::Url::parse(&image_url) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid url").into_response(),
    };

    // 用 URL 的 MD5 作为缓存文件名
    let hash   = {
        use md5::{Digest, Md5};
        format!("{:x}", Md5::digest(image_url.as_bytes()))
    };
    let ext    = std::path::Path::new(url_obj.path())
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let cache_name = format!("{hash}.{ext}");

    let images_dir = state.app_state.read().unwrap().public_dir.join("images");
    let local      = images_dir.join(&cache_name);

    if local.exists() {
        if let Ok(bytes) = fs::read(&local).await {
            return build_response(bytes.into(), mime_from_ext(ext), false);
        }
    }

    log::info!("[img-proxy] downloading {}", &image_url[..image_url.len().min(80)]);
    match download(&[image_url]).await {
        Some((bytes, ct)) => {
            let _ = fs::create_dir_all(&images_dir).await;
            let _ = fs::write(&local, &bytes).await;
            build_response(bytes, &ct, true)
        }
        None => (StatusCode::NOT_FOUND, "image not found").into_response(),
    }
}

// ── MIME 辅助 ─────────────────────────────────────────────

fn mime_from_filename(name: &str) -> &'static str {
    if name.ends_with(".js")    { return "application/javascript"; }
    if name.ends_with(".css")   { return "text/css"; }
    if name.ends_with(".woff2") { return "font/woff2"; }
    if name.ends_with(".woff")  { return "font/woff"; }
    if name.ends_with(".ttf")   { return "font/ttf"; }
    if name.ends_with(".eot")   { return "application/vnd.ms-fontobject"; }
    "application/octet-stream"
}

fn mime_from_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png"          => "image/png",
        "gif"          => "image/gif",
        "webp"         => "image/webp",
        "svg"          => "image/svg+xml",
        _              => "application/octet-stream",
    }
}
