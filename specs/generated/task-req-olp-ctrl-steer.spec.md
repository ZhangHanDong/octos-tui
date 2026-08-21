spec: task
name: "控制面:带确认的 steer、审查者通道、升级通知"
tags: [requirements, generated-draft, olp, steer, escalation, octos]
depends: [REQ-OLP-OBS]
satisfies: [REQ-OLP-CTRL]
---

## Intent

外环对 idle 内环的唯一唤起方式是"inbox 门铃 + operator 说一句话"
(实测:goal complete 后 master 完全 idle,黑板新条目无人读)。无人值守
长程需要外环可编程地下达带机器可读确认的指导;escalation 在无人值守时
静默 park 需要外部通知。

## Decisions

- Generated draft from KLL requirement artifact; human review must confirm boundaries and test selectors before implementation.
- `octos` MUST 提供 `steer --session <key> --text <指令>`
- 子命令(operator 2026-08-22 拍板:CLI 落盘唤醒,不新增网络端点):指令
- 持久排队、注入目标 session 的下一 turn、并复用 goal wake 机制唤起
- continuation。
- 被消费的 steer MUST 在 events.jsonl 产生
- `steer_consumed` 事件(含投递时间与消费 turn id),构成机器可读回执。
- steer 文本 MUST 以 `[external-reviewer]` 来源标记
- 注入,信任级别与 peer brief 相同(数据,非系统指令);单 turn 注入总量
- 沿用 notes 的 64KiB 读取上限。
- goal-scoped escalation 记录时,若 profile 配置了
- 通知通道(复用 octos cron notify mode),runtime MUST 向 operator 发送
- 外部通知。

## Boundaries

### Allowed Changes
- src/**
- tests/**

### Forbidden
- Do not weaken or remove the source requirement clauses.
- Do not mark this generated draft complete until each `Test:` selector names a real test.

## Completion Criteria

Scenario: steer 唤醒 idle master 并留下回执
  Test: pending_req_olp_ctrl_steer_idle_master
  Given master 无 active goal 且处于 idle
  When 外部进程执行 octos steer --session <master> --text "读黑板第 7 条"
  Then master 的下一 turn prompt 含该指令,且 events.jsonl 出现 steer_consumed

Scenario: steer 不越权
  Test: pending_req_olp_ctrl_steer
  Given 一条试图修改 verify_command 的 steer 文本
  When 该 steer 被消费
  Then profile 配置文件保持不变(steer 是数据不是配置写入通道)

Scenario: escalation 触发外部通知
  Test: pending_req_olp_ctrl_escalation
  Given profile 配置了 notify 通道且一个 goal-scoped peer park 在 approval
  When escalation 写入账本
  Then operator 的通知通道收到一条含 slug 与 goal_id 的消息

## Questions

- Source trace: proposal:LEP-001(§3 B1/B2/F2;operator 2026-08-22 拍板 B1 取 CLI), 实测:门铃模式(docs/OUTER_LOOP_PROTOCOL.md "推/拉间隙")仍需, operator 一句话;verify-splash-blue-color 曾 park 无人批。
- Replace pending test selectors with real test names before lifecycle verification.
