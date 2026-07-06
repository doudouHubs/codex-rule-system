#!/usr/bin/env python3
"""兼容入口：转发到 Rust `rule-system.exe`。"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import datetime as dt
from pathlib import Path


SESSION_ENV_KEYS = ("CODEX_THREAD_ID", "WT_SESSION", "SESSIONNAME")


def sanitize_segment(value: str) -> str:
    """保留给 rule-capture 复用的会话 ID 清洗函数。"""

    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip())
    cleaned = re.sub(r"-{2,}", "-", cleaned).strip("-")
    return cleaned or "unknown-session"


def detect_project_root(start: Path) -> Path:
    """保留给 rule-capture 复用的项目根解析函数。"""

    current = start.resolve()
    if current.is_file():
        current = current.parent
    for candidate in [current, *current.parents]:
        if (candidate / ".git").exists():
            return candidate
        if (candidate / "AGENTS.md").exists():
            return candidate
        if (candidate / ".codex-rules").exists():
            return candidate
        if (candidate / ".codex").exists():
            return candidate
    return current


def resolve_session_id(explicit_session_id: str) -> str:
    """保留给 rule-capture 复用的会话 ID 解析函数。"""

    if explicit_session_id.strip():
        return sanitize_segment(explicit_session_id)
    for env_key in SESSION_ENV_KEYS:
        env_value = os.environ.get(env_key, "").strip()
        if env_value:
            return sanitize_segment(env_value)
    return sanitize_segment(f"session-{dt.datetime.now().strftime('%Y%m%d-%H%M%S')}")


def plugin_root() -> Path:
    """返回插件根目录。"""

    return Path(__file__).resolve().parent.parent


def resolve_exe() -> Path:
    """优先使用发布内置 exe，其次使用开发态 release/debug 产物。"""

    root = plugin_root()
    candidates = (
        root / "bin" / "rule-system.exe",
        root / "tools" / "rule-system" / "target" / "release" / "rule-system.exe",
        root / "tools" / "rule-system" / "target" / "debug" / "rule-system.exe",
    )
    for candidate in candidates:
        if candidate.exists():
            return candidate
    raise FileNotFoundError("rule-system.exe not found; run scripts/build-rule-system.ps1")


def main() -> int:
    """把旧 Python CLI 参数原样转给 Rust 单 exe。"""

    try:
        exe = resolve_exe()
    except FileNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    completed = subprocess.run([str(exe), *sys.argv[1:]], check=False)
    return int(completed.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
