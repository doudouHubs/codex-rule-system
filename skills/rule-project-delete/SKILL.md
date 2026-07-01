---
name: rule-project-delete
description: Use when the user wants to deprecate one project-shared rule by id, or explicitly hard-delete it from the project rule library.
---

# Rule Project Delete

## Overview

`$rule-project-delete` 删除项目共享规则。

默认是软删除：把 `status` 改成 `deprecated`。只有用户明确要求硬删除时才使用 `--hard` 物理删除。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-project-delete"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/project_rules.py"

python $script delete --id pr-12345678
python $script delete --id pr-12345678 --hard --json
```

## Guardrails

- 默认软删除，别上来物理删除项目共享资产。
- `--hard` 只在用户明确要求彻底删除时使用。
- 删除项目规则不会删除已经 pick 进会话的规则快照。



