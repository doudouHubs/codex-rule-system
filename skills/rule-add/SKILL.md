---
name: rule-add
description: Add project-level rules.
---

# rule-add

## Overview

`$rule-add` 是新增规则的唯一入口。v0.4 起本地落盘的规则内容只有项目级，写入 `<project_root>/.codex-rules/rules.db`。

它不写会话级规则副本，不写长期记忆，也不写 `project-memory`。当前会话是否采用某条规则由 `$rule-check` 维护选用关系。

## Workflow

1. 从当前 `cwd` 推导项目根；若用户显式给出项目根，以显式参数为准。
2. 先把用户表达整理成清晰、单义、可执行的 `title` 与 `content`，不要把口语原话一股脑塞进数据库。
3. 使用 `bin/rule-system.exe add` 写入项目级 SQLite 规则库。
4. 用 `--module <slug>` 指定强分类模块；不传则进入 `global`。不存在或已废弃模块必须失败。
5. `--scope project` 可传但不改变语义；`--scope session` 已退役，必须失败。
6. 批量新增只认英文分号 `;`，且 `title` 与 `content` 两侧数量必须一致；中文分号 `；` 永远是普通正文标点。
7. 新增成功只表示进入项目规则库；若希望当前会话立即采用，需要显式使用 `$rule-check` 勾选。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-add"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$exe = Join-Path $pluginRoot "bin/rule-system.exe"

# 默认：新增项目级规则
& $exe add --title "输出格式" --content "先给结论，再给证据" --tags "output,format"

# 显式项目级：和默认语义一致
& $exe add --scope project --title "GUI 验收" --content "真实 GUI 交互由用户手动确认" --tags "ui,verification"

# 指定模块：模块必须先由 rule-module 规划并创建
& $exe add --module frontend --title "响应式布局" --content "移动端优先，再适配桌面" --tags "ui,css"

# 批量：标题和内容都用英文分号 `;` 对齐拆分
& $exe add --title "范围约束;输出格式" --content "不要扩题;命令和路径使用反引号包裹"

# 退役路径：会失败
& $exe add --scope session --title "临时规则" --content "这不会再写会话规则"
```

## Output Contract

- 输出项目规则库摘要、规则 ID、状态、标签和内容。
- 新增成功后不要声称当前会话已经采用；当前会话采用必须经过 `$rule-check` 或 `rule-system.exe pick/check`。
- 若本轮原本在处理更大的任务，新增规则后应继续推进原任务，不要停在回执上。

## Guardrails

- 不创建 `.codex/session-rules` 或 `.codex/project-rules`。
- 不把新增规则写进长期记忆或 `project-memory`。
- `title` 和 `content` 必须非空。
- `scope=session` 必须失败，避免重新引入会话级规则 owner。
- 所有 skill 名称必须使用 `rule-*` 前缀。
