---
name: project-rule-list
description: Use when the user wants to inspect, search, or filter the current project's shared rule library before deciding which rules to pick into a session.
---

# Project Rule List

## Overview

`$project-rule-list` 查看当前项目共享规则库。

默认只展示 `active` 规则；带 `--all` 时包含 `deprecated` 规则。展示项目规则时应保留 `id`，因为后续 update/delete/pick 都依赖它。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\project-rule-list"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script list
python $script list --tag output
python $script list --query "响应式"
python $script list --all --json
```

## Guardrails

- 该 skill 只读，不修改项目规则。
- 不展示当前会话规则，当前会话规则由 `$rule-list` 管。
- 不展示长期记忆或 `project-memory`。



