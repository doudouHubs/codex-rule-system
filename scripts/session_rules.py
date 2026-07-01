#!/usr/bin/env python3
"""管理当前项目当前会话规则的最小 CRUD 工具。"""

from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
import os
import re
import sys
import uuid
from pathlib import Path
from typing import Any

import yaml


# 统一允许的会话环境变量优先级。
# 先吃 Codex 原生线程 ID；拿不到时再降级到终端会话标识，避免每次都生成新目录。
SESSION_ENV_KEYS = ("CODEX_THREAD_ID", "WT_SESSION", "SESSIONNAME")
SESSION_SCOPE = "session"
PROJECT_SCOPE = "project"
VALID_ADD_SCOPES = {SESSION_SCOPE, PROJECT_SCOPE}


def now_iso() -> str:
    """返回带时区的 ISO 时间，便于人读和脚本比对。"""
    return dt.datetime.now().astimezone().isoformat(timespec="seconds")


def sanitize_segment(value: str) -> str:
    """清洗目录名片段，避免把环境变量里的奇怪字符直接写进路径。"""
    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip())
    cleaned = re.sub(r"-{2,}", "-", cleaned).strip("-")
    return cleaned or "unknown-session"


def detect_project_root(start: Path) -> Path:
    """
    从当前目录向上推导项目根。

    规则尽量贴近现有仓库习惯：
    1. 最近的 `.git` 目录优先，避免误吸到用户主目录的 `.codex`
    2. 其次是 `AGENTS.md`
    3. 最后才是 `.codex`
    4. 如果都没有，就退回当前目录
    """

    current = start.resolve()
    if current.is_file():
        current = current.parent

    for candidate in [current, *current.parents]:
        if (candidate / ".git").exists():
            return candidate
        if (candidate / "AGENTS.md").exists():
            return candidate
        if (candidate / ".codex").exists():
            return candidate
    return current


def resolve_session_id(explicit_session_id: str) -> str:
    """
    解析当前会话 ID。

    正常路径走 `CODEX_THREAD_ID`，这样同一线程多次调用会命中同一目录。
    只有在环境真的拿不到时，才退回到时间戳兜底。
    """

    if explicit_session_id.strip():
        return sanitize_segment(explicit_session_id)

    for env_key in SESSION_ENV_KEYS:
        env_value = os.environ.get(env_key, "").strip()
        if env_value:
            return sanitize_segment(env_value)

    fallback = f"session-{dt.datetime.now().strftime('%Y%m%d-%H%M%S')}"
    return sanitize_segment(fallback)


def resolve_paths(project_root: Path, session_id: str) -> dict[str, Path]:
    """
    统一计算会话规则目录。

    每次会话一个目录，结构固定为：
    `<project_root>/.codex/session-rules/<session_id>/`
    """

    root = project_root.resolve()
    codex_root = root / ".codex"
    session_root = codex_root / "session-rules" / session_id
    return {
        "project_root": root,
        "codex_root": codex_root,
        "session_rules_root": codex_root / "session-rules",
        "session_root": session_root,
        "rules_file": session_root / "rules.yaml",
        "meta_file": session_root / "meta.yaml",
    }


def load_yaml_file(path: Path, default: Any) -> Any:
    """读取 YAML；文件不存在或损坏时返回调用方给的默认值。"""

    if not path.exists():
        return default
    try:
        loaded = yaml.safe_load(path.read_text(encoding="utf-8"))
    except (OSError, yaml.YAMLError):
        return default
    return default if loaded is None else loaded


def write_yaml_file(path: Path, payload: Any) -> None:
    """统一用 UTF-8 写 YAML，保证中文规则内容不被转义成鬼画符。"""

    path.parent.mkdir(parents=True, exist_ok=True)
    text = yaml.safe_dump(payload, allow_unicode=True, sort_keys=False)
    path.write_text(text, encoding="utf-8", newline="\n")


