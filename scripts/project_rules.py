#!/usr/bin/env python3
"""兼容入口：转发项目规则命令到 Rust `rule-system.exe`。"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


COMMAND_MAP = {
    "add": "add",
    "list": "project-list",
    "update": "project-update",
    "delete": "project-delete",
    "pick": "pick",
    "scan": "scan",
}


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


def translate_args(argv: list[str]) -> list[str]:
    """把旧 Python 子命令映射到 Rust 单 exe 子命令。"""

    if not argv:
        return argv
    command = argv[0]
    if command == "pick" and "--ui" in argv:
        return ["check", *[arg for arg in argv[1:] if arg != "--ui"]]
    return [COMMAND_MAP.get(command, command), *argv[1:]]


def main() -> int:
    """执行转发。"""

    try:
        exe = resolve_exe()
    except FileNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    completed = subprocess.run([str(exe), *translate_args(sys.argv[1:])], check=False)
    return int(completed.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
