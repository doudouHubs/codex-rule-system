---
name: project-rule-add
description: Use when the user wants to add a stable project-shared rule that all conversations in the same project can later search and pick into their current-session rule context.
---

# Project Rule Add

## Overview

`$project-rule-add` 把稳定、可跨会话复用的项目规则写入当前项目规则库。

项目规则库位于 `<project_root>/.codex/project-rules/`，不等同于当前会话规则，也不写长期记忆或 `project-memory`。

## Workflow

1. 判断用户给的是项目级稳定规则，而不是当前会话临时要求。
2. 整理成单义、可执行、可验证的规则文本。
3. 写入项目规则库，状态默认为 `active`。
4. 若希望当前会话立即使用该规则，还需要调用 `$project-rule-pick` 拾取进当前会话。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\project-rule-add"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script add --title "输出格式" --content "命令和路径使用反引号包裹" --tags "output,format"
python $script add --title "执行边界" --content "修改共享层前先确认调用方影响" --json
```

## Guardrails

- 不要把一次性任务细节写成项目规则。
- 不要写长期记忆。
- 不要自动注入所有会话；项目规则必须显式 pick 才进入会话背景。



