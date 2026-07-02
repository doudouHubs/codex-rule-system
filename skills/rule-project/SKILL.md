---
name: rule-project
description: Manage project-shared rules.
---

# rule-project

## Overview

`$rule-project` 是项目共享规则库的统一操作入口。

项目规则库位于 `<project_root>/.codex/project-rules/`。它跨会话共享，但不会自动进入当前会话；需要 `pick` 后才复制为当前会话规则快照。

新增项目规则不归这里维护。新增统一使用 `$rule-add --scope project`，避免“新增规则”出现第二个 owner。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-project"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script list
python $script list --tag output
python $script list --query "响应式"
python $script list --all --json

python $script update --id pr-12345678 --content "先改共享 contract，再改调用方"
python $script update --id pr-12345678 --tags "architecture,contract"
python $script update --id pr-12345678 --status deprecated --json

python $script delete --id pr-12345678
python $script delete --id pr-12345678 --hard --json

python $script pick --ids pr-12345678,pr-abcdef12
python $script pick --tag output
python $script pick --query "响应式" --json
python $script pick --ui --json
python $script pick --ui --query "输出格式" --json
```

## Behavior

- `list` 默认只展示 `active` 规则；`--all` 包含 `deprecated` 规则。
- `update` 必须按 `id` 更新，不做标题模糊匹配。
- `delete` 默认软删除为 `deprecated`；只有用户明确要求彻底删除时才使用 `--hard`。
- `pick` 只复制 `active` 规则进入当前会话，同一会话已有相同 `content` 时不重复插入。
- `pick --ui` 在 Windows 上打开原生选择器，支持模糊查询和多选；用户取消窗口时不写入当前会话。

## Guardrails

- 不在 `$rule-project` 里新增项目规则；新增走 `$rule-add --scope project`。
- 不展示或修改当前会话规则；当前会话规则由 `$rule-list/update/delete` 管。
- 不追溯修改已经 pick 进会话的规则快照。
- 不自动 pick 所有项目规则。
- 不写长期记忆或 `project-memory`。
