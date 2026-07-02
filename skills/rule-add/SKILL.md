---
name: rule-add
description: Add a session or project rule.
---

# rule-add

## Overview

`$rule-add` 是“新增规则”的唯一 canonical owner，用于把规则写入当前会话或项目共享规则库。

作用域由 `--scope` 控制：

- `scope=session`：默认值，写入 `.codex/session-rules/<session_id>/rules.yaml`。
- `scope=project`：写入 `.codex/project-rules/rules.yaml`，供后续会话搜索并显式 pick。

它不写长期记忆，也不写 `project-memory`。

它的职责不只是“落盘”，还要先把用户表达整理成可执行规则；别把用户原话一股脑塞进规则仓库。

## Workflow

1. 从当前 `cwd` 推导项目根；若用户显式给出项目根，则以显式参数为准。
2. 从 `CODEX_THREAD_ID` 解析当前会话目录；若没有该环境变量，再降级到其他会话标识。
3. 先把用户输入整理成清晰、单义、可执行的规则文案，再决定是否写入。
4. 若当前处于 Plan Mode，先判断是否存在关键信息缺口；有缺口时，用交互方式只追问 1-3 个必要问题，等用户确认整理后的规则文本后再写入。
5. 若当前不处于 Plan Mode，则直接根据上下文重组用户表达，生成更准确的 `title` 与 `content`，不要原样照抄含糊措辞。
6. 判断规则作用域：临时、本轮、当前会话约束走 `scope=session`；稳定、跨会话复用、项目级约束走 `scope=project`。
7. `scope=session` 初始化 `<project_root>/.codex/session-rules/<session_id>/rules.yaml` 与 `meta.yaml`，追加字段为：`id`、`title`、`content`、`created_at`、`updated_at`。
8. `scope=project` 初始化 `<project_root>/.codex/project-rules/rules.yaml` 与 `meta.yaml`，追加字段为：`id`、`title`、`content`、`status`、`tags`、`created_at`、`updated_at`、`picked_count`、`last_picked_at`。
9. `scope=session` 中若 `--title` 和 `--content` 出现中文分号 `；`，进入批量模式；`scope=project` 不支持批量模式，避免标签和状态语义被猜错。
10. `scope=session` 成功后在回复结尾输出完整的“收集的规则列表”，并立即恢复被打断的原任务执行流；规则写入只是中间动作，不是终点。
11. `scope=project` 成功后提示该规则只进入项目规则库；若希望当前会话立即使用，还需要显式 `$rule-project` pick。

## Mode Handling

- Plan Mode：
  - 目标是“先澄清，再确认，再写入，再立刻执行”，不能跳步。
  - 只有在确实存在歧义、缺少边界、或一条话里混了多层意思时，才提关键问题；别为了走流程硬问废话。
  - 若当前 turn 可用 `request_user_input`，优先用它做 1-3 个关键问题的交互确认；若工具不可用，再退回简短文本确认。
  - 优先把问题收敛到规则真正需要的字段，例如：作用范围、禁止项、输出顺序、执行优先级。
  - 收到回答后，先给出整理后的候选规则文本，请用户确认；确认后立即执行写入，不允许拖到 Plan Mode 退出后再补。
  - 这里的 `Plan Mode` 指当前对话的协作模式，不等同于 `$plan` / `$do-plan` skill。不要因为出现了 “plan” 这个词就切去本地计划技能。
  - 若用户在候选规则后给出 `Implement the plan.` 或等价确认，应把它视为“确认写入规则 + 继续执行当前原任务”的联合授权；写入完成后必须在同一轮恢复原任务推进。
- Non-Plan Mode：
  - 默认认为用户表达可能带口语、省略、情绪化修饰；先做语义归一化，再写规则。
  - 整理后的规则要保留用户真实约束，不要擅自扩权，不要把临时吐槽润色成长期原则。
  - 写入后立刻按该规则执行当前任务，而不是只做记录。

## Commands

```powershell
# 将 $skillRoot 设置为当前 SKILL.md 所在目录；从脚本文件执行时可用 $PSScriptRoot。
$skillRoot = "C:\path\to\rule-system\skills\rule-add"
$pluginRoot = Resolve-Path (Join-Path $skillRoot "..\..")
$script = Join-Path $pluginRoot "scripts/session_rules.py"

# 默认：写入当前项目当前会话
python $script add --title "范围约束" --content "只查原因，不改代码"

# 项目级共享规则：稳定、跨会话复用的规则写入项目规则库
python $script add --scope project --title "输出格式" --content "命令和路径使用反引号包裹" --tags "output,format"

# 批量：标题和内容都用中文分号 `；` 对齐拆分
python $script add --title "范围约束；输出格式；禁止项" --content "只查原因，不改代码；先给结论，再给证据；不要扩题"

# 指定项目根和会话 ID（主要用于调试或回归）
python $script add --project-root C:\path\to\project --session-id my-session --title "输出格式" --content "先给结论，再给证据"

# JSON 回执
python $script add --title "禁止项" --content "不要扩题" --json
```

## Output Contract

- 默认输出：
  - 一行总览：`当前项目当前会话规则共 N 条。`
  - 完整规则列表：每条仅显示 `content`，格式为 `- 规则内容`
- `scope=project` 输出项目规则库摘要和包含 ID 的规则列表，便于后续 `$rule-project` 操作。
- 技能内要求：
  - 新增前，若做了改写或归一化，需在正文中先用一句话说明“按整理后的规则执行”
  - 新增成功后，回复正文说明新增结果
  - 若本轮任务仍在继续，新增后应立即按该规则推进，而不是停在“已记录”
  - 若本轮原本就在处理更大的实现/分析任务，只允许用极短回执交代规则写入，随后立刻继续原任务；不要把回复收束在规则回执上
  - 回复结尾强制输出完整“收集的规则列表”

## Guardrails

- 不要把新增规则写进长期记忆或 `project-memory`。
- 允许两种新增入口：显式调用 `$rule-add`，或由 `$rule-system` 路由在识别到会影响当前任务执行方式的规则信号时调用 `$rule-add`。
- 所有 skill 名称必须使用 `rule-*` 前缀；项目规则相关操作统一使用 `$rule-project`。
- `title` 和 `content` 必须非空。
- 不要把未经整理的口语原话、情绪化措辞、或语义残缺的半句直接写入规则文件。
- Plan Mode 下若需要确认，必须先确认后写入；确认通过后必须立即执行，不得延后。
- 不要把 `Plan Mode` 误解为必须切换到 `$plan` 或 `$do-plan`；除非用户显式要求，否则 `rule-add` 只负责规则闭环和恢复原任务。
- 在 Plan Mode 中，规则写入成功后不得停在收集回执；必须恢复被打断的主任务执行流。
- 非 Plan Mode 下即使不追问，也必须先做最小充分的语义重组，确保规则可执行、可验证。
- 批量模式下，`title` 与 `content` 用中文分号 `；` 拆分后的数量必须一致；不一致时直接失败，不猜用户意思。
- 批量模式只适用于 `scope=session`；项目规则新增若需要多条，应逐条调用，避免 tag/status 归属混乱。