def ensure_storage(paths: dict[str, Path], session_id: str) -> None:
    """
    确保当前会话目录与基础文件存在。

    `rule-add` 和 `rule-list` 都允许首次调用时自动初始化。
    这里把初始化逻辑做成共享入口，避免 4 个命令各写一套。
    """

    session_root = paths["session_root"]
    session_root.mkdir(parents=True, exist_ok=True)

    rules_file = paths["rules_file"]
    if not rules_file.exists():
        write_yaml_file(rules_file, {"rules": []})

    meta_file = paths["meta_file"]
    if not meta_file.exists():
        write_yaml_file(
            meta_file,
            {
                "session_id": session_id,
                "project_root": str(paths["project_root"]),
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "rules_file": str(rules_file),
                "rule_count": 0,
            },
        )


def load_rules(paths: dict[str, Path]) -> list[dict[str, Any]]:
    """
    读取当前会话的规则列表。

    文件格式固定为：
    `rules: [{id, title, content, created_at, updated_at}, ...]`
    """

    payload = load_yaml_file(paths["rules_file"], {"rules": []})
    if not isinstance(payload, dict):
        return []
    raw_rules = payload.get("rules", [])
    if not isinstance(raw_rules, list):
        return []

    rules: list[dict[str, Any]] = []
    for item in raw_rules:
        if not isinstance(item, dict):
            continue
        rule_id = str(item.get("id", "")).strip()
        title = str(item.get("title", "")).strip()
        content = str(item.get("content", "")).strip()
        if not rule_id or not title or not content:
            continue
        rules.append(
            {
                "id": rule_id,
                "title": title,
                "content": content,
                "created_at": str(item.get("created_at", "")).strip(),
                "updated_at": str(item.get("updated_at", "")).strip(),
            }
        )
    return rules


def save_rules(paths: dict[str, Path], rules: list[dict[str, Any]], session_id: str) -> None:
    """
    写回 rules.yaml 和 meta.yaml。

    这里把 rule_count 和 updated_at 一并刷新，后面 list/summary 不需要再自己补状态。
    """

    write_yaml_file(paths["rules_file"], {"rules": rules})

    existing_meta = load_yaml_file(paths["meta_file"], {})
    if not isinstance(existing_meta, dict):
        existing_meta = {}
    created_at = str(existing_meta.get("created_at", "")).strip() or now_iso()
    meta_payload = {
        "session_id": session_id,
        "project_root": str(paths["project_root"]),
        "created_at": created_at,
        "updated_at": now_iso(),
        "rules_file": str(paths["rules_file"]),
        "rule_count": len(rules),
    }
    write_yaml_file(paths["meta_file"], meta_payload)


def generate_rule_id(existing_ids: set[str]) -> str:
    """生成短 ID，避免用户靠标题改规则时把自己绕晕。"""

    while True:
        candidate = f"r-{uuid.uuid4().hex[:8]}"
        if candidate not in existing_ids:
            return candidate


def split_batch_segments(raw: str) -> list[str]:
    """
    用中文分号拆分批量输入。

    这里明确只把 `；` 当批量分隔符：
    - 保持和用户要求一致，别自作聪明把半角分号也混进来
    - 自动去掉首尾空白与空段，避免出现 `a；；b` 这种脏输入时把空规则塞进去
    """

    if "；" not in raw:
        value = raw.strip()
        return [value] if value else []
    return [segment.strip() for segment in raw.split("；") if segment.strip()]


