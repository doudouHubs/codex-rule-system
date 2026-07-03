#!/usr/bin/env python3
"""管理当前项目共享规则库，并支持拾取规则进入当前会话。"""

from __future__ import annotations

import argparse
import datetime as dt
import importlib.util
import json
import re
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
from typing import Any

import yaml


ACTIVE_STATUS = "active"
DEPRECATED_STATUS = "deprecated"
VALID_STATUSES = {ACTIVE_STATUS, DEPRECATED_STATUS}
PICKER_RELATIVE_PATHS = (
    Path("bin") / "rule-picker-win.exe",
    Path("tools") / "rule-picker-win" / "target" / "release" / "rule-picker-win.exe",
    Path("tools") / "rule-picker-win" / "target" / "debug" / "rule-picker-win.exe",
)


def now_iso() -> str:
    """返回带时区的 ISO 时间，方便跨会话比对规则生命周期。"""

    return dt.datetime.now().astimezone().isoformat(timespec="seconds")


def load_session_rules_module() -> Any:
    """复用会话规则脚本的项目根、会话 ID 与写入逻辑，避免重复造轮子。"""

    module_path = Path(__file__).resolve().with_name("session_rules.py")
    spec = importlib.util.spec_from_file_location("session_rules_shared", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load session rules module: {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def split_csv(raw: str) -> list[str]:
    """解析逗号列表参数，保持 CLI 输入足够简单。"""

    return [segment.strip() for segment in raw.split(",") if segment.strip()]


def normalize_tags(raw: str | list[str] | None) -> list[str]:
    """标签统一去重排序，避免同义空白制造重复标签。"""

    if raw is None:
        return []
    if isinstance(raw, str):
        values = split_csv(raw)
    else:
        values = [str(item).strip() for item in raw if str(item).strip()]
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        normalized = re.sub(r"\s+", "-", value.strip())
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        result.append(normalized)
    return result


def load_yaml_file(path: Path, default: Any) -> Any:
    """读取 YAML；文件缺失或损坏时返回默认值，保证 list 首次可用。"""

    if not path.exists():
        return default
    try:
        loaded = yaml.safe_load(path.read_text(encoding="utf-8"))
    except (OSError, yaml.YAMLError):
        return default
    return default if loaded is None else loaded


def write_yaml_file(path: Path, payload: Any) -> None:
    """统一 UTF-8 写 YAML，项目规则里中文不做转义。"""

    path.parent.mkdir(parents=True, exist_ok=True)
    text = yaml.safe_dump(payload, allow_unicode=True, sort_keys=False)
    path.write_text(text, encoding="utf-8", newline="\n")


def resolve_paths(project_root: Path) -> dict[str, Path]:
    """计算项目规则库路径，和会话规则目录保持同级但不同 owner。"""

    root = project_root.resolve()
    project_rules_root = root / ".codex" / "project-rules"
    return {
        "project_root": root,
        "codex_root": root / ".codex",
        "project_rules_root": project_rules_root,
        "rules_file": project_rules_root / "rules.yaml",
        "meta_file": project_rules_root / "meta.yaml",
    }


def ensure_storage(paths: dict[str, Path]) -> None:
    """首次使用时初始化项目规则库。"""

    paths["project_rules_root"].mkdir(parents=True, exist_ok=True)
    if not paths["rules_file"].exists():
        write_yaml_file(paths["rules_file"], {"rules": []})
    if not paths["meta_file"].exists():
        write_yaml_file(
            paths["meta_file"],
            {
                "project_root": str(paths["project_root"]),
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "rules_file": str(paths["rules_file"]),
                "rule_count": 0,
            },
        )


def normalize_rule(raw: Any) -> dict[str, Any] | None:
    """把 YAML 里的项目规则收敛成稳定结构，过滤脏数据。"""

    if not isinstance(raw, dict):
        return None
    rule_id = str(raw.get("id", "")).strip()
    title = str(raw.get("title", "")).strip()
    content = str(raw.get("content", "")).strip()
    status = str(raw.get("status", ACTIVE_STATUS)).strip() or ACTIVE_STATUS
    if status not in VALID_STATUSES:
        status = ACTIVE_STATUS
    if not rule_id or not title or not content:
        return None
    picked_count_raw = raw.get("picked_count", 0)
    try:
        picked_count = int(picked_count_raw)
    except (TypeError, ValueError):
        picked_count = 0
    return {
        "id": rule_id,
        "title": title,
        "content": content,
        "status": status,
        "tags": normalize_tags(raw.get("tags", [])),
        "created_at": str(raw.get("created_at", "")).strip(),
        "updated_at": str(raw.get("updated_at", "")).strip(),
        "picked_count": max(0, picked_count),
        "last_picked_at": str(raw.get("last_picked_at", "")).strip(),
    }


def load_rules(paths: dict[str, Path]) -> list[dict[str, Any]]:
    """读取项目规则库。"""

    payload = load_yaml_file(paths["rules_file"], {"rules": []})
    if not isinstance(payload, dict):
        return []
    raw_rules = payload.get("rules", [])
    if not isinstance(raw_rules, list):
        return []
    rules: list[dict[str, Any]] = []
    for raw in raw_rules:
        rule = normalize_rule(raw)
        if rule is not None:
            rules.append(rule)
    return rules


def save_rules(paths: dict[str, Path], rules: list[dict[str, Any]]) -> None:
    """保存项目规则与 meta，active/deprecated 都计入规则总数。"""

    write_yaml_file(paths["rules_file"], {"rules": rules})
    existing_meta = load_yaml_file(paths["meta_file"], {})
    if not isinstance(existing_meta, dict):
        existing_meta = {}
    created_at = str(existing_meta.get("created_at", "")).strip() or now_iso()
    write_yaml_file(
        paths["meta_file"],
        {
            "project_root": str(paths["project_root"]),
            "created_at": created_at,
            "updated_at": now_iso(),
            "rules_file": str(paths["rules_file"]),
            "rule_count": len(rules),
            "active_rule_count": sum(1 for rule in rules if rule["status"] == ACTIVE_STATUS),
        },
    )


def generate_rule_id(existing_ids: set[str]) -> str:
    """生成项目规则 ID，使用 `pr-` 前缀避免和会话规则 `r-` 混淆。"""

    while True:
        candidate = f"pr-{uuid.uuid4().hex[:8]}"
        if candidate not in existing_ids:
            return candidate


def find_rule_index(rules: list[dict[str, Any]], rule_id: str) -> int:
    """按 ID 定位项目规则。"""

    for index, rule in enumerate(rules):
        if str(rule.get("id", "")).strip() == rule_id:
            return index
    return -1


def filter_rules(
    rules: list[dict[str, Any]],
    *,
    include_all: bool = False,
    tag: str = "",
    query: str = "",
    ids: list[str] | None = None,
) -> list[dict[str, Any]]:
    """统一过滤逻辑，供 list 和 pick 共用。"""

    wanted_ids = {item.strip() for item in ids or [] if item.strip()}
    normalized_tag = tag.strip()
    normalized_query = query.strip().lower()
    result: list[dict[str, Any]] = []
    for rule in rules:
        if not include_all and rule["status"] != ACTIVE_STATUS:
            continue
        if wanted_ids and rule["id"] not in wanted_ids:
            continue
        if normalized_tag and normalized_tag not in rule["tags"]:
            continue
        if normalized_query:
            haystack = " ".join([rule["id"], rule["title"], rule["content"], *rule["tags"]]).lower()
            if normalized_query not in haystack:
                continue
        result.append(rule)
    return result


def build_summary_line(rules: list[dict[str, Any]]) -> str:
    """项目规则库摘要，默认只报 active 标题，方便人眼扫。"""

    active_rules = [rule for rule in rules if rule["status"] == ACTIVE_STATUS]
    if not active_rules:
        return "项目规则库 active 规则 0 条"
    titles = "、".join(rule["title"] for rule in active_rules)
    return f"项目规则库 active 规则 {len(active_rules)} 条：{titles}"


def build_display_rules(rules: list[dict[str, Any]]) -> list[str]:
    """项目规则列表展示层，保留 ID 便于 pick/update/delete。"""

    return [f"{rule['id']} [{rule['status']}] {rule['title']}: {rule['content']}" for rule in rules]


def build_payload(
    *,
    action: str,
    paths: dict[str, Path],
    rules: list[dict[str, Any]],
    selected_rules: list[dict[str, Any]] | None = None,
    changed_rule: dict[str, Any] | None = None,
    message: str = "",
    session_payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """统一 JSON 回执，便于 skill 层和测试脚本读取。"""

    selected = selected_rules or []
    return {
        "action": action,
        "project_root": str(paths["project_root"]),
        "project_rules_root": str(paths["project_rules_root"]),
        "rules_file": str(paths["rules_file"]),
        "meta_file": str(paths["meta_file"]),
        "rule_count": len(rules),
        "active_rule_count": sum(1 for rule in rules if rule["status"] == ACTIVE_STATUS),
        "summary": build_summary_line(rules),
        "message": message,
        "changed_rule": changed_rule,
        "selected_count": len(selected),
        "selected_rules": selected,
        "display_rules": build_display_rules(selected if selected else rules),
        "rules": rules,
        "session_payload": session_payload,
    }


def print_result(payload: dict[str, Any], json_mode: bool) -> None:
    """同时支持 JSON 和简洁文本输出。"""

    if json_mode:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return
    print(payload["summary"])
    display_rules = payload.get("display_rules", [])
    if display_rules:
        for line in display_rules:
            print(f"- {line}")
    else:
        print("- 当前无项目规则")


def resolve_project_context(args: argparse.Namespace) -> tuple[Any, Path, dict[str, Path]]:
    """解析项目根并确保项目规则库存在。"""

    session_rules = load_session_rules_module()
    project_root = Path(args.project_root).resolve() if args.project_root else session_rules.detect_project_root(Path.cwd())
    paths = resolve_paths(project_root)
    ensure_storage(paths)
    return session_rules, project_root, paths


def add_project_rule(
    *,
    project_root: Path,
    title: str,
    content: str,
    tags: str | list[str] | None = None,
) -> dict[str, Any]:
    """
    新增项目共享规则并返回统一 payload。

    该函数是项目规则“新增”的唯一实现 owner。`session_rules.py --scope project`
    和旧 `project_rules.py add` 都必须走这里，避免两个入口各自维护字段和写入语义。
    """

    normalized_title = title.strip()
    normalized_content = content.strip()
    if not normalized_title or not normalized_content:
        raise ValueError("title and content are required")

    paths = resolve_paths(project_root)
    ensure_storage(paths)
    rules = load_rules(paths)
    existing_ids = {rule["id"] for rule in rules}
    timestamp = now_iso()
    changed_rule = {
        "id": generate_rule_id(existing_ids),
        "title": normalized_title,
        "content": normalized_content,
        "status": ACTIVE_STATUS,
        "tags": normalize_tags(tags),
        "created_at": timestamp,
        "updated_at": timestamp,
        "picked_count": 0,
        "last_picked_at": "",
    }
    rules.append(changed_rule)
    save_rules(paths, rules)
    return build_payload(action="add", paths=paths, rules=rules, changed_rule=changed_rule, message="project rule added")


def cmd_add(args: argparse.Namespace) -> int:
    """兼容旧 CLI：项目规则新增转发到共享新增 owner。"""

    session_rules = load_session_rules_module()
    project_root = Path(args.project_root).resolve() if args.project_root else session_rules.detect_project_root(Path.cwd())
    try:
        payload = add_project_rule(
            project_root=project_root,
            title=args.title,
            content=args.content,
            tags=args.tags,
        )
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    print_result(payload, args.json)
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    """查看项目共享规则库。"""

    _, _, paths = resolve_project_context(args)
    rules = load_rules(paths)
    selected = filter_rules(rules, include_all=args.all, tag=args.tag, query=args.query)
    payload = build_payload(action="list", paths=paths, rules=rules, selected_rules=selected, message="project rules listed")
    print_result(payload, args.json)
    return 0


def cmd_update(args: argparse.Namespace) -> int:
    """按 ID 更新项目规则。"""

    if not any([args.title.strip(), args.content.strip(), args.tags.strip(), args.status.strip()]):
        print("at least one of title/content/tags/status is required", file=sys.stderr)
        return 1
    if args.status.strip() and args.status.strip() not in VALID_STATUSES:
        print(f"status must be one of: {', '.join(sorted(VALID_STATUSES))}", file=sys.stderr)
        return 1
    _, _, paths = resolve_project_context(args)
    rules = load_rules(paths)
    index = find_rule_index(rules, args.id.strip())
    if index < 0:
        print(f"project rule not found: {args.id.strip()}", file=sys.stderr)
        return 1
    changed_rule = dict(rules[index])
    if args.title.strip():
        changed_rule["title"] = args.title.strip()
    if args.content.strip():
        changed_rule["content"] = args.content.strip()
    if args.tags.strip():
        changed_rule["tags"] = normalize_tags(args.tags)
    if args.status.strip():
        changed_rule["status"] = args.status.strip()
    changed_rule["updated_at"] = now_iso()
    rules[index] = changed_rule
    save_rules(paths, rules)
    payload = build_payload(action="update", paths=paths, rules=rules, changed_rule=changed_rule, message="project rule updated")
    print_result(payload, args.json)
    return 0


def cmd_delete(args: argparse.Namespace) -> int:
    """默认软删除项目规则，显式 `--hard` 才物理删除。"""

    _, _, paths = resolve_project_context(args)
    rules = load_rules(paths)
    index = find_rule_index(rules, args.id.strip())
    if index < 0:
        print(f"project rule not found: {args.id.strip()}", file=sys.stderr)
        return 1
    changed_rule = dict(rules[index])
    if args.hard:
        del rules[index]
        action = "delete-hard"
        message = "project rule hard deleted"
    else:
        changed_rule["status"] = DEPRECATED_STATUS
        changed_rule["updated_at"] = now_iso()
        rules[index] = changed_rule
        action = "delete"
        message = "project rule deprecated"
    save_rules(paths, rules)
    payload = build_payload(action=action, paths=paths, rules=rules, changed_rule=changed_rule, message=message)
    print_result(payload, args.json)
    return 0


def existing_session_contents(session_rules: Any, project_root: Path, session_id: str) -> set[str]:
    """读取当前会话已有 content，pick 时据此避免重复插入。"""

    session_paths = session_rules.resolve_paths(project_root, session_id)
    session_rules.ensure_storage(session_paths, session_id)
    return {rule["content"] for rule in session_rules.load_rules(session_paths)}


def add_entries_to_session(session_rules: Any, project_root: Path, session_id: str, entries: list[dict[str, str]]) -> dict[str, Any]:
    """把项目规则快照写入当前会话规则。"""

    session_paths = session_rules.resolve_paths(project_root, session_id)
    session_rules.ensure_storage(session_paths, session_id)
    rules = session_rules.load_rules(session_paths)
    existing_ids = {str(rule.get("id", "")).strip() for rule in rules}
    changed_rules: list[dict[str, Any]] = []
    for entry in entries:
        rule_id = session_rules.generate_rule_id(existing_ids)
        existing_ids.add(rule_id)
        timestamp = session_rules.now_iso()
        changed_rule = {
            "id": rule_id,
            "title": entry["title"],
            "content": entry["content"],
            "created_at": timestamp,
            "updated_at": timestamp,
        }
        rules.append(changed_rule)
        changed_rules.append(changed_rule)
    session_rules.save_rules(session_paths, rules, session_id)
    return session_rules.build_payload(
        action="pick" if len(changed_rules) == 1 else "pick-batch",
        paths=session_paths,
        session_id=session_id,
        rules=rules,
        changed_rule=changed_rules[0] if len(changed_rules) == 1 else None,
        changed_rules=changed_rules,
        message="project rules picked into session",
    )


def plugin_root() -> Path:
    """返回插件仓库根目录，供脚本从源码树或 Codex cache 中定位内置工具。"""

    return Path(__file__).resolve().parent.parent


def resolve_picker_executable() -> Path:
    """定位 Windows 原生 picker。

    优先使用随插件发布的 `bin/rule-picker-win.exe`；开发态没有复制 exe 时，
    允许直接使用 Rust crate 的 debug/release 产物，方便本地迭代。
    """

    root = plugin_root()
    for relative_path in PICKER_RELATIVE_PATHS:
        candidate = root / relative_path
        if candidate.exists():
            return candidate
    raise FileNotFoundError(
        "rule picker executable not found; build tools/rule-picker-win or place bin/rule-picker-win.exe"
    )


def build_picker_rules(selected_rules: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """把项目规则压成 picker 需要的最小 JSON，避免 UI 工具依赖 YAML 结构。"""

    return [
        {
            "id": rule["id"],
            "title": rule["title"],
            "content": rule["content"],
            "status": rule["status"],
            "tags": rule["tags"],
        }
        for rule in selected_rules
    ]


def run_picker(selected_rules: list[dict[str, Any]], query: str) -> dict[str, Any] | None:
    """启动 Win32 picker 并返回用户确认的选择与编辑结果。

    返回 `None` 表示用户取消；返回 payload 表示用户确认。payload 中
    `selected_ids` 用于 pick，`updates` 用于先保存 UI 内编辑过的项目规则。
    这里用临时 JSON 文件作为进程边界，避免命令行长度、转义和中文编码翻车。
    """

    picker = resolve_picker_executable()
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as handle:
        json.dump(build_picker_rules(selected_rules), handle, ensure_ascii=False)
        rules_file = Path(handle.name)
    try:
        command = [str(picker), "--rules", str(rules_file)]
        if query.strip():
            command.extend(["--query", query.strip()])
        completed = subprocess.run(command, check=False, capture_output=True, text=True, encoding="utf-8")
    finally:
        try:
            rules_file.unlink()
        except OSError:
            pass

    if completed.returncode != 0:
        message = completed.stderr.strip() or completed.stdout.strip() or "rule picker failed"
        raise RuntimeError(message)
    try:
        payload = json.loads(completed.stdout.strip() or "{}")
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"rule picker returned invalid JSON: {exc}") from exc
    if bool(payload.get("cancelled", False)):
        return None
    selected_ids = payload.get("selected_ids", [])
    if not isinstance(selected_ids, list):
        raise RuntimeError("rule picker returned invalid selected_ids")
    updates = payload.get("updates", [])
    if not isinstance(updates, list):
        raise RuntimeError("rule picker returned invalid updates")
    return {
        "selected_ids": [str(item).strip() for item in selected_ids if str(item).strip()],
        "updates": updates,
    }


def apply_picker_updates(
    *,
    paths: dict[str, Path],
    rules: list[dict[str, Any]],
    updates: list[Any],
    allowed_ids: set[str],
) -> list[dict[str, Any]]:
    """保存 picker 内编辑过的项目规则。

    Win32 picker 只负责交互，不直接写 YAML；这里统一做 ID 边界、字段校验、
    时间戳刷新和落盘，避免 UI 工具变成第二个项目规则存储 owner。
    """

    changed_rules: list[dict[str, Any]] = []
    if not updates:
        return changed_rules

    index_by_id = {rule["id"]: index for index, rule in enumerate(rules)}
    timestamp = now_iso()
    for raw_update in updates:
        if not isinstance(raw_update, dict):
            raise ValueError("rule picker returned invalid update item")
        rule_id = str(raw_update.get("id", "")).strip()
        if not rule_id:
            raise ValueError("rule picker returned update without id")
        if rule_id not in allowed_ids:
            raise ValueError(f"rule picker returned update outside visible rule set: {rule_id}")
        index = index_by_id.get(rule_id)
        if index is None:
            raise ValueError(f"project rule not found: {rule_id}")

        title = str(raw_update.get("title", "")).strip()
        content = str(raw_update.get("content", "")).strip()
        status = str(raw_update.get("status", ACTIVE_STATUS)).strip() or ACTIVE_STATUS
        if not title or not content:
            raise ValueError("updated project rule title and content are required")
        if status not in VALID_STATUSES:
            raise ValueError(f"status must be one of: {', '.join(sorted(VALID_STATUSES))}")

        changed_rule = dict(rules[index])
        changed_rule["title"] = title
        changed_rule["content"] = content
        changed_rule["status"] = status
        changed_rule["tags"] = normalize_tags(raw_update.get("tags", []))
        changed_rule["updated_at"] = timestamp
        rules[index] = changed_rule
        changed_rules.append(changed_rule)

    save_rules(paths, rules)
    return changed_rules


def pick_project_rules(
    *,
    session_rules: Any,
    project_root: Path,
    paths: dict[str, Path],
    rules: list[dict[str, Any]],
    selected: list[dict[str, Any]],
    session_id_raw: str,
) -> dict[str, Any]:
    """把已确定的项目规则集合复制进当前会话。

    CLI、tag/query 和 Win32 UI 都走这个函数，保证“快照复制、去重、统计回写”
    只有一个 owner，不因为新增交互方式扩散出第二套 pick 逻辑。
    """

    if not selected:
        return build_payload(
            action="pick",
            paths=paths,
            rules=rules,
            selected_rules=[],
            message="no active project rules matched",
            session_payload=None,
        )

    session_id = session_rules.resolve_session_id(session_id_raw)
    existing_contents = existing_session_contents(session_rules, project_root, session_id)
    entries: list[dict[str, str]] = []
    picked_project_ids: set[str] = set()
    for rule in selected:
        # pick 是快照复制，不是动态引用；同时避免同一会话重复插入相同正文。
        if rule["content"] in existing_contents:
            continue
        entries.append({"title": "项目规则", "content": rule["content"]})
        existing_contents.add(rule["content"])
        picked_project_ids.add(rule["id"])
    session_payload: dict[str, Any] | None = None
    if entries:
        session_payload = add_entries_to_session(session_rules, project_root, session_id, entries)
        timestamp = now_iso()
        for rule in rules:
            if rule["id"] in picked_project_ids:
                rule["picked_count"] = int(rule.get("picked_count", 0)) + 1
                rule["last_picked_at"] = timestamp
                rule["updated_at"] = timestamp
        save_rules(paths, rules)
    payload = build_payload(
        action="pick",
        paths=paths,
        rules=rules,
        selected_rules=selected,
        message="project rules picked into current session" if entries else "matched project rules already exist in current session",
        session_payload=session_payload,
    )
    return payload


def cmd_pick(args: argparse.Namespace) -> int:
    """拾取项目规则快照进入当前会话规则。"""

    session_rules, project_root, paths = resolve_project_context(args)
    rules = load_rules(paths)
    ids = split_csv(args.ids)
    if args.ui:
        if sys.platform != "win32":
            print("--ui is only supported on Windows", file=sys.stderr)
            return 1
        # UI 是“管理窗口”，不是纯 pick 结果列表：必须让窗口始终打开，
        # 并且能管理 deprecated 规则。query 只作为窗口初始搜索词交给 picker，
        # 不在 Python 层提前过滤掉候选，否则搜索无匹配时会把管理窗口饿成空进程。
        selected = filter_rules(rules, include_all=True, tag=args.tag, ids=ids)
        try:
            picker_payload = run_picker(selected, args.query)
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            print(str(exc), file=sys.stderr)
            return 1
        if picker_payload is None:
            payload = build_payload(
                action="pick-ui-cancel",
                paths=paths,
                rules=rules,
                selected_rules=[],
                message="rule picker cancelled",
                session_payload=None,
            )
            print_result(payload, args.json)
            return 0
        allowed_ids = {rule["id"] for rule in selected}
        try:
            changed_rules = apply_picker_updates(
                paths=paths,
                rules=rules,
                updates=picker_payload.get("updates", []),
                allowed_ids=allowed_ids,
            )
        except ValueError as exc:
            print(str(exc), file=sys.stderr)
            return 1
        selected_ids = set(picker_payload.get("selected_ids", []))
        selected = [
            rule
            for rule in rules
            if rule["id"] in allowed_ids.intersection(selected_ids) and rule["status"] == ACTIVE_STATUS
        ]
        if not selected:
            payload = build_payload(
                action="pick-ui-update" if changed_rules else "pick-ui-empty",
                paths=paths,
                rules=rules,
                selected_rules=[],
                changed_rule=changed_rules[0] if len(changed_rules) == 1 else None,
                message=(
                    "project rules updated; no active project rules selected"
                    if changed_rules
                    else "no active project rules selected"
                ),
                session_payload=None,
            )
            print_result(payload, args.json)
            return 0
    else:
        selected = filter_rules(rules, include_all=False, tag=args.tag, query=args.query, ids=ids)
    if not selected:
        print("no active project rules matched", file=sys.stderr)
        return 1
    payload = pick_project_rules(
        session_rules=session_rules,
        project_root=project_root,
        paths=paths,
        rules=rules,
        selected=selected,
        session_id_raw=args.session_id,
    )
    print_result(payload, args.json)
    return 0


def build_parser() -> argparse.ArgumentParser:
    """构造 CLI。"""

    parser = argparse.ArgumentParser(description="Manage project-shared rules under .codex/project-rules/.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    add_parser = subparsers.add_parser("add", help="Add one project-shared rule")
    add_parser.add_argument("--title", required=True, help="Project rule title")
    add_parser.add_argument("--content", required=True, help="Project rule content")
    add_parser.add_argument("--tags", default="", help="Comma-separated tags")
    add_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    add_parser.add_argument("--json", action="store_true", help="Output JSON")
    add_parser.set_defaults(func=cmd_add)

    list_parser = subparsers.add_parser("list", help="List project-shared rules")
    list_parser.add_argument("--all", action="store_true", help="Include deprecated rules")
    list_parser.add_argument("--tag", default="", help="Filter by tag")
    list_parser.add_argument("--query", default="", help="Filter by keyword")
    list_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    list_parser.add_argument("--json", action="store_true", help="Output JSON")
    list_parser.set_defaults(func=cmd_list)

    update_parser = subparsers.add_parser("update", help="Update one project-shared rule")
    update_parser.add_argument("--id", required=True, help="Project rule id")
    update_parser.add_argument("--title", default="", help="New title")
    update_parser.add_argument("--content", default="", help="New content")
    update_parser.add_argument("--tags", default="", help="New comma-separated tags")
    update_parser.add_argument("--status", default="", help="New status: active or deprecated")
    update_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    update_parser.add_argument("--json", action="store_true", help="Output JSON")
    update_parser.set_defaults(func=cmd_update)

    delete_parser = subparsers.add_parser("delete", help="Deprecate or hard-delete one project-shared rule")
    delete_parser.add_argument("--id", required=True, help="Project rule id")
    delete_parser.add_argument("--hard", action="store_true", help="Physically delete the project rule")
    delete_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    delete_parser.add_argument("--json", action="store_true", help="Output JSON")
    delete_parser.set_defaults(func=cmd_delete)

    pick_parser = subparsers.add_parser("pick", help="Copy matched active project rules into current-session rules")
    pick_parser.add_argument("--ids", default="", help="Comma-separated project rule ids")
    pick_parser.add_argument("--tag", default="", help="Pick active rules by tag")
    pick_parser.add_argument("--query", default="", help="Pick active rules by keyword")
    pick_parser.add_argument("--ui", action="store_true", help="Open the native Windows picker before copying rules")
    pick_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    pick_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    pick_parser.add_argument("--json", action="store_true", help="Output JSON")
    pick_parser.set_defaults(func=cmd_pick)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
