---
name: rule-module
description: Manage rule module enum entries after planned module governance.
---

# rule-module

## Overview

`$rule-module` 管理规则模块枚举表 `rule_modules`。模块是强分类 owner，不是随手贴的弱标签；每条规则只能属于一个模块，默认 `global`。

模块变更必须先进入 Plan Mode 或完成等价的明确计划：说明要新增/修改/删除的 slug、业务边界、受影响规则、回滚方式，再执行命令。没有计划时，不要直接改模块表，这玩意儿乱加就会变成“分类坟场”，老遭罪了。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-module"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe module-list
& $exe module-list --all

& $exe module-add --slug frontend --display-name "前端"
& $exe module-update --slug frontend --display-name "Web 前端"
& $exe module-update --slug frontend --status deprecated
& $exe module-delete --slug frontend
```

## Behavior

- `global` 自动初始化，表示跨模块通用规则。
- `global` 禁止删除，禁止废弃。
- `slug` 只允许小写字母、数字、单短横线，建议 1-64 字符。
- `display-name` 面向 UI 展示，可使用中文。
- 删除模块前必须没有任何规则引用，否则 fail-fast。
- 废弃模块后不能再用于新增或编辑规则。

## Planning Gate

修改模块表前必须先完成计划，至少覆盖：

- 模块边界：这个模块负责什么，不负责什么。
- 命名理由：为什么 slug 是稳定业务枚举，而不是临时标签。
- 影响范围：哪些规则会归入或移出该模块。
- 删除/废弃策略：是否有引用规则，如何处理。

只查看模块列表不需要 Plan Mode；新增、更新、删除模块必须走计划。

## Guardrails

- 不要把 tags 需求升级成 module；多值、临时、弱分类继续用 tags。
- 不要创建同义模块，例如 `frontend`、`web`、`ui` 三个互相打架。
- 不要绕过 `$rule-module` 直接改 SQLite。
- 所有 rule-system skill 名称继续保持 `rule-*`。
