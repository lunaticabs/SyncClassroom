#!/usr/bin/env python3
"""
convert-icons.py  --  为 Tauri 构建生成所需的全套图标

输出（每个 app）:
  icons/icon.png          256x256，托盘 + 通用
  icons/32x32.png
  icons/128x128.png
  icons/128x128@2x.png
  icons/icon.ico          Windows NSIS 安装程序
  icons/icon.icns         macOS app bundle（需要 pillow 或 cairosvg）

依赖: pip install Pillow
"""
import sys, shutil, struct, io
from pathlib import Path
from PIL import Image

ROOT   = Path(__file__).parent.parent
ASSETS = ROOT / "assets"
BUILD  = ROOT / "build"

APPS = {
    "teacher": {
        "src_ico": BUILD  / "icon-teacher.ico",
        "src_png": ASSETS / "tray-icon.png",
        "out_dir": ROOT / "apps" / "teacher" / "src-tauri" / "icons",
    },
    "student": {
        "src_ico": BUILD  / "icon-student.ico",
        "src_png": ASSETS / "tray-icon.png",
        "out_dir": ROOT / "apps" / "student" / "src-tauri" / "icons",
    },
}

def make_icns(base_img: Image.Image, dest: Path):
    """
    用纯 Python 生成最小可用的 .icns（包含 ic07=128px、ic08=256px 两帧）。
    不依赖 iconutil / macOS 工具链，跨平台可用。
    """
    ICON_TYPES = [
        ("ic07", 128),
        ("ic08", 256),
        ("ic09", 512),
    ]
    chunks = []
    for type_id, size in ICON_TYPES:
        img = base_img.resize((size, size), Image.LANCZOS).convert("RGBA")
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        png_data = buf.getvalue()
        # icns chunk: 4字节 type + 4字节长度（含头部8字节）+ 数据
        chunk_len = 8 + len(png_data)
        chunks.append(type_id.encode() + struct.pack(">I", chunk_len) + png_data)

    total_len = 8 + sum(len(c) for c in chunks)
    with open(dest, "wb") as f:
        f.write(b"icns" + struct.pack(">I", total_len))
        for c in chunks:
            f.write(c)

for name, cfg in APPS.items():
    out = cfg["out_dir"]
    out.mkdir(parents=True, exist_ok=True)

    # 读取基础图像（优先从 .ico 提取最大帧）
    try:
        ico = Image.open(cfg["src_ico"])
        frames = []
        try:
            while True:
                frames.append(ico.copy())
                ico.seek(ico.tell() + 1)
        except EOFError:
            pass
        base = max(frames, key=lambda f: f.size[0]) if frames else ico
    except Exception:
        base = Image.open(cfg["src_png"])

    base = base.convert("RGBA")

    # PNG 各尺寸
    for size in [32, 128, 256]:
        img = base.resize((size, size), Image.LANCZOS)
        img.save(out / f"{size}x{size}.png")
        if size == 128:
            base.resize((256, 256), Image.LANCZOS).save(out / "128x128@2x.png")
        if size == 256:
            img.save(out / "icon.png")

    # Windows .ico
    shutil.copy(cfg["src_ico"], out / "icon.ico")

    # macOS .icns（纯 Python，无需 iconutil）
    make_icns(base, out / "icon.icns")

    print(f"[icons] {name}: OK  ({out.relative_to(ROOT)})")

print("[icons] All done.")
