---
name: rule-update
description: Update selected project rules.
---

# rule-update

## Overview

`$rule-update` 用于按 `id` 更新当前会话已选用的项目级规则本体。

v0.4 起不存在会话级规则副本。更新会直接修改 `.codex-rules/rules.db` 里的项目规则；其他已经选用同一规则的会话之后会看到最新内容。

## Workflow

1. 解析当前项目根和当前 `session_id`。
2. 确认目标 `id` 已被当前会话选用；未选用则失败。
3. 至少更新 `title` 或 `content` 其中一项。
4. 写回项目级规则本体，并刷新当前会话选用规则列表。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-update"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe update --id rule-12345678 --content "只查原因，暂时不要改代码"
& $exe update --id rule-12345678 --title "范围限制" --content "只看 AI 批改页面"
& $exe update --id rule-12345678 --content "先给结论"
```

## Output Contract

- 默认输出当前会话选用规则完整列表。
- JSON 输出包含 `changed_rule`、`rules`、`display_rules`、`db_file`。

## Guardrails

- 必须按 `id` 更新，不能靠标题或全文模糊匹配乱改。
- 目标规则必须已被当前会话选用。
- 不创建会话级规则副本。
- 更新不会自动写长期记忆。
