---
name: project-rule-pick
description: Use when the user wants to select one or more active project-shared rules by id, tag, or keyword and copy them into the current conversation's session-rule context.
---

# Project Rule Pick

## Overview

`$project-rule-pick` 把项目规则库里的 `active` 规则复制到当前会话规则中。

pick 是快照复制，不是动态引用。项目规则后续变化不会悄悄改变当前会话背景。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\project-rule-pick"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script pick --ids pr-12345678,pr-abcdef12
python $script pick --tag output
python $script pick --query "响应式" --json
```

## Behavior

- 只 pick `active` 项目规则。
- 默认写入当前会话规则标题为 `项目规则`，正文保持项目规则 `content`。
- 同一会话中已有相同 `content` 时不重复插入。
- pick 成功后，当前会话规则展示策略仍由 `$rule-display` 负责。

## Guardrails

- 不自动 pick 所有项目规则。
- 不 pick `deprecated` 规则。
- 不把项目规则写进长期记忆或 `project-memory`。



