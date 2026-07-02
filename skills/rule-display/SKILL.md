---
name: rule-display
description: Display current-session rules.
---

# rule-display

## Overview

`$rule-display` 负责会话规则的结尾展示策略。

它不修改规则，只决定“本轮回复结尾该不该展示规则、展示完整列表还是摘要”。结尾展示的事实来源只能是当前项目当前会话的 `rules.yaml`。

## Display Rules

1. 当 `$rule-add`、`$rule-update`、`$rule-delete` 或 `$rule-check` pick 成功执行后，本轮回复结尾必须输出完整“收集的规则列表”。
2. 完整列表只展示当前有效规则的 `content`，格式固定为：

```text
收集的规则列表
- 规则内容
```

3. 非更新轮若当前会话存在规则，结尾只输出摘要：

```text
当前会话规则 N 条：标题 1、标题 2
```

4. 当前会话没有规则时，不输出规则小节。
5. 用户显式调用 `$rule-list` 时，按 `$rule-list` 的完整列表或摘要模式输出。

## Source Of Truth

- 唯一事实来源：`<project_root>/.codex/session-rules/<session_id>/rules.yaml`。
- 未 pick 的项目规则库不参与普通结尾展示。
- 不展示长期记忆条目。
- 不展示 `project-memory` 条目。
- 不展示猜测、临时想法、失效结论或未写入规则。

## Separation From Memory

- 会话规则默认只作用于当前项目当前会话。
- 切换项目或切换会话后，不自动沿用旧会话规则。
- 只有用户明确要求写长期记忆，且长期记忆系统确认后，才进入长期记忆流程。

## Recommended Command

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-display"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

# 完整列表
python $script list

# 摘要
python $script list --summary
```

## Guardrails

- 不要把“规则展示”写进全局提示词大段细则；全局只保留路由。
- 不要把项目规则、长期记忆、会话规则混在同一列表里。
- 如果规则刚变更，优先完整列表；如果只是普通回复，优先摘要；如果没有规则，别硬凑小节。


