---
name: rule-check
description: Check, edit, and select project rules.
---

# rule-check

## Overview

`$rule-check` 是项目级规则库的检查、编辑、废弃、删除和当前会话选用入口。

规则内容只保存在 `<project_root>/.codex-rules/rules.db` 的项目级规则表中；当前会话只保存 `session_id -> rule_id` 的选用关系，不复制规则正文。

v0.4 起规则拥有单值强分类 `module`。`rule-check` 支持按模块筛选；选择业务模块时默认同时显示 `global` 通用规则。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-check"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

& $exe project-list
& $exe project-list --tag output
& $exe project-list --query "响应式"
& $exe project-list --module frontend
& $exe project-list --all

& $exe check
& $exe check --query "输出格式"
& $exe check --module frontend

& $exe project-update --id rule-12345678 --content "先改共享 contract，再改调用方"
& $exe project-update --id rule-12345678 --tags "architecture,contract"
& $exe project-update --id rule-12345678 --module backend
& $exe project-update --id rule-12345678 --status deprecated

& $exe project-delete --id rule-12345678
& $exe project-delete --id rule-12345678 --hard

& $exe pick --ids rule-12345678,rule-abcdef12
& $exe pick --tag output
& $exe pick --query "响应式"
```

## Behavior

- 默认需要人工检查、编辑或选择规则时，优先使用 `pick --ui` 打开 Windows 原生 checklist 管理窗口。
- `list` 默认只展示 `active` 规则；`--all` 包含 `deprecated` 规则。
- `--module global` 只显示 `global`；`--module frontend` 显示 `frontend + global`。
- UI 左上模块筛选只改变可见行，不清空已勾选规则。
- UI 表格里的 `状态` 和 `模块` 单元格是状态/模块的唯一编辑入口；点击后下拉切换，选择后立即更新当前行内存态，最终仍以“保存编辑”或“保存并选取”提交。
- `update` 按 `id` 更新项目规则本体；已选用该规则的会话之后会读取最新内容。
- `delete` 默认把项目规则标记为 `deprecated`；只有用户明确要求彻底删除时才使用 `--hard`。
- `pick` 不复制规则正文，只更新当前 `session_id` 的选用关系。
- `pick --ui` 启动时必须按当前 `session_id` 恢复已勾选规则。
- `pick --ui` 的复选框表示当前会话是否采用该规则；单击或高亮某行只表示右侧正在编辑该规则。
- 搜索过滤只改变可见行，不清空已勾选规则。
- 未勾选任何规则时点击确认，表示当前会话不采用任何可见范围内确认后的规则；项目规则本体仍可保存编辑。
- `deprecated` 规则可显示和编辑，但不会被选用到当前会话。
- 用户取消窗口时不写规则本体，也不改当前会话选用关系。

## Guardrails

- 不创建或读取 `.codex/session-rules`、`.codex/project-rules` 或 `rules.yaml`。
- `check` 的 UI 和 SQLite 写入都由 `bin/rule-system.exe` 统一执行，不再绕 Python。
- 不自动选择所有项目规则。
- 不写长期记忆或 `project-memory`。
