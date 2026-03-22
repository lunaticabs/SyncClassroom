// ============================================================
// Socket.io 事件处理（替换 server.js 中 io.on('connection', ...) 整段）
//
// socketioxide 实现 Socket.io v4 协议 → 前端 socket.io-client 零改动
// ============================================================
use serde_json::{json, Value};
use socketioxide::socket::DisconnectReason;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use std::net::SocketAddr;

use crate::server::state::SharedState;

pub fn register(io: &SocketIo, shared: SharedState) {
    let shared_clone = shared.clone();

    io.ns("/", move |socket: SocketRef, Data(auth): Data<Value>| {
        let shared = shared_clone.clone();

        // ── 角色判定 ───────────────────────────────────────
        // 通过 token 认证教师端（教师端启动时生成 uuid 注入 WebView）
        let token = auth
            .as_object()
            .and_then(|o| o.get("token"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let is_host = {
            let st = shared.read().unwrap();
            !st.host_token.is_empty() && token == st.host_token
        };
        let role = if is_host { "host" } else { "viewer" };

        // 获取客户端 IP（ConnectInfo 注入）
        let peer_ip = socket
            .req_parts()
            .extensions
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        log::info!("[socket] connect  role={role} ip={peer_ip} id={}", socket.id);

        // ── 发送初始状态 ──────────────────────────────────
        {
            let st = shared.read().unwrap();
            let _ = socket.emit(
                "role-assigned",
                json!({
                    "role":             role,
                    "currentCourseId":  st.current_course_id,
                    "currentSlideIndex":st.current_slide_index,
                    "courseCatalog":    st.course_catalog,
                    "hostSettings":     st.host_settings,
                }),
            );
        }

        // ── Host 加入 hosts 房间 ──────────────────────────
        if is_host {
            let _ = socket.join("hosts");
            let count = shared.read().unwrap().student_ips.len();
            let _ = socket.emit("student-status", json!({ "count": count, "action": "init" }));
        } else {
            // ── Student 上线 ─────────────────────────────
            let entry = {
                let mut st = shared.write().unwrap();
                let prev = *st.student_ips.get(&peer_ip).unwrap_or(&0);
                st.student_ips.insert(peer_ip.clone(), prev + 1);
                if prev == 0 {
                    let e = st.push_log("join", &peer_ip);
                    Some((st.student_ips.len(), e))
                } else {
                    None
                }
            };
            if let Some((count, entry)) = entry {
                let _ = socket.broadcast().to("hosts").emit(
                    "student-status",
                    json!({ "count": count, "action": "join", "ip": peer_ip }),
                );
                let _ = socket.broadcast().to("hosts").emit("student-log-entry", entry);
            }
        }

        // ── 事件处理 ──────────────────────────────────────

        // 查询学生数（教师端主动请求）
        {
            let shared = shared.clone();
            socket.on("get-student-count", move |socket: SocketRef| {
                let count = shared.read().unwrap().student_ips.len();
                let _ = socket.emit("student-status", json!({ "count": count, "action": "init" }));
            });
        }

        // 翻页同步（仅 host 可广播）
        if is_host {
            let shared = shared.clone();
            socket.on("sync-slide", move |socket: SocketRef, Data(data): Data<Value>| {
                if let Some(idx) = data.get("slideIndex").and_then(|v| v.as_u64()) {
                    shared.write().unwrap().current_slide_index = idx as usize;
                }
                let _ = socket.broadcast().emit("sync-slide", &data);
            });
        }

        // 课程切换
        if is_host {
            let shared = shared.clone();
            socket.on("select-course", move |socket: SocketRef, Data(data): Data<Value>| {
                let course_id = data.get("courseId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let payload = {
                    let mut st = shared.write().unwrap();
                    if let Some(course) = st.course_catalog.iter().find(|c| c.id == course_id).cloned() {
                        st.current_course_id    = Some(course_id.clone());
                        st.current_slide_index  = 0;
                        log::info!("[course] switched → {}", course.title);
                        Some(json!({
                            "courseId":     course_id,
                            "courseFile":   course.file,
                            "slideIndex":   0,
                            "hostSettings": st.host_settings,
                        }))
                    } else {
                        None
                    }
                };
                if let Some(p) = payload {
                    let _ = socket.broadcast().emit("course-changed", &p);
                    let _ = socket.emit("course-changed", &p);
                }
            });
        }

        // 结束课程
        if is_host {
            let shared = shared.clone();
            socket.on("end-course", move |socket: SocketRef| {
                {
                    let mut st = shared.write().unwrap();
                    st.current_course_id   = None;
                    st.current_slide_index = 0;
                }
                log::info!("[course] ended");
                let _ = socket.broadcast().emit("course-ended", json!({}));
                let _ = socket.emit("course-ended", json!({}));
            });
        }

        // 注册课件依赖映射（课件加载时前端发来 filename → CDN URL）
        {
            let shared = shared.clone();
            socket.on(
                "register-dependencies",
                move |_: SocketRef, Data(deps): Data<Value>| {
                    if let Some(arr) = deps.as_array() {
                        let mut st = shared.write().unwrap();
                        for dep in arr {
                            if let (Some(filename), Some(src)) = (
                                dep.get("filename").and_then(|v| v.as_str()),
                                dep.get("publicSrc").and_then(|v| v.as_str()),
                            ) {
                                st.dependency_map
                                    .entry(filename.to_string())
                                    .or_insert_with(|| src.to_string());
                            }
                        }
                    }
                },
            );
        }

        // 学生上报异常行为
        if !is_host {
            let shared = shared.clone();
            let peer_ip2 = peer_ip.clone();
            socket.on(
                "student-alert",
                move |socket: SocketRef, Data(data): Data<Value>| {
                    let alert_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let entry = shared.write().unwrap().push_log(&alert_type, &peer_ip2);
                    let _ = socket.broadcast().to("hosts").emit(
                        "student-alert",
                        json!({ "ip": peer_ip2, "type": alert_type }),
                    );
                    let _ = socket.broadcast().to("hosts").emit("student-log-entry", entry);
                },
            );
        }

        // 教师端推送设置变更
        if is_host {
            let shared = shared.clone();
            socket.on(
                "host-settings",
                move |socket: SocketRef, Data(settings): Data<Value>| {
                    if let Ok(new_settings) = serde_json::from_value(settings.clone()) {
                        shared.write().unwrap().host_settings = new_settings;
                    }
                    let _ = socket.broadcast().emit("host-settings", &settings);
                },
            );
        }

        // 教师端推送新管理员密码 hash 给所有学生
        if is_host {
            socket.on(
                "set-admin-password",
                move |socket: SocketRef, Data(data): Data<Value>| {
                    if data.get("hash").and_then(|v| v.as_str()).is_some() {
                        let _ = socket.broadcast().emit("set-admin-password", &data);
                    }
                },
            );
        }

        // 刷新课程目录
        if is_host {
            let shared = shared.clone();
            socket.on("refresh-courses", move |socket: SocketRef| {
                let courses = {
                    let mut st = shared.write().unwrap();
                    st.course_catalog = st.scan_courses();
                    st.course_catalog.clone()
                };
                let _ = socket.emit(
                    "course-catalog-updated",
                    json!({ "courses": courses }),
                );
            });
        }

        // ── 断开连接 ──────────────────────────────────────
        if !is_host {
            let shared = shared.clone();
            let peer_ip3 = peer_ip.clone();
            socket.on_disconnect(move |socket: SocketRef, _: DisconnectReason| {
                log::info!("[socket] disconnect ip={peer_ip3}");
                let result = {
                    let mut st = shared.write().unwrap();
                    let remaining = st.student_ips.get(&peer_ip3).copied().unwrap_or(1) - 1;
                    if remaining == 0 {
                        st.student_ips.remove(&peer_ip3);
                        let entry = st.push_log("leave", &peer_ip3);
                        Some((st.student_ips.len(), entry))
                    } else {
                        st.student_ips.insert(peer_ip3.clone(), remaining);
                        None
                    }
                };
                if let Some((count, entry)) = result {
                    let _ = socket.broadcast().to("hosts").emit(
                        "student-status",
                        json!({ "count": count, "action": "leave", "ip": peer_ip3 }),
                    );
                    let _ = socket.broadcast().to("hosts").emit("student-log-entry", entry);
                }
            });
        }
    });
}
