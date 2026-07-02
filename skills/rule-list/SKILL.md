---
name: rule-list
description: List current-session rules.
---

# rule-list

## Overview

`$rule-list` 用于查看当前项目当前会话已经入库的规则。

显式调用时默认返回简版完整规则列表；带 `--summary` 时只返回数量和标题，适合非更新轮摘要。

## Workflow

1. 定位当前项目当前会话的规则目录。
2. 若目录不存在则自动初始化空仓。
3. 读取 `rules.yaml`，按当前内容返回完整列表或摘要。
4. 完整列表默认只展示规则 `content`，一行一条，不展示 `id`、标题、时间戳。
5. 该技能只读，不修改任何规则内容。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-list"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

# 完整列表
python $script list

# 摘要
python $script list --summary
 
# JSON
python $script list --json
```

## Output Contract

- 完整列表模式：
  - 一行总览：`当前项目当前会话规则共 N 条。`
  - 规则明细：每条仅显示 `content`，格式为 `- 规则内容`
- 摘要模式：
  - `rule_count`
  - `rule_titles`
  - `summary`

## Guardrails

- `rule-list` 不修改规则。
- 只读取当前项目当前会话，不串到长期记忆或别的会话目录。
- 用户显式查看时给完整列表；非更新轮摘要策略由 `$rule-display` 负责。



