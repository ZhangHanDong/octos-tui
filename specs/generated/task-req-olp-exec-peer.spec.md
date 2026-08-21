spec: task
name: "执行硬化:peer 工具链、默认隔离、机制化验证"
tags: [requirements, generated-draft, olp, peer, verification, octos]
depends: [REQ-OLP-OBS]
satisfies: [REQ-OLP-EXEC]
---

## Intent

实测三类执行可信度缺陷:peer 的 shell 缺 cargo 只能交付"未验证"结果;
多写者共用工作区仅靠 AGENTS.md 纪律兜底;内环两次以 lib-only 测试的
绿色失实声称"已验证"(CI 实际编译失败)。验证必须从纪律降为机制。

## Decisions

- Generated draft from KLL requirement artifact; human review must confirm boundaries and test selectors before implementation.
- peer 与 master 的工具 shell MUST 继承 operator 的
- PATH,或使用 profile 显式配置的 `tool_path`;两者皆缺时 MUST 在 result
- 中声明工具链不可用。
- `peer_handoff` 的 worktree 缺省 MUST 可由 profile
- 配置;开启后 peer 结束(closed 且成果已被 gather)时 runtime MUST 自动
- 清理其 `wt/` 克隆(operator 2026-08-22 拍板:默认开 + 完成即清)。
- profile MUST 支持 `verify_command` 配置;goal-scoped
- peer 交付时 runtime MUST 执行该命令并把结果(pass|fail|skipped 及原因)
- 写入 result.md frontmatter 的 `verified:` 字段与 goal 账本。
- `verify_command` MUST 仅来自 operator 手写的
- profile 配置;模型工具、黑板、steer 通道 MUST NOT 能写入或修改该字段。

## Boundaries

### Allowed Changes
- src/**
- tests/**

### Forbidden
- Do not weaken or remove the source requirement clauses.
- Do not mark this generated draft complete until each `Test:` selector names a real test.

## Completion Criteria

Scenario: peer 工具链继承
  Test: pending_req_olp_exec_peer
  Given operator 的 PATH 含 cargo 且 profile 未配 tool_path
  When peer 执行 bash 工具运行 cargo --version
  Then 命令成功且输出版本号

Scenario: 交付触发机制化验证并落账
  Test: pending_req_olp_exec_requirement
  Given profile 配置 verify_command 为 cargo test --all-targets
  When 一个 goal-scoped peer 完成交付
  Then result.md frontmatter 含 verified: pass 或 fail,且账本记录同值

Scenario: 验证失败的交付被如实标记
  Test: pending_req_olp_exec_requirement
  Given verify_command 会因编译错误退出非零
  When peer 声称完成并交付
  Then verified: fail 且外环可从事件流看到 turn_error 或 fail 记录

Scenario: worktree 完成即清
  Test: pending_req_olp_exec_worktree
  Given profile 开启默认 worktree
  When peer closed 且成果已 gather
  Then peers/<slug>/wt/ 被删除而 result.md 与账本保留

## Questions

- Source trace: proposal:LEP-001(§3 C1/C2/C3;operator 2026-08-22 拍板 C2 取, "默认开 + 完成即清"), 实测:peer 报告"工具链确实有问题";两次 lib-only 失实验证, (specs 黑板第 3/6 条)。
- Replace pending test selectors with real test names before lifecycle verification.
