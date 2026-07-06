---
name: rule-scan
description: Import legacy YAML rules into the SQLite rule database.
---

# rule-scan

## Overview

`$rule-scan` 是 v0.4 的显式旧数据导入入口。它从旧 `.codex/project-rules` 和 `.codex/session-rules` YAML 文件扫描规则，录入 `<project_root>/.codex-rules/rules.db`。

它不是运行时兼容层；导入完成后，`$rule-add`、`$rule-check`、`$rule-list` 等主路径仍然只读 SQLite。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-scan"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

# 默认从 <project_root>/.codex 扫描旧项目规则和旧会话规则
& $exe scan --project-root "F:\path\to\project"

# 只导入旧项目规则，不恢复旧 session 的选用关系
& $exe scan --project-root "F:\path\to\project" --project-only

# 指定旧 .codex 数据源目录
& $exe scan --project-root "F:\path\to\project" --source "F:\path\to\project\.codex"

# 机器可读回执
& $exe scan --project-root "F:\path\to\project" --json
```

## Behavior

- 默认扫描 `<project_root>/.codex/project-rules/rules.yaml`。
- 默认扫描 `<project_root>/.codex/session-rules/<session_id>/rules.yaml`。
- 旧项目规则导入为项目级规则。
- 旧会话规则导入为项目级规则，并恢复该旧 `session_id` 对应的 `rule_selections` 选用关系。
- 导入时自动分析模块：优先使用 YAML 里的 `module` 字段；没有时按 `tags/title/content` 推断 `frontend/backend/docs/testing/workflow/output/global`。
- 推断出的非 `global` 模块会自动创建，便于后续用 `$rule-module` 和 `$rule-check` 整理。
- 相同 `title + content` 已存在时复用现有规则，不重复插入。
- 缺少 `title` 或 `content` 的脏数据会被跳过。
- `deprecated` 规则会导入为 `deprecated`，但不会被恢复选用到会话。

## Guardrails

- 只在用户明确要求扫描、导入、迁移旧 YAML 规则时使用。
- 不要在日常新增、查看、选择规则时读取旧 YAML。
- 导入后不要保留 `.codex/session-rules` 或 `.codex/project-rules` 作为事实来源。
- 自动模块归类是确定性启发式，不调用 LLM；导入后如需调整模块，用 `$rule-check` 编辑规则模块，或先走 Plan Mode 再用 `$rule-module` 治理模块枚举。
- 不写长期记忆或 `project-memory`。