def build_add_entries(title_raw: str, content_raw: str) -> list[dict[str, str]]:
    """
    解析 rule-add 的单条/批量输入。

    规则：
    1. `title` 和 `content` 都不含 `；` 时，按单条规则处理
    2. 任一侧含 `；` 时进入批量模式
    3. 批量模式要求标题数和内容数一一对应；数量不一致直接报错

    这么做虽然朴素，但边界清楚，不会因为“猜用户意思”把规则写歪。
    """

    title_segments = split_batch_segments(title_raw)
    content_segments = split_batch_segments(content_raw)

    if not title_segments or not content_segments:
        raise ValueError("title and content are required")

    is_batch = len(title_segments) > 1 or len(content_segments) > 1
    if not is_batch:
        return [{"title": title_segments[0], "content": content_segments[0]}]

    if len(title_segments) != len(content_segments):
        raise ValueError(
            "batch add requires the same number of title/content segments when using Chinese semicolon `；`"
        )

    return [
        {"title": title_segments[index], "content": content_segments[index]}
        for index in range(len(title_segments))
    ]


def build_summary_line(rules: list[dict[str, Any]]) -> str:
    """按产品约定输出平时摘要：数量 + 标题。"""

    if not rules:
        return "当前会话规则 0 条"
    titles = "、".join(str(rule.get("title", "")).strip() for rule in rules)
    return f"当前会话规则 {len(rules)} 条：{titles}"


def build_display_rules(rules: list[dict[str, Any]]) -> list[str]:
    """
    构造默认展示层。

    用户已经明确要求：展示层默认只保留 `content`，一行一条。
    所以这里把完整规则列表压成：
    `- 当前项目始终为响应式`
    而不是把 `id/标题/时间戳/path` 再端出来刷屏。
    """

    return [str(rule.get("content", "")).strip() for rule in rules if str(rule.get("content", "")).strip()]


def build_payload(
    *,
    action: str,
    paths: dict[str, Path],
    session_id: str,
    rules: list[dict[str, Any]],
    changed_rule: dict[str, Any] | None = None,
    changed_rules: list[dict[str, Any]] | None = None,
    message: str = "",
) -> dict[str, Any]:
    """构造统一回执，保证 4 个 skill 的读法一致，不会一会儿像人话一会儿像乱码。"""

    return {
        "action": action,
        "project_root": str(paths["project_root"]),
        "session_id": session_id,
        "session_root": str(paths["session_root"]),
        "rules_file": str(paths["rules_file"]),
        "meta_file": str(paths["meta_file"]),
        "rule_count": len(rules),
        "rule_titles": [str(rule.get("title", "")).strip() for rule in rules],
        "summary": build_summary_line(rules),
        "message": message,
        "changed_rule": changed_rule,
        "changed_rules": changed_rules or ([] if changed_rule is None else [changed_rule]),
        "display_rules": build_display_rules(rules),
        "rules": rules,
    }


