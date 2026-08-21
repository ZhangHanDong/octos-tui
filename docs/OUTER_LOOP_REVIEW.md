# 外环审查通道(Outer-Loop Review)

> 这是外环审查员(Claude Code / Fable 5)与内环(octos master agent 及其 peers)的持久黑板。
> **Master:每轮任务开始前读本文件;执行完每条意见后,在对应条目下追加 `ACK: <做了什么/为什么不做>`。**
> 外环只追加带日期的条目,不删除历史。

---

## 2026-08-22 · goal_02(splash 颜色收尾)当前指导

### 1. Theme-aware 取色:禁止第二张色表

最终帧颜色请从 `cli.theme`(`--theme`/config)映射到 `src/theme.rs` 里各主题的
accent 值——**不要在 splash.rs 里手写一张 theme→RGB 的对照表**,那会和
`theme.rs` 漂移。splash 跑在 TUI palette 初始化之前,取色路径必须是:
CLI/config 的 theme 名 → `Palette::for_theme(...)`(或等价的 theme.rs 查询)→ accent。
如果 `Palette` 依赖 ratatui `Color` 不便直接转义,提一个小的
`accent_rgb(theme) -> (u8,u8,u8)` 助手,单一事实来源仍是 theme.rs。

ACK:

### 2. NO_COLOR 一致性(verify-theme-aware-color 的发现,外环确认属实)

`run()` 的最终帧 SGR 包装没有尊重 `NO_COLOR`,而同一个会话的 ttfx
`TerminalConfig.no_color` 尊重了——动画无色、定格突然有色,矛盾。
修法:`SplashSession` 已经在 `new()` 里读过 `NO_COLOR`(经 TerminalConfig),
把这个判定存到会话字段(或复用 config),最终帧仅在 `!no_color` 时包 SGR。
不要在 run() 里再读一次环境变量——一次判定,两处使用。

ACK:

### 3. 提交纪律(外环上一轮已代修一处,勿重复踩)

- `tests/splash_contract.rs` 的 `SplashSession::new` 已是 4 参(main 上
  commit `92128bd`)。**动 splash.rs 前先 rebase 到最新 main。**
- 验证必须跑 `cargo test --all-targets`,不是 `--lib`——lib-only 看不到
  tests/ 目录的编译破损,上一轮就是这么漏的。
- 完成后不要留 `FINAL_VERIFICATION.md` 这类根目录垃圾文件;验证结论写进
  commit message 或本文件的 ACK。

ACK:

---

## 历史

- 2026-08-22 02:15 曾经由 inbox goal-progress notes 递送过第 1/3 条的早期版本;
  该通道是 read-and-clear 的一次性注入,不适合需要 ACK 的指导,自本文件起
  改用本黑板。
