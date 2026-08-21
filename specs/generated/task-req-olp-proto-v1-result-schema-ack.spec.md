spec: task
name: "协议 v1:result schema、ACK 语法、车道模板、可执行合约"
tags: [requirements, generated-draft, olp, protocol, octoscode]
depends: [REQ-OLP-CTRL, REQ-OLP-EXEC, REQ-OLP-OBS]
satisfies: [REQ-OLP-PROTO]
---

## Intent

OLP v0 的 R2(诚实验证)与 R1(ACK)靠自然语言约定;分歧路径
(内环有理由拒绝外环意见)从未定形;苦力车道机制存在但无开箱配置;
协议本身没有可执行合约钉住。

## Decisions

- Generated draft from KLL requirement artifact; human review must confirm boundaries and test selectors before implementation.
- result.md frontmatter MUST 固化 schema:必含
- slug、outcome、updated_unix、turn、verified、protocol(olp/v1);未知
- 字段消费方 MUST 忽略。
- 黑板 ACK MUST 采用定式语法
- `ACK(done|wontdo|blocked): <说明>`;对 `wontdo` 外环 MUST 接受或升级
- operator,MUST NOT 循环打回同一条目。
- octoscode 文档 MUST 提供 sub_providers 车道配置
- 模板(cheap/strong 各一例,description 写明选道标准)及与双环的推荐
- 搭配矩阵(分析/验证→cheap,实施→primary,keeper→primary)。
- 仓库 MUST 有 `specs/task-agent-runtime-olp.spec`
- 可执行合约,其场景绑定 REQ-OLP-OBS/EXEC/CTRL 关键行为的集成测试,
- 随 CI 运行。

## Boundaries

### Allowed Changes
- src/**
- tests/**

### Forbidden
- Do not weaken or remove the source requirement clauses.
- Do not mark this generated draft complete until each `Test:` selector names a real test.

## Completion Criteria

Scenario: 交付带协议版本与验证级别
  Test: pending_req_olp_proto_requirement
  Given 一个 goal-scoped peer 在 olp/v1 运行时完成交付
  When 读取 result.md frontmatter
  Then protocol 字段为 olp/v1 且 verified 字段存在

Scenario: wontdo 分歧路径终止于升级而非循环
  Test: pending_req_olp_proto_wontdo
  Given 黑板某条目的 ACK 为 wontdo 及理由
  When 外环处理该 ACK
  Then 外环在同一条目下追加"接受"或"升级 operator",不再产生新打回

Scenario: 车道模板可直接生效
  Test: pending_req_olp_proto_requirement
  Given 按文档模板配置 cheap 车道
  When master handoff 时传 model: cheap
  Then peer_staged 事件的 model_lane 为 cheap

## Questions

- Source trace: proposal:LEP-001(§3 E1/E3/D1), 实测:两次失实"已验证"声明促成 verified 字段;黑板至今全为顺从 ACK,, wontdo 分支未演练(闭环评估的缺口 2)。
- Replace pending test selectors with real test names before lifecycle verification.
