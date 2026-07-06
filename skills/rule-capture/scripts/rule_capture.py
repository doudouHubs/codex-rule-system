#!/usr/bin/env python3
"""从最近上下文里提炼原子规则候选。"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any


# 规则信号词分级：强信号优先，弱信号用于补分。
STRONG_RULE_CUES = (
    "不要",
    "别",
    "不能",
    "只",
    "仅",
    "必须",
    "先",
    "再",
    "优先",
    "默认",
    "保持",
    "支持",
    "保留",
    "改为",
    "用于",
    "统一",
    "固定",
    "使用",
    "输出",
    "回复",
    "验证",
    "确认",
    "限制",
    "限定",
)
MEDIUM_RULE_CUES = (
    "应该",
    "需要",
    "希望",
    "建议",
    "格式",
    "结论",
    "证据",
    "命令",
    "路径",
    "中文",
    "英文",
    "空格",
    "展示",
    "清理",
    "删除",
    "新增",
    "更新",
    "摘要",
    "列表",
    "原子",
    "拆分",
    "规则",
    "上下文",
)

REQUIREMENT_PATTERNS = (
    re.compile(r"^(?:先|再).+"),
    re.compile(r".+(?:用于|是为了|用来).+"),
    re.compile(r".+(?:改为|保留|支持|统一|固定).+"),
    re.compile(r"^从.+(?:分析|提炼|抽取).+规则$"),
    re.compile(r"^按.+(?:拆分|分解).+"),
    re.compile(r"^(?:形成|生成).+规则$"),
    re.compile(r"^(?:便于|方便).+(?:验证|校验)$"),
)

DROP_PATTERNS = (
    re.compile(r"^两者不同$"),
)

RULE_SPLIT_PATTERN = re.compile(
    r"[，,]\s*(?=(?:不要|别|不能|只|仅|先|再|必须|优先|默认|保持|命令|路径|中文|英文|输出|回复|验证|确认|使用|限制|限定|支持|保留|改为|规则|长期记忆|会话|项目|系统|从|按|形成|生成|方便|便于|[^，,]{0,8}(?:用于|是为了|用来|不能|必须|保留|改为|支持|默认|优先|固定|分析|提炼|抽取|拆分|分解)))"
)

# 这些前缀通常只是口水，不值得进规则正文。
PREFIX_NOISE = (
    "请你",
    "请",
    "你要",
    "你得",
    "你需要",
    "记住",
    "注意",
    "最好",
    "希望你",
    "我希望",
    "要求是",
    "要求",
)

# 规则正文过长时会显著降低可验证性，这里先定一个保守阈值。
MAX_RULE_LEN = 36
DEFAULT_MAX_RULES = 8


def load_session_rules_module() -> Any:
    """动态加载共享的 session_rules 工具，复用项目根和会话解析逻辑。"""

    # 插件化后，CRUD 规则共享同一个插件级脚本。
    # 这里不再依赖 `rule-add` skill 目录，避免插件内部出现隐式旧 owner。
    module_path = Path(__file__).resolve().parents[3] / "scripts" / "session_rules.py"
    spec = importlib.util.spec_from_file_location("session_rules_shared", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load session rules module: {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_context_text(args: argparse.Namespace) -> str:
    """读取上下文来源：优先 `--context`，其次文件，最后 stdin。"""

    if args.context.strip():
        return args.context
    if args.context_file:
        return args.context_file.read_text(encoding="utf-8")
    if not sys.stdin.isatty():
        return sys.stdin.read()
    return ""


def normalize_text(text: str) -> str:
    """归一化空白和常见引号，避免后续匹配被脏字符搞偏。"""

    collapsed = text.replace("\r\n", "\n").replace("\r", "\n")
    collapsed = collapsed.replace("“", "\"").replace("”", "\"").replace("’", "'").replace("‘", "'")
    collapsed = re.sub(r"[ \t]+", " ", collapsed)
    return collapsed.strip()


def detect_speaker(line: str) -> tuple[str, str]:
    """尽量识别 user / assistant，说不清就标 unknown。"""

    normalized = line.strip()
    patterns = (
        ("user", r"^(user|用户|需求|human)\s*[:：]\s*(.+)$"),
        ("assistant", r"^(assistant|助手|codex|回复)\s*[:：]\s*(.+)$"),
        ("system", r"^(system|系统)\s*[:：]\s*(.+)$"),
    )
    for speaker, pattern in patterns:
        match = re.match(pattern, normalized, re.IGNORECASE)
        if match:
            return speaker, match.group(2).strip()
    return "unknown", normalized


def split_lines_to_segments(text: str) -> list[dict[str, Any]]:
    """按行和句号初步切开上下文，保留说话方和原始顺序。"""

    segments: list[dict[str, Any]] = []
    order = 0
    # 先把项目符号统一成独立行，利于抓到显式规则句。
    prepared = re.sub(r"\n\s*[-•]\s*", "\n- ", text)
    for raw_line in prepared.split("\n"):
        line = raw_line.strip()
        if not line:
            continue
        speaker, content = detect_speaker(line)
        for sentence in re.split(r"[。！？]\s*", content):
            sentence = sentence.strip(" -\t")
            if not sentence:
                continue
            segments.append(
                {
                    "speaker": speaker,
                    "text": sentence,
                    "order": order,
                }
            )
            order += 1
    return segments


def split_atomic_fragments(text: str) -> list[str]:
    """
    尝试把一句大话拆成原子约束。

    这里只做保守拆分：
    - `；` / `;` 一定拆
    - `，` / `,` 只在后半句明显像一个新约束时才拆
    """

    coarse_parts = [part.strip() for part in re.split(r"[；;]\s*", text) if part.strip()]
    fragments: list[str] = []
    for part in coarse_parts:
        # 逗号只在后半句明显像新规则时才拆，避免把正常解释句切成碎渣。
        split_parts = RULE_SPLIT_PATTERN.split(part)
        fragments.extend(fragment.strip() for fragment in split_parts if fragment.strip())
    return fragments


def strip_prefix_noise(text: str) -> str:
    """去掉口语前缀，让规则正文更短、更硬。"""

    current = text.strip()
    changed = True
    while changed:
        changed = False
        for prefix in PREFIX_NOISE:
            if current.startswith(prefix):
                current = current[len(prefix) :].strip(" ，,:：")
                changed = True
    return current


def compact_rule_content(text: str) -> str:
    """压缩常见废话，让规则更接近可执行句。"""

    current = strip_prefix_noise(text)
    replacements = {
        "暂时先": "先",
        "尽量保持": "保持",
        "必须要": "必须",
        "需要先": "先",
        "不要再": "不要",
        "只需要": "只",
        "请务必": "必须",
        "直接使用": "使用",
        "是为了": "用于",
        "用来": "用于",
        "目的是": "用于",
        "主要是为了": "用于",
        "就是为了": "用于",
        "这个规则收集系统": "规则收集",
        "最近的一个": "最近一个",
        "从最近的一个": "从最近一个",
        "按最小单元的方式": "按最小单元",
        "分析出": "分析",
        "方便及时验证": "便于及时验证",
        "，两者不同": "",
    }
    for source, target in replacements.items():
        current = current.replace(source, target)
    current = re.sub(r"[\"']+", "", current)
    current = re.sub(r"\s+", " ", current).strip(" ，,:：;；。")
    return current


def rule_signal_score(text: str, speaker: str) -> int:
    """给候选片段打分，尽量优先保留真约束而不是背景解释。"""

    score = 0
    if speaker == "user":
        score += 3
    elif speaker == "assistant":
        score += 2
    else:
        score += 1

    score += sum(2 for cue in STRONG_RULE_CUES if cue in text)
    score += sum(1 for cue in MEDIUM_RULE_CUES if cue in text)

    if 4 <= len(text) <= 48:
        score += 1
    if any(token in text for token in ("不要", "只", "先", "必须", "优先", "默认")):
        score += 2
    if any(pattern.search(text) for pattern in REQUIREMENT_PATTERNS):
        score += 2
    return score


def infer_title(content: str) -> str:
    """把正文映射到固定标题桶，避免标题五花八门。"""

    # `先/再` 这类顺序约束通常需要单独成条，避免被 `只/仅` 抢先误分到范围类。
    if content.startswith(("先", "再")) or "结论优先" in content or "顺序" in content:
        return "输出顺序"
    if any(token in content for token in ("不要", "别", "不能", "禁止")):
        return "禁止项"
    if any(token in content for token in ("最小单元", "原子", "拆分", "分解", "便于验证")):
        return "验收要求"
    if any(token in content for token in ("最近", "当前", "会话", "本轮")) and any(
        token in content for token in ("从", "只", "仅", "限定")
    ):
        return "范围约束"
    if any(token in content for token in ("只", "仅", "范围", "限定", "聚焦")):
        return "范围约束"
    if any(token in content for token in ("输出", "格式", "反引号", "中文", "英文", "空格", "命令", "路径")):
        return "输出格式"
    if any(token in content for token in ("验证", "验收", "确认", "测试", "检查")):
        return "验收要求"
    if any(token in content for token in ("优先", "默认")):
        return "执行优先级"
    return "需求约束"


def build_reason(original: str, speaker: str, score: int) -> str:
    """给人看的简短依据说明，别写成长篇检讨。"""

    speaker_text = {
        "user": "用户近轮上下文",
        "assistant": "assistant 近轮上下文",
        "unknown": "近轮上下文",
        "system": "system 近轮上下文",
    }.get(speaker, "近轮上下文")
    excerpt = original.strip()
    if len(excerpt) > 28:
        excerpt = f"{excerpt[:28]}..."
    return f"{speaker_text}命中规则信号（score={score}）：{excerpt}"


def is_rule_like(content: str) -> bool:
    """最后一道闸，过滤纯背景句。"""

    if len(content) < 4:
        return False
    if any(pattern.search(content) for pattern in DROP_PATTERNS):
        return False
    if any(cue in content for cue in STRONG_RULE_CUES + MEDIUM_RULE_CUES):
        return True
    return any(pattern.search(content) for pattern in REQUIREMENT_PATTERNS)


def dedupe_candidates(candidates: list[dict[str, Any]], max_rules: int) -> list[dict[str, Any]]:
    """按正文去重，保留分数更高、出现更早的版本。"""

    best_by_content: dict[str, dict[str, Any]] = {}
    for candidate in candidates:
        key = candidate["content"]
        existing = best_by_content.get(key)
        if existing is None or candidate["score"] > existing["score"]:
            best_by_content[key] = candidate

    ranked = sorted(best_by_content.values(), key=lambda item: (-item["score"], item["order"]))
    trimmed = ranked[:max_rules]
    # 输出时重新编号，便于人眼确认。
    for index, item in enumerate(trimmed, start=1):
        item["index"] = index
    return trimmed


def extract_candidates(context_text: str, max_rules: int) -> list[dict[str, Any]]:
    """核心提炼流程：切句、打分、拆分、归一化、去重。"""

    normalized = normalize_text(context_text)
    segments = split_lines_to_segments(normalized)
    # 规则候选默认只从用户近轮表达里提炼，避免把 assistant 的自我承诺误收成硬规则。
    # 若上下文根本没打 `user:` 标签，再退回 `unknown`，兼容裸文本转录。
    eligible_speakers = {"user"} if any(segment["speaker"] == "user" for segment in segments) else {"unknown"}
    raw_candidates: list[dict[str, Any]] = []
    for segment in segments:
        if segment["speaker"] not in eligible_speakers:
            continue
        base_score = rule_signal_score(segment["text"], segment["speaker"])
        if base_score < 4:
            continue

        for fragment in split_atomic_fragments(segment["text"]):
            content = compact_rule_content(fragment)
            if not is_rule_like(content):
                continue

            # 长句优先再压一次，实在压不下来也给低一点分，不直接瞎截断。
            penalty = 1 if len(content) > MAX_RULE_LEN else 0
            candidate_score = max(1, base_score - penalty)
            raw_candidates.append(
                {
                    "index": 0,
                    "title": infer_title(content),
                    "content": content,
                    "evidence": fragment.strip(),
                    "reason": build_reason(fragment, segment["speaker"], candidate_score),
                    "source": segment["speaker"],
                    "score": candidate_score,
                    "order": segment["order"],
                }
            )
    return dedupe_candidates(raw_candidates, max_rules=max_rules)


def build_payload(args: argparse.Namespace, candidates: list[dict[str, Any]], context_text: str) -> dict[str, Any]:
    """构造统一 JSON 回执，给技能层做选择交互和批量写入。"""

    session_rules = load_session_rules_module()
    project_root = Path(args.project_root).resolve() if args.project_root else session_rules.detect_project_root(Path.cwd())
    session_id = session_rules.resolve_session_id(args.session_id)

    # rule-add v0.3 仍保留批量新增，但批量协议只认英文分号。
    # 中文分号属于自然语言标点，不能再作为结构化分隔符输出。
    batch_title = ";".join(candidate["title"] for candidate in candidates)
    batch_content = ";".join(candidate["content"] for candidate in candidates)
    excerpt = normalize_text(context_text)
    if len(excerpt) > 240:
        excerpt = f"{excerpt[:240]}..."

    return {
        "action": "extract",
        "project_root": str(project_root),
        "session_id": session_id,
        "candidate_count": len(candidates),
        "candidates": candidates,
        "batch_title": batch_title,
        "batch_content": batch_content,
        "context_excerpt": excerpt,
    }


def print_human(payload: dict[str, Any]) -> None:
    """人类可读输出，给技能层和人工排查都留点面子。"""

    count = int(payload["candidate_count"])
    print(f"候选规则共 {count} 条。")
    if count == 0:
        print("- 当前最近上下文里没有足够清晰的规则候选。")
        return

    for candidate in payload["candidates"]:
        print(f"{candidate['index']}. [{candidate['title']}] {candidate['content']}")
        print(f"   依据：{candidate['reason']}")

    print(f"batch_title: {payload['batch_title']}")
    print(f"batch_content: {payload['batch_content']}")


def cmd_extract(args: argparse.Namespace) -> int:
    """从上下文里提炼规则候选。"""

    context_text = read_context_text(args)
    if not normalize_text(context_text):
        print("context is required", file=sys.stderr)
        return 1

    candidates = extract_candidates(context_text=context_text, max_rules=args.max_rules)
    payload = build_payload(args=args, candidates=candidates, context_text=context_text)
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0

    print_human(payload)
    return 0


def build_parser() -> argparse.ArgumentParser:
    """构造 CLI。"""

    parser = argparse.ArgumentParser(description="Extract atomic rule candidates from recent dialogue context.")
    subparsers = parser.add_subparsers(dest="command", required=True)

    extract_parser = subparsers.add_parser("extract", help="Extract atomic rule candidates from context text")
    extract_parser.add_argument("--context", default="", help="Inline context text")
    extract_parser.add_argument("--context-file", type=Path, default=None, help="Path to a UTF-8 context text file")
    extract_parser.add_argument("--project-root", default="", help="Optional explicit project root")
    extract_parser.add_argument("--session-id", default="", help="Optional explicit session id")
    extract_parser.add_argument("--max-rules", type=int, default=DEFAULT_MAX_RULES, help="Maximum candidate rules to return")
    extract_parser.add_argument("--json", action="store_true", help="Output JSON instead of human-readable text")
    extract_parser.set_defaults(func=cmd_extract)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
