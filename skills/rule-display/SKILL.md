---
name: rule-display
description: Display rules selected by the current session.
---

# rule-display

## Overview

`$rule-display` 负责当前会话选用规则的结尾展示策略。

它不修改规则，只决定本轮回复结尾展示完整列表、摘要，还是不展示。事实来源是 `<project_root>/.codex-rules/rules.db` 中当前 `session_id` 的选用关系。

## Display Rules

1. 当 `$rule-check` 更新选用关系、`$rule-update` 更新已选规则、或 `$rule-delete` 取消选用后，本轮回复结尾输出完整“收集的规则列表”。
2. 完整列表只展示当前会话已选 active 规则的 `content`，格式固定为：

```text
收集的规则列表
- 规则内容
```

3. 非更新轮若当前会话存在选用规则，结尾只输出摘要：

```text
当前会话选用规则 N 条：标题 1、标题 2
```

4. 当前会话没有选用规则时，不输出规则小节。
5. 用户显式调用 `$rule-list` 时，按 `$rule-list` 的完整列表或摘要模式输出。

## Source Of Truth

- 唯一事实来源：`<project_root>/.codex-rules/rules.db`。
- 只展示当前 `session_id` 已选用的 active 项目规则。
- 未选用的项目规则库不参与普通结尾展示。
- 不展示长期记忆条目。
- 不展示 `project-memory` 条目。

## Recommended Command

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-display"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe list
& $exe list --summary
```

## Guardrails

- 不读取旧 `.codex/session-rules` 或 `.codex/project-rules`。
- 不把未选用项目规则、长期记忆、当前会话选用规则混在同一列表里。
- 如果规则刚变更，优先完整列表；普通回复优先摘要；没有选用规则就别硬凑小节。
