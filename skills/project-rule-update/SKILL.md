---
name: project-rule-update
description: Use when the user wants to modify an existing project-shared rule by id, including title, content, tags, or active/deprecated status.
---

# Project Rule Update

## Overview

`$project-rule-update` 按 `id` 更新当前项目规则库中的共享规则。

已被某个会话 pick 过的规则不会被追溯修改；pick 进入会话的是当时的规则快照。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\project-rule-update"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script update --id pr-12345678 --content "先改共享 contract，再改调用方"
python $script update --id pr-12345678 --tags "architecture,contract"
python $script update --id pr-12345678 --status deprecated --json
```

## Guardrails

- 必须按 `id` 更新，不做标题模糊匹配。
- 不追溯修改已经进入会话规则的快照。
- 不写长期记忆或 `project-memory`。