def load_project_rules_module() -> Any:
    """按需加载项目规则模块，让 `rule-add --scope project` 复用唯一项目新增 owner。"""

    module_path = Path(__file__).resolve().with_name("project_rules.py")
    spec = importlib.util.spec_from_file_location("project_rules_shared", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load project rules module: {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def print_result(payload: dict[str, Any], json_mode: bool) -> None:
    """同时支持 JSON 和人类可读输出，方便 skill 指令和人工排查共用。"""

    if json_mode:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return

    if payload["action"] == "summary":
        print(payload["summary"])
        return

    print(f"当前项目当前会话规则共 {payload['rule_count']} 条。")
    display_rules = payload.get("display_rules", [])
    if display_rules:
        for content in display_rules:
            print(f"- {content}")
    else:
        print("- 当前无规则")
    return

def find_rule_index(rules: list[dict[str, Any]], rule_id: str) -> int:
    """按 ID 定位规则；找不到返回 -1。"""

    for index, rule in enumerate(rules):
        if str(rule.get("id", "")).strip() == rule_id:
            return index
    return -1


def cmd_add_session(args: argparse.Namespace) -> int:
    """新增一条或多条会话规则。"""

    try:
        add_entries = build_add_entries(args.title, args.content)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    project_root = Path(args.project_root).resolve() if args.project_root else detect_project_root(Path.cwd())
    session_id = resolve_session_id(args.session_id)
    paths = resolve_paths(project_root, session_id)
    ensure_storage(paths, session_id)

    rules = load_rules(paths)
    existing_ids = {str(rule.get("id", "")).strip() for rule in rules}
    changed_rules: list[dict[str, Any]] = []
    for entry in add_entries:
        rule_id = generate_rule_id(existing_ids)
        existing_ids.add(rule_id)
        timestamp = now_iso()
        changed_rule = {
            "id": rule_id,
            "title": entry["title"],
            "content": entry["content"],
            "created_at": timestamp,
            "updated_at": timestamp,
        }
        rules.append(changed_rule)
        changed_rules.append(changed_rule)
    save_rules(paths, rules, session_id)

    payload = build_payload(
        action="add-batch" if len(changed_rules) > 1 else "add",
        paths=paths,
        session_id=session_id,
        rules=rules,
        changed_rule=changed_rules[0] if len(changed_rules) == 1 else None,
        changed_rules=changed_rules,
        message="rules added" if len(changed_rules) > 1 else "rule added",
    )
    print_result(payload, args.json)
    return 0


def cmd_add_project(args: argparse.Namespace) -> int:
    """新增项目共享规则，作为 `$rule-add --scope project` 的实现入口。"""

    if "；" in args.title or "；" in args.content:
        print("project-scope add does not support Chinese semicolon batch mode", file=sys.stderr)
        return 1

    project_root = Path(args.project_root).resolve() if args.project_root else detect_project_root(Path.cwd())
    project_rules = load_project_rules_module()
    try:
        payload = project_rules.add_project_rule(
            project_root=project_root,
            title=args.title,
            content=args.content,
            tags=args.tags,
        )
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    project_rules.print_result(payload, args.json)
    return 0


def cmd_add(args: argparse.Namespace) -> int:
    """
    统一新增入口。

    `session` 保留旧 `$rule-add` 行为；`project` 复用项目规则新增 owner。
    这么做是为了收敛“新增规则”的路由，同时不破坏项目规则独立生命周期字段。
    """

    if args.scope == SESSION_SCOPE:
        return cmd_add_session(args)
    if args.scope == PROJECT_SCOPE:
        return cmd_add_project(args)
    print(f"scope must be one of: {', '.join(sorted(VALID_ADD_SCOPES))}", file=sys.stderr)
    return 1


def cmd_update(args: argparse.Namespace) -> int:
    """按 ID 更新标题或正文。"""

    if not args.title.strip() and not args.content.strip():
        print("at least one of title/content is required", file=sys.stderr)
        return 1

    project_root = Path(args.project_root).resolve() if args.project_root else detect_project_root(Path.cwd())
    session_id = resolve_session_id(args.session_id)
    paths = resolve_paths(project_root, session_id)
    ensure_storage(paths, session_id)

    rules = load_rules(paths)
    index = find_rule_index(rules, args.id.strip())
    if index < 0:
        print(f"rule not found: {args.id.strip()}", file=sys.stderr)
        return 1

    rule = dict(rules[index])
    if args.title.strip():
        rule["title"] = args.title.strip()
    if args.content.strip():
        rule["content"] = args.content.strip()
    rule["updated_at"] = now_iso()
    rules[index] = rule
    save_rules(paths, rules, session_id)

    payload = build_payload(
        action="update",
        paths=paths,
        session_id=session_id,
        rules=rules,
        changed_rule=rule,
        message="rule updated",
    )
    print_result(payload, args.json)
    return 0


def cmd_delete(args: argparse.Namespace) -> int:
    """删除规则；有 `id` 删单条，无 `id` 清空当前会话全部规则。"""

    rule_id = args.id.strip()

    project_root = Path(args.project_root).resolve() if args.project_root else detect_project_root(Path.cwd())
    session_id = resolve_session_id(args.session_id)
    paths = resolve_paths(project_root, session_id)
    ensure_storage(paths, session_id)

    rules = load_rules(paths)
    if not rule_id:
        cleared_rules = list(rules)
        rules = []
        save_rules(paths, rules, session_id)

        payload = build_payload(
            action="delete-all",
            paths=paths,
            session_id=session_id,
            rules=rules,
            changed_rule=None,
            changed_rules=cleared_rules,
            message="all session rules deleted",
        )
        print_result(payload, args.json)
        return 0

    index = find_rule_index(rules, rule_id)
    if index < 0:
        print(f"rule not found: {rule_id}", file=sys.stderr)
        return 1

    changed_rule = dict(rules[index])
    del rules[index]
    save_rules(paths, rules, session_id)

    payload = build_payload(
        action="delete",
        paths=paths,
        session_id=session_id,
        rules=rules,
        changed_rule=changed_rule,
        message="rule deleted",
    )
    print_result(payload, args.json)
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    """查看当前项目当前会话规则；默认全量，也支持摘要。"""

    project_root = Path(args.project_root).resolve() if args.project_root else detect_project_root(Path.cwd())
    session_id = resolve_session_id(args.session_id)
    paths = resolve_paths(project_root, session_id)
    ensure_storage(paths, session_id)

    rules = load_rules(paths)
    payload = build_payload(
        action="list",
        paths=paths,
        session_id=session_id,
        rules=rules,
        changed_rule=None,
        message="rules listed",
    )

    if args.summary:
        summary_payload = {
            "action": "summary",
            "project_root": payload["project_root"],
            "session_id": payload["session_id"],
            "session_root": payload["session_root"],
            "rules_file": payload["rules_file"],
            "rule_count": payload["rule_count"],
            "rule_titles": payload["rule_titles"],
            "summary": payload["summary"],
        }
        print_result(summary_payload, args.json)
        return 0

    print_result(payload, args.json)
    return 0


def build_parser() -> argparse.ArgumentParser:
    """构造 CLI。"""

    parser = argparse.ArgumentParser(
        description="Manage current-project current-session rules under .codex/session-rules/<session_id>/."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    add_parser = subparsers.add_parser("add", help="Add one rule; defaults to current-session scope")
    add_parser.add_argument(
        "--scope",
        choices=sorted(VALID_ADD_SCOPES),
        default=SESSION_SCOPE,
        help="Rule target scope: session for current conversation, project for shared project library",
    )
    add_parser.add_argument("--title", required=True, help="Rule title")
    add_parser.add_argument("--content", required=True, help="Rule content")
    add_parser.add_argument("--tags", default="", help="Comma-separated tags for project-scope rules")
    add_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    add_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    add_parser.add_argument("--json", action="store_true", help="Output JSON instead of human-readable text")
    add_parser.set_defaults(func=cmd_add)

    update_parser = subparsers.add_parser("update", help="Update one session rule by id")
    update_parser.add_argument("--id", required=True, help="Rule id")
    update_parser.add_argument("--title", default="", help="New title")
    update_parser.add_argument("--content", default="", help="New content")
    update_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    update_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    update_parser.add_argument("--json", action="store_true", help="Output JSON instead of human-readable text")
    update_parser.set_defaults(func=cmd_update)

    delete_parser = subparsers.add_parser("delete", help="Delete one session rule by id, or clear all rules when id is omitted")
    delete_parser.add_argument("--id", default="", help="Optional rule id; omit to clear all session rules")
    delete_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    delete_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    delete_parser.add_argument("--json", action="store_true", help="Output JSON instead of human-readable text")
    delete_parser.set_defaults(func=cmd_delete)

    list_parser = subparsers.add_parser("list", help="List current-session rules")
    list_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    list_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    list_parser.add_argument("--summary", action="store_true", help="Only show count and titles")
    list_parser.add_argument("--json", action="store_true", help="Output JSON instead of human-readable text")
    list_parser.set_defaults(func=cmd_list)

    return parser


def main() -> int:
    """CLI 入口。"""

    parser = build_parser()
    args = parser.parse_args()
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
