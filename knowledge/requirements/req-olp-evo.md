---
kind: requirement
id: REQ-OLP-EVO
title: "进化环阶段 0:外环侧三源采集哨、进化黑板、缺陷记录目录"
status: accepted
liveness: auto
tags: [olp, evolution, harness, observability]
---

## Problem

内环 harness 的缺陷(预算耗尽无 ACK、围栏 peer 冷编译、goal 陈旧卡态)只在事故后靠人肉复盘沉淀成散文,下一场战役再踩。外环需要一个机械的、幂等的采集面,把内环摩擦从三个既有来源增量落成症状卡,并有一个入库的缺陷记录目录供主审合并与立案。阶段 0 只读:不改运行时、不改协议、不写 ACK。

## Requirements

[REQ-OLP-EVO-SOURCES] 采集脚本 `scripts/olp-evo-harvest.sh` MUST 只从活板 `<repo>/.octos/OUTER_LOOP_REVIEW.md`、实例 `events.jsonl`、MCP 审计板 `OUTER_LOOP_MCP.md` 三个来源读取,并 MUST NOT 读取 `docs/OUTER_LOOP_REVIEW.md` 冻结快照。

[REQ-OLP-EVO-TRIGGERS] 采集脚本 MUST 把活板中的 `ACK(blocked)`、`ACK(wontdo)`、`作废 #`、`R2 记档` 行,events.jsonl 中 kind 为 `escalation`、`turn_error` 或 kind 为 `goal_transition` 且 detail 含 `blocked`/`budget_limited` 的行,以及 MCP 审计板中含 `ask_outer`/`report_blocked` 的条目行,各识别为一个触发器并各落一张卡。

[REQ-OLP-EVO-CARD] 每张卡 MUST 以 `### EVO-NNNN` 开头并携带 `trigger`、`source`、`envelope`(来源路径、行号、内容 sha256 前 16 位、采集时间戳)、`symptom` 四个字段行,且 MUST NOT 含建议或修复方案字段。

[REQ-OLP-EVO-IDEMPOTENT] 对同一来源状态重复运行采集脚本 MUST NOT 产生重复卡片;脚本 MUST 以每源游标(行数与字节数)加内容摘要去重。

[REQ-OLP-EVO-TRUNCATE] 来源当前字节数小于游标字节数时,脚本 MUST 重置该源游标并以内容摘要去重,MUST NOT 重复落卡,并 MUST 在 stderr 打印 `truncated:` 警告。

[REQ-OLP-EVO-NOACK] 采集脚本 MUST NOT 向活板写入任何内容,MUST NOT 生成任何以 `ACK(` 开头的行;进化黑板 `<repo>/.octos/EVOLUTION.md` 是其唯一写目标。

[REQ-OLP-EVO-STATE] 游标、取号与已见摘要 MUST 持久化在外环状态目录(缺省 `~/.octos/outer/evo/`,经环境变量 `OLP_EVO_STATE` 覆盖),MUST NOT 缺省落 /tmp,且读游标、写卡、写游标 MUST 在同一把 flock 内完成。

[REQ-OLP-EVO-ID] EVO 编号 MUST 从 `EVO-0001` 起单调递增且在取号锁内分配。

[REQ-OLP-EVO-REQUIRED] 活板缺失时脚本 MUST 以退出码 2 失败且 MUST NOT 创建进化黑板或状态目录。

[REQ-OLP-EVO-OPTIONAL] events.jsonl 或 MCP 审计板缺失时脚本 MUST 跳过该源、在 stderr 打印 `skip:` 并以退出码 0 结束。

[REQ-OLP-EVO-DRYRUN] `--dry-run` MUST 把将要落的卡打印到 stdout,且 MUST NOT 写进化黑板、游标或取号。

[REQ-OLP-EVO-RECORDS] 缺陷记录、修复记忆、算子表 MUST 位于本仓库 `knowledge/context/evolution/`,记录 frontmatter MUST 含 `kind: context`、`id`、`repo`、`layers`、`status`、`severity`、`recurrence`、`fingerprint` 字段。

[REQ-OLP-EVO-ISSUE] 状态为 `filed` 及之后的缺陷记录 MUST 在 frontmatter 含 `issue` 链接。

