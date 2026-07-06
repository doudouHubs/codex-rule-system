---
name: rule-delete
description: Unselect rules from the current session.
---

# rule-delete

## Overview

`$rule-delete` 用于取消当前会话采用的项目规则。

它不删除项目级规则本体，只删除当前 `session_id -> rule_id` 的选用关系。项目规则废弃或硬删除统一走 `$rule-check`。

## Workflow

1. 解析当前项目根和当前 `session_id`。
2. 若传入 `--id`，只取消当前会话对该规则的选用。
3. 若未传入 `--id`，清空当前会话全部选用关系。
4. 输出删除后的当前会话选用规则列表。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-delete"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe delete --id rule-12345678
& $exe delete
& $exe delete --id rule-12345678
```

## Output Contract

- 默认输出当前会话仍选用的规则完整列表。
- JSON 输出包含 `changed_rules`、`rules`、`session_id`、`db_file`。

## Guardrails

- 不删除项目规则本体。
- 传 `--id` 时目标规则必须已被当前会话选用。
- 未传 `--id` 时只清空当前会话选用关系，不影响其他会话。
- 不读取或写入旧 YAML。
