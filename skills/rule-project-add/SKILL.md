---
name: rule-project-add
description: Legacy compatibility guidance for adding a stable project-shared rule; forward to `$rule-add --scope project`.
---

# Rule Project Add

## Overview

`$rule-project-add` 是旧项目新增入口的兼容说明，不再是新增项目规则的 canonical owner。

新增项目规则必须转发到 `$rule-add --scope project`。

项目规则库位于 `<project_root>/.codex/project-rules/`，不等同于当前会话规则，也不写长期记忆或 `project-memory`。

## Workflow

1. 判断用户给的是项目级稳定规则，而不是当前会话临时要求。
2. 整理成单义、可执行、可验证的规则文本。
3. 调用 `$rule-add --scope project` 写入项目规则库，状态默认为 `active`。
4. 若希望当前会话立即使用该规则，还需要调用 `$rule-project-pick` 拾取进当前会话。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-add"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

python $script add --scope project --title "输出格式" --content "命令和路径使用反引号包裹" --tags "output,format"
python $script add --scope project --title "执行边界" --content "修改共享层前先确认调用方影响" --json
```

## Guardrails

- 不要把一次性任务细节写成项目规则。
- 不要写长期记忆。
- 不要自动注入所有会话；项目规则必须显式 pick 才进入会话背景。
- 不要在该 skill 内维护第二套新增逻辑；它只负责兼容旧路由并转发到 `$rule-add --scope project`。



