# SyncClassroom — 构建任务
# 用法: just <任务名>
# 安装: cargo install just

# 默认列出所有任务
default:
    @just --list

# ── 开发模式 ─────────────────────────────────────────────

# 启动教师端（内嵌服务器，热重载）
dev-teacher:
    cd apps/teacher && cargo tauri dev

# 启动学生端
dev-student:
    cd apps/student && cargo tauri dev

# ── 生产构建 ─────────────────────────────────────────────

# 构建教师端安装包
build-teacher:
    cd apps/teacher && cargo tauri build

# 构建学生端安装包
build-student:
    cd apps/student && cargo tauri build

# 构建两端
build-all: build-teacher build-student

# ── 图标生成 ─────────────────────────────────────────────

# 生成所有图标（需要 Python + Pillow）
icons:
    python3 build/convert-icons.py

# ── 初始化（首次克隆后运行）─────────────────────────────

# 安装 tauri-cli，生成图标
setup:
    cargo install tauri-cli --version "^2" --locked
    just icons

# ── 清理 ─────────────────────────────────────────────────

clean:
    cargo clean --manifest-path apps/teacher/src-tauri/Cargo.toml
    cargo clean --manifest-path apps/student/src-tauri/Cargo.toml
