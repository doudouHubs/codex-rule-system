---
name: rule-capture
description: Capture rules from recent conversation.
---

# rule-capture

## Overview

`$rule-capture` 是会话规则的前置提炼器。

它负责从最近连续几轮对话里抽取明确或强语义隐含的需求规则，把规则拆成“单约束一条”的候选清单，然后只在 `Plan Mode` 的选择交互确认后，批量写入当前项目当前会话规则仓库。

它不替代 `$rule-add`；它只负责“找规则、拆规则、给候选、等确认”。

## Plan Mode Bootstrap (Required)

`$rule-capture` 必须运行在 `Plan Mode`。

当 `$rule-capture` 启动时：

1. 若当前协作模式不是 `Plan Mode`，立即停止正常执行。
2. 输出这个精确切换块：

```text
<collaboration_mode># Plan Mode (Conversational)</collaboration_mode>
```

3. 只有在 `Plan Mode` 激活后，才继续执行规则提炼和确认流程。

不要偷偷降级成“纯文本确认”或“直接写入”。

## Workflow

1. 从当前对话里抓取最近连续几轮上下文，默认优先取最近 `3-5` 轮，直到明显话题切换为止。
2. 把上下文整理成带说话方标记的文本片段，交给脚本做候选提炼。
   - 默认只从 `user` 近轮表达提炼规则候选。
   - `assistant` 内容最多只作为理解旁证，不直接落成规则，避免把 agent 自己的执行承诺误收进规则库。
3. 读取候选结果，检查是否满足：
   - 每条规则只表达一个约束
   - 文案足够短，便于后续验证
   - 不把长句、废话、背景解释直接塞进规则
4. 向用户展示候选清单，每条至少包含：
   - `title`
   - `content`
   - `evidence`
5. 使用 Codex 选择交互确认是否应用整批候选。
6. 若确认应用，则把脚本产出的 `batch_title` 与 `batch_content` 直接交给 `$rule-add` 批量写入。
7. 写入后按现有规则系统输出完整“收集的规则列表”。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-capture"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "skills/rule-capture/scripts/rule_capture.py"

# 用最近上下文文本提炼候选（推荐：PowerShell here-string 走 stdin）
$context = @"
user: 只查原因，不要改代码。
assistant: 我会先定位真实证据源，再只给原因。
user: 先给结论，再给证据。
"@
$context | python $script extract --json

# 也支持直接传 context
python $script extract --context "user: 只查原因，不要改代码。assistant: 我会先定位真实证据源。user: 先给结论，再给证据。" --json
```

## Output Contract

- 脚本输出必须包含：
  - `candidate_count`
  - `candidates`
  - `batch_title`
  - `batch_content`
- 每个 candidate 至少包含：
  - `title`
  - `content`
  - `evidence`
  - `source`
- 技能层输出要求：
  - 先展示候选清单
  - 再走选择交互确认
  - 确认后才允许调用 `$rule-add`

## Atomic Rule Policy

- 每条规则只允许表达一个约束。
- 默认标题范围固定为：
  - `范围约束`
  - `输出顺序`
  - `禁止项`
  - `输出格式`
  - `验收要求`
  - `执行优先级`
  - `需求约束`
- 下面这些情况必须继续拆分或直接丢弃：
  - 一条里同时出现多个并列要求
  - 一条里既有约束又有背景解释
  - 一条长到不方便肉眼验证

## Guardrails

- 默认分析范围只限最近连续几轮当前对话，不读取长期记忆，不跑历史会话检索。
- 默认允许“全量语义推断”，但每条候选必须能给出简短依据，不能空想。
- 不要因为 assistant 自己说过一句话，就把纯自嗨承诺写成规则；默认只提炼用户近轮表达，除非用户明确复述或确认。
- 不允许未经选择交互确认就直接写入会话规则。
- 若最终没有足够清晰的候选规则，明确返回“无候选”，不要硬凑。
- 候选规则写入时必须复用现有 `$rule-add`，不要另起存储目录或第二套规则格式。



