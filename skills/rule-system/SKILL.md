---
name: rule-system
description: Use when the user asks about the Rule System plugin, session-rule boundaries, project-shared rule libraries, how rule-add/rule-project skills relate, or wants global prompt rule-system guidance moved into plugin-managed instructions.
---

# Rule System

## Overview

`$rule-system` 是 `rule-system` 插件的总控说明层。

它不直接写规则；它负责解释规则系统边界、路由关系和全局提示词瘦身原则。具体读写动作统一交给 `$rule-*`。

## Boundaries

- 会话规则只作用于当前项目当前会话，事实来源是 `<project_root>/.codex/session-rules/<session_id>/rules.yaml`。
- 会话规则用于当前任务和当前会话内的需求更迭，不等同于长期记忆。
- 项目规则库作用于当前项目内所有会话，事实来源是 `<project_root>/.codex/project-rules/rules.yaml`。
- 项目规则库不会自动注入会话；必须通过 `$rule-project` 的 `pick` 操作显式拾取后，才进入当前会话规则。
- `$rule-project` 的 `pick` 是快照复制，不是动态引用；项目规则后续修改不会悄悄改变已拾取的会话规则。
- 长期记忆只记录稳定沟通偏好、身份偏好和跨项目复用的工作习惯；未确认前不要写入长期记忆。
- `project-memory` 是项目工作流记忆层，不是当前会话规则仓库。
- 规则系统不得把会话规则或项目规则自动提升为长期记忆或 `project-memory`。

## Routing

- 用户要求新增规则、记录当前约束、把某条要求收集成规则：使用 `$rule-add`。
- 用户要求新增项目级共享规则：使用 `$rule-add --scope project`。
- 用户要求修改规则：使用 `$rule-update`。
- 用户要求删除规则或清空当前会话规则：使用 `$rule-delete`。
- 用户要求查看当前规则：使用 `$rule-list`。
- 用户要求从最近上下文分析可沉淀规则候选：使用 `$rule-capture`。
- 需要决定本轮结尾是否展示完整规则或摘要：使用 `$rule-display`。
- 用户要求查看、搜索、修改、废弃、删除项目规则库，或把项目共享规则带入当前会话背景：使用 `$rule-project`。

## Global Prompt Migration Policy

全局提示词只应该保留极薄路由，不应该承载规则系统细则。

推荐全局只保留：

```text
规则收集、会话规则 CRUD、项目规则库、规则拾取、规则展示和上下文规则捕获由 rule-system 插件接管；遇到相关意图时使用对应 $rule-* skill，项目新增使用 $rule-add --scope project。
```

下面这些内容应放在插件内，而不是堆进全局提示词：

- `session-rules` 存储路径和字段细节
- `project-rules` 存储路径和字段细节
- rule CRUD 的操作流程和输出格式
- rule-project 聚合入口的 list/update/delete/pick 操作流程和输出格式
- `rule-capture` 的候选提炼规则
- 规则与长期记忆、`project-memory` 的边界
- 规则变更后完整展示、非变更轮摘要展示、无规则不展示等策略

## Guardrails

- 不要在 `$rule-system` 里直接增删改查规则；调用对应动作 skill。
- 不要把插件策略复制回全局提示词，除非只是保留一条路由句。
- 不要让全局提示词和插件同时维护同一条细则，重复 owner 会导致漂移。



