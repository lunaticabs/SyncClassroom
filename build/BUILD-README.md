# 构建说明

## 前置条件

| 工具 | 版本 | 安装 |
|------|------|------|
| Rust | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| Node.js | 20+ | https://nodejs.org |
| pnpm | 9+ | `npm install -g pnpm` |
| Python | 3.11+ | https://python.org |
| Pillow | latest | `pip install Pillow` |
| VS C++ Build Tools | 2022 | 含 MSVC v143 + Windows SDK |
| WebView2 | 内置/自动 | Win11 已内置；Win10 安装时自动引导 |

## 步骤

```bash
# 1. 安装 JS 依赖
pnpm install

# 2. 生成图标（首次 / 更换图标后执行）
python build/convert-icons.py

# 3. 构建两端安装包
pnpm build:all
```

输出位置：
- `apps/teacher/src-tauri/target/release/bundle/nsis/*.exe`
- `apps/student/src-tauri/target/release/bundle/nsis/*.exe`

## 开发模式

```bash
pnpm dev:teacher   # 教师端，服务在 localhost:3000
pnpm dev:student   # 学生端（另开终端）
```

## CI/CD

- Push `feature/*` 分支 → build-branch.yml，产物保留 7 天
- Push tag `v*` → release.yml，自动创建 GitHub Release
