spec: task
name: "启动动画：ttfx 渲染 OCTOS logo splash"
inherits: project
tags: [tui, startup, splash, ttfx, branding]
estimate: 1d
---

## 意图

octoscode 启动时（`backend_ensure` 之后、`event_loop::run` 接管终端之前）在主屏播放
一段 OCTOS ASCII logo 动画，用 [ttfx](https://github.com/omacom-io/ttfx) 引擎的公开
原语（`Effect::build`/`next_frame` + `Terminal` 帧原语）在 octos-tui 侧自建帧循环渲染。
每次启动从精选效果列表随机抽一个，播放受 1500ms 上限与按键跳过约束；结束时（无论跑完
还是截断）在原地留下完整 logo + 版本号作为 banner，然后正常进入 TUI。动画是纯装饰，
任何失败都静默跳过，绝不阻断启动。

## 已定决策

- **依赖**：`Cargo.toml` 新增 `ttfx = { git = "https://github.com/omacom-io/ttfx", rev = "<pin>" }`
  （pin 到当前 main tip），`.cargo/config.toml.example` 补一段 patch 到本地
  `../consult/ttfx` 的示例——与 `octos-core` 的 git-dep + 本地 patch 模式一致。
  新增依赖的正当理由：本任务即为集成该引擎；ttfx 自身仅依赖 clap + terminal_size。
- **挂载点**：`src/main.rs` 中 `backend_ensure::ensure_octos_backend` 之后、
  `event_loop::run(cli)` 之前调用 `splash::play(&cli)`；`update`/`doctor` 在更早的
  `cmd::dispatch` 已退出，天然不播。
- **门控**：`splash::should_play(inputs) -> bool` 为纯函数（入参打包 no_splash 标志、
  `OCTOSCODE_NO_SPLASH` 环境变量、stdout `IsTerminal`、`CI` 环境变量、终端宽高与 logo
  尺寸），任一跳过条件命中即返回 false。CLI 新增 `--no-splash` 标志。
- **内容**：`OCTOS` figlet 风格 ASCII art（const 字符串，约 40 列宽）+ 尾行
  `octoscode v{CARGO_PKG_VERSION}`。
- **随机效果**：从精选列表 `SPLASH_EFFECTS`（首版取 `decrypt`、`beams`、`sweep`、
  `wipe`、`slice`、`expand` 六种，观感实测后可调整成员）中随机抽取；随机种子取
  `SystemTime` 纳秒，选择函数 `pick_effect(seed) -> EffectCommand` 可用固定种子单测。
- **帧循环**：`run_splash(effect, ctx, out, should_stop)` 使用 ttfx 公开原语
  `prep_canvas` → 循环 { `should_stop()` 为真即中断；`next_frame` → `print_frame` →
  `enforce_framerate` } → `restore_cursor`；`out: &mut impl Write` 与 `Clock`（real /
  virtual）均可注入，测试用虚拟时钟 + `Vec<u8>` 收帧、不真实 sleep。生产路径的
  `should_stop` = 超过 1500ms deadline ∨ `crossterm::event::poll(0)` 读到任意按键
  ∨ 终端 resize；播放期间临时开 raw mode（防按键回显），RAII guard 保证异常路径也
  关闭 raw mode 并恢复光标。
- **终态**：循环结束后统一原样打印完整 logo 文本（默认前景色），使截断与跑完的
  视觉终态一致；banner 留在 scrollback，与 inline scrollback 模型不冲突。
- **失败静默**：`splash::play` 返回 `()`，内部 `run` 的任何 `Err`（引擎错误、IO 错误、
  终端探测失败）都被吞掉，启动继续。

## 边界

### Allowed Changes
- Cargo.toml
- Cargo.lock
- .cargo/config.toml.example
- src/main.rs
- src/lib.rs
- src/splash.rs
- src/cli.rs
- src/transport.rs
- tests/splash_contract.rs
- specs/**

### Forbidden
- 不改变 `event_loop` 接管终端的时序与方式；splash 不进入 alternate screen。
- splash 的任何失败不得使进程退出或延迟启动超出 deadline 本身。
- 除 `ttfx` 外不新增 crate 依赖。
- 不在本 crate 引入 unsafe（信号处理留在 ttfx 内部，本任务不调用其信号 API）。

## 完成条件

场景: 非 TTY 时自动跳过
  测试: should_play_false_when_stdout_not_tty
  假设 门控入参 stdout_is_tty 为 false 且其余条件均允许播放
  当 调用 should_play
  那么 返回 false

场景: --no-splash 与环境变量关闭
  测试: should_play_false_on_flag_or_env
  假设 门控入参 no_splash 标志为 true 或 OCTOSCODE_NO_SPLASH 已设置
  当 调用 should_play
  那么 返回 false

场景: CI 环境自动跳过
  测试: should_play_false_in_ci
  假设 门控入参 ci 为 true 且其余条件均允许播放
  当 调用 should_play
  那么 返回 false

场景: 终端过窄跳过
  测试: should_play_false_when_terminal_narrower_than_logo
  假设 终端宽度小于 logo 宽度
  当 调用 should_play
  那么 返回 false

场景: 条件齐备时播放
  测试: should_play_true_when_interactive_and_wide_enough
  假设 stdout 为 TTY、无关闭标志与环境变量、非 CI、终端宽度足够
  当 调用 should_play
  那么 返回 true

场景: 随机效果取自精选列表
  测试: pick_effect_stays_in_curated_list
  假设 任意 64 个连续种子值
  当 逐一调用 pick_effect
  那么 每次返回的效果名都属于 SPLASH_EFFECTS 列表

场景: 精选效果虚拟时钟冒烟
  测试: curated_effects_produce_frames_on_virtual_clock
  假设 SPLASH_EFFECTS 中的每个效果与固定种子、虚拟时钟、Vec 写入器
  当 运行 run_splash 且 should_stop 恒为 false
  那么 每个效果产出至少 1 帧且 run_splash 返回 Ok

场景: --no-splash 标志可被 CLI 解析
  测试: cli_parses_no_splash_flag
  当 以 --no-splash 参数解析 Cli
  那么 解析结果的 no_splash 字段为 true

场景: 截断后终态为完整 logo
  测试: truncated_run_ends_with_full_logo
  假设 should_stop 在第 3 帧后返回 true
  当 运行 run_splash 并执行终态打印
  那么 写入器末尾内容包含完整 logo 文本与版本号行

场景: 引擎错误静默不阻断
  测试: play_swallows_engine_errors
  假设 以空输入文本构造 splash 使引擎构建失败
  当 调用内部 run 路径
  那么 返回错误被吞掉且函数正常返回、不 panic

<!-- lint-ack: decision-coverage — ttfx git 依赖由 cargo build 本身机械验证，无需场景 -->
<!-- lint-ack: precedence-fallback-coverage — 门控为无序 OR 组合而非优先级链，五个单条件场景已穷举 -->
<!-- lint-ack: boundary-entry-point — main.rs 挂载点时序（backend_ensure 之后、event_loop 之前）靠人工真机验证，无法在集成测试中机械绑定 -->
<!-- lint-ack: bdd-implementation-detail-step — 本任务交付物即终端渲染输出，断言写入器内容是行为本身 -->
