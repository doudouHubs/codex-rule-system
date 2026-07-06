---
name: rule-system
description: Explain rule-system boundaries.
---

# rule-system

## Overview

`$rule-system` 是 `rule-system` 插件的总控说明层。

它不直接写规则；它负责解释规则系统边界、路由关系和全局提示词瘦身原则。具体读写动作统一交给 `$rule-*`。

## Boundaries

- v0.3 起规则内容只有项目级，事实来源是 `<project_root>/.codex-rules/rules.db`。
- 当前会话采用哪些规则由 `rule_selections` 通过 `session_id` 逻辑隔离保存。
- 会话不保存规则正文，不存在 `.codex/session-rules/<session_id>/rules.yaml`。
- 项目规则不再保存到 `.codex/project-rules/rules.yaml`。
- 旧 YAML 只允许通过 `$rule-scan` 显式导入 SQLite；日常主路径不得读取旧 YAML。
- `$rule-check` 的 `pick` 是更新选用关系，不是复制快照；项目规则后续修改会影响已选用该规则的会话展示。
- 长期记忆只记录稳定沟通偏好、身份偏好和跨项目复用的工作习惯；未确认前不要写入长期记忆。
- `project-memory` 是项目工作流记忆层，不是规则仓库。

## Routing

- 用户要求新增规则、记录当前约束、把某条要求收集成规则：使用 `$rule-add`。
- 用户要求查看、搜索、编辑、废弃、删除项目规则库，或选择当前会话采用哪些规则：使用 `$rule-check`；需要人工检查时默认打开 UI。
- 用户要求查看当前会话采用的规则：使用 `$rule-list`。
- 用户要求修改当前会话已采用规则：使用 `$rule-update`。
- 用户要求当前会话不再采用某条规则或清空采用关系：使用 `$rule-delete`。
- 用户要求从最近上下文分析可沉淀规则候选：使用 `$rule-capture`。
- 用户要求扫描、导入、迁移旧 `.codex` YAML 规则：使用 `$rule-scan`。
- 需要决定本轮结尾是否展示完整规则或摘要：使用 `$rule-display`。

## Global Prompt Migration Policy

全局提示词只应该保留极薄路由，不应该承载规则系统细则。

推荐全局只保留：

```text
规则收集、项目级规则库、当前会话规则选用、规则展示和上下文规则捕获由 rule-system 插件接管；遇到相关意图时使用对应 $rule-* skill。
```

下面这些内容应放在插件内，而不是堆进全局提示词：

- `.codex-rules/rules.db` 的 schema 与路径细节
- 项目规则 CRUD 与当前会话选用关系
- rule-check UI 检查/编辑/选择流程
- rule-capture 候选提炼规则
- rule-scan 旧 YAML 显式导入规则
- 规则与长期记忆、`project-memory` 的边界
- 规则变更后完整展示、非变更轮摘要展示、无规则不展示等策略

## Guardrails

- 不要在 `$rule-system` 里直接增删改查规则；调用对应动作 skill。
- 不要把插件策略复制回全局提示词，除非只是保留一条路由句。
- 不要让全局提示词和插件同时维护同一条细则，重复 owner 会导致漂移。