[REQ-OLP-EVO-INIT] `scripts/olp-init.sh` 在目标项目未忽略整个 `.octos/` 时 MUST 把 `.octos/EVOLUTION.md` 追加进 `.gitignore`,已忽略时 MUST 跳过。

## Scenarios

Scenario: 三源各有一条新触发行时落三张卡
  Given 夹具活板含一行 ACK(blocked),events.jsonl 含一行 kind=turn_error,MCP 审计板含一条 report_blocked 条目
  When 以空状态目录运行 olp-evo-harvest.sh
  Then 进化黑板中以 `### EVO-` 开头的行数等于 3
  And 三张卡的编号依次为 EVO-0001、EVO-0002、EVO-0003
  And 每张卡含以 `envelope:` 与 `symptom:` 开头的行

Scenario: 重复运行不重复落卡
  Given 上一场景运行完毕
  When 再次运行 olp-evo-harvest.sh
  Then 进化黑板的 sha256 与运行前相等
  And cursor.json 的 sha256 与运行前相等

Scenario: 采集从不触碰活板
  Given 任意夹具
  When 运行 olp-evo-harvest.sh
  Then 活板的 sha256 与运行前相等
  And 进化黑板中以 `ACK(` 开头的行数等于 0

Scenario: docs 冻结快照被忽略
  Given docs/OUTER_LOOP_REVIEW.md 含一行新的 ACK(blocked) 而活板无新行
  When 运行 olp-evo-harvest.sh
  Then 进化黑板中以 `### EVO-` 开头的行数与运行前相等

Scenario: 来源截断后重置游标且不重复
  Given 已采集过的 events.jsonl 被截断为只剩最后一行且该行已采集
  When 运行 olp-evo-harvest.sh
  Then stderr 含 `truncated:`
  And 进化黑板中以 `### EVO-` 开头的行数与运行前相等
  And cursor.json 中该源的 lines 字段等于 1

Scenario: 活板缺失即失败
  Given 仓库目录下不存在 .octos/OUTER_LOOP_REVIEW.md
  When 运行 olp-evo-harvest.sh
  Then 退出码等于 2
  And 进化黑板文件不存在
  And 状态目录不存在

Scenario: dry-run 零写入
  Given 夹具含一条新触发行
  When 以 --dry-run 运行 olp-evo-harvest.sh
  Then stdout 含 `### EVO-`
  And 进化黑板文件不存在
  And 状态目录不存在

Scenario: 可选来源缺失时跳过
  Given 只有活板存在,OLP_EVO_EVENTS 为空且 OLP_EVO_MCP_BOARD 指向不存在的路径
  When 运行 olp-evo-harvest.sh
  Then 退出码等于 0
  And stderr 含 `skip:`

Scenario: 记录目录与首两条记录就位
  Given 仓库检出
  When 读取 knowledge/context/evolution/
  Then README.md、FLAW-template.md、memory.md、operators.md 四个文件存在
  And FLAW-001.md 的 frontmatter 含 `issues/2236`
  And FLAW-002.md 的 frontmatter 含 `issues/2237`

Scenario: olp-init 为未忽略 .octos 的项目追加 EVOLUTION.md 忽略
  Given 一个临时 git 仓库,其 .gitignore 不含 .octos
  When 运行 scripts/olp-init.sh
  Then .gitignore 含 `.octos/EVOLUTION.md`

## Dependencies

- REQ-OLP-OBS(events.jsonl 字段与 kind 集合)
- REQ-OLP-PROTO(ACK v1 定式)

## Source Trace

- proposal:LEP-003(operator 2026-09-04 直令"进化环开始落地,依然 sdd")
- issue:octos-org/octos#2236、#2237(进化环首两条候选,已立案)
- 实测:2026-09-04 octos 活板 #45 战役,围栏 peer 冷编译耗尽 50 迭代、goal_create 被 archived 挡住

## Open Questions

- events.jsonl 的实例发现:阶段 0 由调用方显式传路径,自动发现留待阶段 2。

## Next

Single exit: compile this requirement into a task contract(`specs/task-req-olp-evo-p0.spec.md`)。
