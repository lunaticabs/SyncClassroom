// ============================================================
// 服务器共享状态（替换 server.js 中所有全局变量）
// ============================================================
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ── 数据结构 ─────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Course {
    pub id:    String,
    pub file:  String,
    pub title: String,
    pub icon:  String,
    pub desc:  String,
    pub color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSettings {
    pub force_fullscreen:      bool,
    pub sync_follow:           bool,
    pub alert_join:            bool,
    pub alert_leave:           bool,
    pub alert_fullscreen_exit: bool,
    pub alert_tab_hidden:      bool,
}

impl Default for HostSettings {
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

// ── 主状态结构体 ──────────────────────────────────────────

pub struct AppState {
    pub current_course_id:    Option<String>,
    pub current_slide_index:  usize,
    pub course_catalog:       Vec<Course>,
    /// IP → 连接数（同 IP 多 tab 只算一个学生）
    pub student_ips:          HashMap<String, usize>,
    pub host_settings:        HostSettings,
    /// 环形日志，最多 500 条
    pub student_log:          VecDeque<Value>,
    /// 课件注册的依赖映射 filename → CDN URL
    pub dependency_map:       HashMap<String, String>,
    /// 教师端识别 token（防止 LAN 内其他机器冒充 host）
    pub host_token:           String,
    /// 静态文件根目录（public/）
    pub public_dir:           PathBuf,
}

impl AppState {
    pub fn new(public_dir: PathBuf, host_token: String) -> Self {
        let mut state = Self {
            current_course_id:   None,
            current_slide_index: 0,
            course_catalog:      vec![],
            student_ips:         HashMap::new(),
            host_settings:       HostSettings::default(),
            student_log:         VecDeque::new(),
            dependency_map:      HashMap::new(),
            host_token,
            public_dir,
        };
        state.course_catalog = state.scan_courses();
        state
    }

    // ── 课程目录扫描（替换 scanCourses()）────────────────

    pub fn scan_courses(&self) -> Vec<Course> {
        let dir = self.public_dir.join("courses");
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
            return vec![];
        }

        let re_title = regex::Regex::new(r#"title:\s*["'](.+?)["']"#).unwrap();
        let re_icon  = regex::Regex::new(r#"icon:\s*["'](.+?)["']"#).unwrap();
        let re_desc  = regex::Regex::new(r#"desc:\s*["'](.+?)["']"#).unwrap();
        let re_color = regex::Regex::new(r#"color:\s*["'](.+?)["']"#).unwrap();

        walkdir::WalkDir::new(&dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy();
                e.file_type().is_file()
                    && (name.ends_with(".js") || name.ends_with(".ts") || name.ends_with(".tsx"))
            })
            .filter_map(|e| {
                let file = e.file_name().to_string_lossy().to_string();
                let content = std::fs::read_to_string(e.path()).ok()?;

                // 只从 window.CourseData 之后的内容提取元数据
                let search_from = content
                    .find("window.CourseData")
                    .map(|i| &content[i..])
                    .unwrap_or(&content);

                let id    = file.trim_end_matches(".tsx").trim_end_matches(".ts").trim_end_matches(".js").to_string();
                let title = re_title.captures(search_from).map(|c| c[1].to_string()).unwrap_or_else(|| id.clone());
                let icon  = re_icon .captures(search_from).map(|c| c[1].to_string()).unwrap_or_else(|| "📚".to_string());
                let desc  = re_desc .captures(search_from).map(|c| c[1].to_string()).unwrap_or_default();
                let color = re_color.captures(search_from).map(|c| c[1].to_string())
                    .unwrap_or_else(|| "from-blue-500 to-indigo-600".to_string());

                Some(Course { id, file, title, icon, desc, color })
            })
            .collect()
    }

    // ── 日志推送 ─────────────────────────────────────────

    pub fn push_log(&mut self, log_type: &str, ip: &str) -> Value {
        let entry = json!({
            "time": Utc::now().to_rfc3339(),
            "type": log_type,
            "ip":   ip,
        });
        self.student_log.push_back(entry.clone());
        if self.student_log.len() > 500 {
            self.student_log.pop_front();
        }
        entry
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
