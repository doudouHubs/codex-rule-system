---
name: rule-update
description: Update current-session rules.
---

# rule-update

## Overview

`$rule-update` 用于按 `id` 更新当前项目当前会话里已经存在的规则。

底层复用插件级 `scripts/session_rules.py`，只改目标规则，不碰长期记忆。

## Workflow

1. 定位当前项目当前会话的 `rules.yaml`。
2. 按 `id` 查找规则；找不到就明确报错，不准装死。
3. 至少更新 `title` 或 `content` 其中一项，并刷新 `updated_at`。
4. 成功后在回复结尾输出完整的“收集的规则列表”。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-update"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

# 改正文
python $script update --id r-12345678 --content "只查原因，暂时不要改代码"

# 改标题和正文
python $script update --id r-12345678 --title "范围限制" --content "只看 AI 批改页面"

# JSON 回执
python $script update --id r-12345678 --content "先给结论" --json
```

## Output Contract

- 默认输出：
  - 一行总览：`当前项目当前会话规则共 N 条。`
  - 完整规则列表：每条仅显示 `content`，格式为 `- 规则内容`
- 技能内要求：
  - 更新成功后，正文说明更新内容
  - 回复结尾强制输出完整“收集的规则列表”

## Guardrails

- 必须按 `id` 更新，不能靠标题或全文模糊匹配乱改。
- 若目标规则不存在，必须明确报错。
- 更新不会自动升级长期记忆。



