---
kind: requirement
id: REQ-OLP-EVO-RETRO
title: "进化环阶段 1:retro 入口、两条固定批注行形、issue 模板"
status: accepted
liveness: auto
tags: [olp, evolution, harness, retro]
---

## Problem

阶段 0 让感知机械化(采集哨落卡)、记录入库(缺陷记录目录),但"诊断"仍靠外环人肉:读一堆卡、手数复发、手写记录骨架、手起草 issue。本阶段给外环一个机械的 retro 入口:把上次 retro 之后的卡按候选分组、数复发、给出层提示与记录骨架,输出一份 retro 简报;判断(归层、锚定、立案)仍由持主审锁的外环做。同时把外环两种高价值批注(改判、R2 记档)定成固定行形,让采集哨也能收它们。

## Requirements

[REQ-OLP-EVO-RETRO-INPUT] `scripts/olp-evo-retro.sh <repo-root> [--dry-run]` MUST 只读取 `<repo>/.octos/EVOLUTION.md` 中编号大于 `retro.json` 记录的 `last_id` 的卡,并 MUST NOT 读取或写入审查活板。

[REQ-OLP-EVO-RETRO-GROUP] 脚本 MUST 按候选键 = `trigger` + 归一化后的 `symptom`(小写、去数字与十六进制串与路径、去首尾空白、截 60 字符)把卡分组,每组一个候选。

[REQ-OLP-EVO-RETRO-RECURRENCE] 每个候选 MUST 给出 `recurrence_hint` = 该组卡中互不相同的来源锚点数(活板卡取条目编号,events 卡取 goal_id 或 slug 或 session,MCP 卡取 ask id),同一锚点重复只计一次。

[REQ-OLP-EVO-RETRO-LAYER] 每个候选 MUST 带一个层提示,取自固定表:`ack_blocked`、`ack_wontdo`、`goal_blocked`、`goal_budget_limited`、`escalation` → Lifecycle;`report_blocked`、`ask_outer_timeout` → Tooling·Context;`turn_error` → Execution·Lifecycle;`override` → 规程·任务书;`r2_record` → Verification。

[REQ-OLP-EVO-RETRO-BRIEF] 脚本 MUST 把简报写到 `<OLP_EVO_STATE 或 ~/.octos/outer/evo>/<项目键>/retro/<UTC 时间戳>.md`,简报 MUST 含卡片数、候选数、每个候选的键、触发器、层提示、recurrence_hint、卡片编号列表,以及一段可直接复制为 `knowledge/context/evolution/FLAW-NNN.md` 的记录骨架。

[REQ-OLP-EVO-RETRO-NEXTID] 记录骨架的 `id` MUST 取 `knowledge/context/evolution/FLAW-*.md` 现有最大编号加一,候选之间依次递增。

[REQ-OLP-EVO-RETRO-CURSOR] 非 dry-run 运行结束时脚本 MUST 把本次处理的最大卡编号写入 `retro.json` 的 `last_id`;再次运行 MUST 报告零新卡且不再生成简报。

[REQ-OLP-EVO-RETRO-DRYRUN] `--dry-run` MUST 把简报打印到 stdout,并 MUST NOT 创建或修改 `retro.json` 与 `retro/` 目录。

[REQ-OLP-EVO-RETRO-MALFORMED] 缺少 `trigger:`、`identity:` 或 `symptom:` 行的卡 MUST 在 stderr 以 `malformed-card: EVO-NNNN` 报告并跳过,MUST NOT 中止运行。

[REQ-OLP-EVO-RETRO-EMPTY] 没有新卡时脚本 MUST 以退出码 0 结束并在 stdout 打印 `retro: 0 new card(s)`。

[REQ-OLP-EVO-RETRO-OVERRIDE] 采集哨 MUST 把活板中去掉前导 `> ` 与空白后以 `改判(` 开头的行识别为触发器 `override`,以 `R2 记档(` 开头的行识别为 `r2_record`;正文中间出现的同名子串 MUST NOT 触发。

