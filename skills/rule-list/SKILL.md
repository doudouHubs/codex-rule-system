---
name: rule-list
description: List rules selected by the current session.
---

# rule-list

## Overview

`$rule-list` 用于查看当前项目当前会话已经选用的项目级规则。

事实来源是 `<project_root>/.codex-rules/rules.db` 中的 `rule_selections JOIN rules JOIN rule_details`，不是会话级规则文件。

## Workflow

1. 解析当前项目根和当前 `session_id`。
2. 读取当前会话选用的 active 项目规则。
3. 完整列表默认只展示规则 `content`，一行一条，不展示时间戳。
4. `--summary` 只返回数量和标题，适合非更新轮摘要。
5. 该技能只读，不修改规则本体或选用关系。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-list"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe list
& $exe list --summary
& $exe list --all
```

## Output Contract

- 完整列表模式：
  - 一行总览：`当前会话选用规则共 N 条。`
  - 规则明细：每条仅显示 `content`，格式为 `- 规则内容`
- 摘要模式：
  - `rule_count`
  - `rule_titles`
  - `summary`

## Guardrails

- 不读取旧 `.codex/session-rules` 或 `.codex/project-rules`。
- 不展示未被当前 session 选用的项目规则。
- 不展示长期记忆或 `project-memory`。
