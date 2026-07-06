---
name: rule-capture
description: Capture project-level rule candidates from recent conversation.
---

# rule-capture

## Overview

`$rule-capture` 是项目级规则候选提炼器。

它负责从最近连续几轮对话里抽取明确或强语义隐含的需求规则，把规则拆成“单约束一条”的候选清单；用户确认后，复用 `$rule-add` 写入 `<project_root>/.codex-rules/rules.db`。

它不替代 `$rule-add`，只负责“找规则、拆规则、给候选、等确认”。

## Workflow

1. 从当前对话里抓取最近连续几轮上下文，默认优先取最近 `3-5` 轮，直到明显话题切换为止。
2. 把上下文整理成带说话方标记的文本片段，交给脚本做候选提炼。
3. 默认只从 `user` 近轮表达提炼规则候选；`assistant` 内容最多只作为理解旁证。
4. 展示候选清单，每条至少包含 `title`、`content`、`evidence`。
5. 用户确认后，把脚本产出的 `batch_title` 与 `batch_content` 交给 `$rule-add` 批量写入项目级规则库。
6. 若希望当前会话立即采用这些规则，还需要通过 `$rule-check` 勾选。

## Commands

```powershell
$skillRoot = "C:\path\to\rule-system\skills\rule-capture"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "skills/rule-capture/scripts/rule_capture.py"

$context = @"
user: 只查原因，不要改代码。
assistant: 我会先定位真实证据源，再只给原因。
user: 先给结论，再给证据。
"@
$context | python $script extract --json

python $script extract --context "user: 只查原因，不要改代码。user: 先给结论，再给证据。" --json
```

## Output Contract

- 脚本输出必须包含：
  - `candidate_count`
  - `candidates`
  - `batch_title`
  - `batch_content`
- `batch_title` 与 `batch_content` 使用英文分号 `;` 作为 `$rule-add` 批量协议分隔符。
- 每个 candidate 至少包含 `title`、`content`、`evidence`、`source`。

## Atomic Rule Policy

- 每条规则只允许表达一个约束。
- 默认标题范围固定为：`范围约束`、`输出顺序`、`禁止项`、`输出格式`、`验收要求`、`执行优先级`、`需求约束`。
- 一条里同时出现多个并列要求时必须继续拆分。
- 一条里既有约束又有背景解释时只保留可执行约束。

## Guardrails

- 默认分析范围只限最近连续几轮当前对话，不读取长期记忆，不跑历史会话检索。
- 不要因为 assistant 自己说过一句话，就把纯自嗨承诺写成规则。
- 不允许未经确认就直接写入项目规则库。
- 若最终没有足够清晰的候选规则，明确返回“无候选”，不要硬凑。
- 候选规则写入必须复用 `$rule-add`，不要另起存储目录或第二套规则格式。
