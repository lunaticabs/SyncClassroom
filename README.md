# SyncClassroom

局域网 AI 互动教学系统。教师端内嵌 Rust HTTP + Socket.io 服务器，学生端连接教师端。

**纯 Rust 工具链，无 Node.js 依赖。**

## 架构

```
SyncClassroom/
├── apps/
│   ├── teacher/src-tauri/    教师端（内嵌 axum 服务器）
│   │   └── src/
│   │       ├── lib.rs        Tauri 入口，spawn tokio 服务器
│   │       ├── config.rs     设置读写
│   │       ├── commands.rs   Tauri Commands
│   │       └── server/
│   │           ├── mod.rs    axum Router
│   │           ├── state.rs  共享状态（RwLock）
│   │           ├── socket.rs socketioxide 事件处理
│   │           └── routes/
│   │               ├── api.rs    REST 路由
│   │               └── proxy.rs  CDN 缓存代理
│   └── student/src-tauri/    学生端（连接教师机）
│       └── src/
│           ├── lib.rs        Tauri 入口
│           ├── config.rs     学生端配置（IP、密码）
│           ├── commands.rs   Tauri Commands
│           └── autostart.rs  开机自启（Windows 注册表）
├── public/                   共享前端（静态文件，无构建步骤）
├── assets/                   图标原图
├── build/                    图标转换脚本
├── flake.nix                 Nix devshell
├── justfile                  任务运行器
└── Cargo.toml                工作区配置
```

## 快速开始

### 前置条件

```bash
# Rust（如果还没有）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# just（任务运行器）
cargo install just

# Python + Pillow（仅图标生成需要）
pip install Pillow

# Windows 额外需要：
# - Visual Studio 2022 C++ Build Tools（含 MSVC v143 + Windows SDK）
# - WebView2（Win11 已内置；Win10 由安装程序自动引导下载）
```

### 首次初始化

```bash
just setup    # 安装 tauri-cli + 生成图标
```

### 开发模式

```bash
just dev-teacher   # 教师端，服务在 localhost:3000
just dev-student   # 学生端（另开终端）
```

### 生产构建

```bash
just build-all
# Windows 输出：
#   apps/teacher/src-tauri/target/release/bundle/nsis/*.exe
#   apps/student/src-tauri/target/release/bundle/nsis/*.exe
# macOS 输出：
#   apps/*/src-tauri/target/release/bundle/dmg/*.dmg
```

### 使用 Nix devshell

```bash
nix develop          # 进入开发环境（自动提供 Python/Pillow/just）
just setup           # 首次初始化
just dev-teacher
```

或配合 direnv：
```bash
echo "use flake" > .envrc && direnv allow
```

## 所有 just 任务

```
just              # 列出所有任务
just setup        # 首次初始化
just dev-teacher  # 教师端开发模式
just dev-student  # 学生端开发模式
just build-teacher
just build-student
just build-all
just icons        # 重新生成图标
just clean        # 清理构建产物
```

## 关键设计

**服务器内嵌**：教师端在 Tauri 的 tokio 运行时内直接 `spawn` axum 服务器，无需 sidecar，整个应用只有一个可执行文件。

**Socket.io 兼容**：`socketioxide` 实现 Socket.io v4 协议，前端 `socket.io-client` 代码零改动。

**Host 认证**：教师端启动时生成一次性 UUID token，注入 WebView，socket 连接时携带 token，Rust 服务端据此区分教师/学生角色。

## 与 Electron 版对比

| 项目 | Electron 版 | Tauri 版 |
|---|---|---|
| 安装包 | ~120 MB | ~8 MB |
| 运行时 | Chromium + Node.js | 系统 WebView |
| 进程数 | 2（主进程 + server fork）| 1 |
| 构建工具 | npm + electron-builder + pkg | cargo + tauri-cli |
| Node.js 依赖 | 是 | **否** |
