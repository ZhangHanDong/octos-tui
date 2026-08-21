spec: task
name: "Global 运行时生命周期:驾驶舱脚本、注入纪律、headless client"
tags: [requirements, generated-draft, olp, lifecycle, herdr, tmux, headless, octoscode]
depends: [REQ-OLP-CTRL, REQ-OLP-OBS]
satisfies: [REQ-OLP-LIFE]
---

## Intent

外环能观测(REQ-OLP-OBS)、能指导(REQ-OLP-CTRL),但运行时仍要
operator 手动开终端拉起;session client-coupled 契约决定了没有 client
的 serve 无法打开 peer 会话。外环需要合法、可审计、人类可随时接管的
方式发起并驾驶 global 实例。

## Decisions

- Generated draft from KLL requirement artifact; human review must confirm boundaries and test selectors before implementation.
- 仓库 MUST 提供 `scripts/octos-global.sh`,封装
- launch/inject/read/attach 四原语;launch 前 MUST 检查目标 instance 的
- serve 锁,被持有时 MUST 拒绝启动并报告持有者,MUST NOT 抢锁或 kill。
- 脚本 MUST 抽象驾驶舱后端:herdr 可用时优先
- (`herdr agent prompt`/`agent read`/`agent send-keys`),否则回退 tmux
- (`send-keys`/`capture-pane`);两后端对四原语的行为契约 MUST 一致。
- inject 原语 MUST 在注入前读取画面/输出并判定
- 状态:检测到 approval/question 界面时 MUST NOT 注入并 MUST 通知
- operator;审批按键(y/s/n 及其等价)在任何情况下 MUST NOT 经脚本注入。
- 实例非计划终止(驾驶舱会话消失或 serve 锁在无
- 关停指令下释放)时,外环 MUST 通知 operator;MUST NOT 静默自动重启。
- octoscode MUST 提供 `--headless` client 模式
- (阶段 2):承担全部 client 协议职责(capabilities 握手、session/open、
- 消费 peer/staged 打开 peer 会话、事件泵),不渲染、不读键盘;指令入口
- 仅 REQ-OLP-CTRL 的 steer,观测仅 REQ-OLP-OBS;与 TUI 以 serve 锁互斥。

## Boundaries

### Allowed Changes
- src/**
- tests/**

### Forbidden
- Do not weaken or remove the source requirement clauses.
- Do not mark this generated draft complete until each `Test:` selector names a real test.

## Completion Criteria

Scenario: 锁被持有时拒绝启动
  Test: pending_req_olp_life_requirement
  Given operator 的 TUI 实例正持有 serve 锁
  When 外环执行 scripts/octos-global.sh launch
  Then 脚本以非零退出并输出持有者信息,不产生第二个实例

Scenario: approval 画面阻断注入
  Test: pending_req_olp_life_approval
  Given global 实例画面上有可见的 approval 卡
  When 外环调用 inject 原语提交一条 prompt
  Then 注入被拒绝且 operator 收到通知,composer 未收到任何按键

Scenario: 后端等价性
  Test: pending_req_olp_life_requirement
  Given 同一条 prompt 分别经 herdr 后端与 tmux 后端注入空闲 composer
  When master 的下一个 turn 开始
  Then 两种后端下 turn 的用户消息内容一致

Scenario: headless 模式打开 peer 会话
  Test: pending_req_olp_life_headless_peer
  Given octoscode --headless 连接 serve 且 master handoff 了一个 peer
  When peer/staged 通知到达
  Then peer 会话被打开并开始执行(无任何渲染输出)

Scenario: 非计划终止告警
  Test: pending_req_olp_life_requirement
  Given global 实例正在运行且无人下达关停
  When 驾驶舱会话消失
  Then operator 通知通道收到实例终止告警且不发生自动重启

## Questions

- Source trace: proposal:LEP-002(operator 2026-08-22 拍板:脚本化 + herdr 优先/, tmux 回退 + --headless + 抢锁互斥), 实测:锁竞争 = 启动黑屏事故(2026-08-22);pty 驱动验证了技术可行但, 无人类可视界面。
- Replace pending test selectors with real test names before lifecycle verification.
