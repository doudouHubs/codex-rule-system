---
name: rule-delete
description: Delete current-session rules.
---

# rule-delete

## Overview

`$rule-delete` 用于删除当前项目当前会话里的规则。

删除采用物理删除，不做软删除状态机，先把系统收得简单点，别养出一堆半死不活的垃圾规则。

## Workflow

1. 定位当前项目当前会话的 `rules.yaml`。
2. 若传入 `--id`，按 `id` 查找并删除目标规则；若未传入 `--id`，直接清空当前会话全部规则。
3. 删除成功后写回 YAML，并刷新 `meta.yaml`。
4. 回复结尾强制输出删除后的完整“收集的规则列表”。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-delete"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

# 删除一条规则
python $script delete --id r-12345678

# 不传参数：清空当前会话全部规则
python $script delete

# JSON 回执
python $script delete --id r-12345678 --json
```

## Output Contract

- 默认输出：
  - 一行总览：`当前项目当前会话规则共 N 条。`
  - 完整规则列表：每条仅显示 `content`，格式为 `- 规则内容`
- 技能内要求：
  - 删除成功后，正文说明删除了哪条规则，或说明已清空当前会话全部规则
  - 回复结尾强制输出完整“收集的规则列表”

## Guardrails

- 传 `--id` 时必须按 `id` 删除，不准糊里糊涂按标题误删。
- 未传 `--id` 时，默认清空当前会话全部规则。
- 传 `--id` 但找不到规则时必须明确失败。
- 删除只影响当前项目当前会话，不影响长期记忆与其他会话目录。