[REQ-OLP-EVO-RETRO-ISSUE] `knowledge/context/evolution/ISSUE-template.md` MUST 存在并含 Summary、Environment、Reproduction、Root cause、Expected behavior、Tests requested、Related 七节标题。

[REQ-OLP-EVO-RETRO-SKILL] `/octoloop` skill 卡的 outer 模式 MUST 写明 retro 步骤:何时运行(战役收官,或进化黑板新卡 ≥ 10 张)、命令、简报处置(每次最多推进 3 条记录、立案阈值跨 goal 复发 ≥ 2 或 S1)、issue 发布归 operator。

[REQ-OLP-EVO-RETRO-BOOT] `docs/OLP_OUTER_BOOT.md` §7 MUST 给出改判与 R2 记档两条批注的固定行形与示例。

## Scenarios

Scenario: 三张卡分成两个候选并数出复发
  Given 进化黑板含两张 trigger 为 ack_blocked、symptom 仅数字不同且来自 ### 12 与 ### 13 的卡,和一张 trigger 为 turn_error 的卡
  When 运行 olp-evo-retro.sh
  Then 简报含 `candidates: 2`
  And ack_blocked 候选的 recurrence_hint 等于 2
  And turn_error 候选的层提示含 Execution

Scenario: 记录骨架使用下一个 FLAW 编号
  Given 记录目录已有 FLAW-001 与 FLAW-002
  When 运行 olp-evo-retro.sh 得到两个候选
  Then 简报中的骨架 id 依次为 FLAW-003 与 FLAW-004

Scenario: 游标推进后重跑零新卡
  Given 已运行过一次 retro
  When 再次运行 olp-evo-retro.sh
  Then stdout 含 `retro: 0 new card(s)`
  And retro 目录中的简报文件数不变

Scenario: dry-run 零写入
  Given 进化黑板含新卡
  When 以 --dry-run 运行 olp-evo-retro.sh
  Then stdout 含 `candidates:`
  And 状态目录中不存在 retro.json 与 retro 目录

Scenario: 畸形卡被报告并跳过
  Given 进化黑板含一张缺少 identity 行的卡与一张完整的卡
  When 运行 olp-evo-retro.sh
  Then stderr 含 `malformed-card:`
  And 简报含 `candidates: 1`

Scenario: 改判与 R2 记档行形触发采集
  Given 活板含一行 `> 改判(作废 #40):以本条为准` 与一行 `> R2 记档(#41):声称 verified 复验不符`,另有正文提到"改判(" 的普通句子
  When 运行 olp-evo-harvest.sh
  Then 进化黑板恰新增两张卡,trigger 分别为 override 与 r2_record

Scenario: issue 模板与 skill 卡、BOOT 就位
  Given 仓库检出
  When 读取 knowledge/context/evolution/ISSUE-template.md、.claude/skills/octoloop/SKILL.md、docs/OLP_OUTER_BOOT.md
  Then 模板含七节标题
  And skill 卡 outer 模式含 `olp-evo-retro.sh`
  And BOOT §7 含 `改判(` 与 `R2 记档(` 行形

## Dependencies

- REQ-OLP-EVO(卡片格式、进化黑板、记录目录)
- REQ-OLP-PROTO(黑板与 ACK 定式)

## Source Trace

- proposal:LEP-003 §Decision 第 2、3 项(retro、立案)与 §Unresolved Questions(改判/R2 定式)
- operator 2026-09-05 直令"开始阶段 1,依然 sdd"
- 实测:2026-09-04 阶段 0 首个闭环(FLAW-001/002 → octos #2236/#2237 → PR #2240/#2241)全程人肉 retro

## Open Questions

- 候选键的归一化规则在真实卡片上的误合并率,阶段 2 用回放夹具校准。

## Next

Single exit: compile this requirement into a task contract(`specs/task-req-olp-evo-p1.spec.md`)。
