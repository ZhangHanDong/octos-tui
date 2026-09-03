spec: task
name: "进化环阶段 0:三源采集哨、进化黑板、缺陷记录目录(只读影子试点)"
tags: [olp, evolution, harness, observability]
satisfies: [REQ-OLP-EVO]
estimate: 1d
---

## 意图

为 OctoLoop 外环增加一个机械的、幂等的采集面:`scripts/olp-evo-harvest.sh` 从
活板、events.jsonl、MCP 审计板三个既有来源增量识别内环摩擦,落成症状卡追加到
`<repo>/.octos/EVOLUTION.md`;同时在本仓库建立缺陷记录目录
`knowledge/context/evolution/`,回填 octos #2236 与 #2237 两条记录。这是 LEP-003
进化环的阶段 0,只读影子试点:不改运行时代码、不改协议、不写 ACK。

## 已定决策

- 脚本用法 `scripts/olp-evo-harvest.sh <repo-root> [--dry-run]`;来源路径由环境
  变量给定:`OLP_EVO_REVIEW_BOARD`(缺省 `<repo>/.octos/OUTER_LOOP_REVIEW.md`)、
  `OLP_EVO_EVENTS`(缺省空,空即跳过)、`OLP_EVO_MCP_BOARD`(缺省
  `~/.octos/outer/OUTER_LOOP_MCP.md`)、`OLP_EVO_BOARD`(缺省
  `<repo>/.octos/EVOLUTION.md`)、`OLP_EVO_STATE`(缺省 `~/.octos/outer/evo`)。
- 状态文件:`<state>/cursor.json`(每源 `{path, lines, bytes}`)、`<state>/next_id`、
  `<state>/seen.txt`(每行一个 sha256 前 16 位);三者与卡片写入在
  `<state>/harvest.lock` 的同一把 `flock -x` 内完成。
- 卡片经 `scripts/olp-board-append.sh` 追加(正文走 stdin),格式:
  `### EVO-NNNN（<UTC 时间戳>，harvest）` 后跟 `trigger:`、`source:`、`envelope:`、
  `symptom:` 四个字段行;`symptom` 取触发行去掉前导标记后的前 200 个字符。
- 触发器识别用 `grep -F` 子串,不猜前缀格式:活板 `ACK(blocked)`、`ACK(wontdo)`、
  `作废 #`、`R2 记档`;events.jsonl 用 python3 标准库解析 JSON,kind 为
  `escalation`、`turn_error`,或 kind 为 `goal_transition` 且 detail 含 `blocked`
  或 `budget_limited`;MCP 审计板 `ask_outer`、`report_blocked`。
- 截断检测:来源当前字节数小于游标字节数即视为截断,stderr 打印
  `truncated: <path>` 并把该源游标重置为 0,再靠 `seen.txt` 去重。
- 退出码:活板缺失 2;其它来源缺失跳过并在 stderr 打印 `skip: <path>`,退出 0;
  `--dry-run` 只打印卡到 stdout。
<!-- lint-ack: decision-coverage — 用法、追加方式、测试形态三条决策由全部场景共同行使,不单列场景 -->
- 测试为 Rust 集成测试 `tests/olp_evo_harvest.rs`,通过 `std::process::Command`
  调用脚本,夹具复制到 `tempfile`/`std::env::temp_dir()` 下的临时目录,状态目录
  经 `OLP_EVO_STATE` 指向临时目录;夹具源文件放 `fixtures/evolution/`。
- 记录目录文件:`README.md`(索引与状态机)、`FLAW-template.md`、`memory.md`
  (表头行)、`operators.md`(算子表与固定禁改段,内容取自 LEP-003 设计文档
  §7)、`FLAW-001.md`、`FLAW-002.md`(内容取自 `~/.octos/outer/evo/issues/` 两份
  草稿,frontmatter 含 `kind: context`、`id`、`repo: octos`、`layers`、
  `status: filed`、`severity`、`recurrence`、`fingerprint`、`issue`)。
- `scripts/olp-init.sh`:在既有黑板 gitignore 块之后追加同样式的
  `.octos/EVOLUTION.md` 忽略逻辑,整个 `.octos/` 已忽略时跳过。
- 不新增 Cargo 依赖;脚本只依赖 bash、coreutils、flock、python3、sha256sum。

## 边界

### Allowed Changes
- scripts/olp-evo-harvest.sh
- scripts/olp-init.sh
- tests/olp_evo_harvest.rs
- fixtures/evolution/**
- knowledge/context/evolution/**
- docs/OCTOLOOP_FEATURES.md

### Forbidden
- 不改 `src/**` 任何运行时代码。
- 不改 `AGENTS.md`、`.octos/loop.md`、`.claude/skills/**`、
  `docs/OUTER_LOOP_PROTOCOL.md`、`docs/OLP_OUTER_BOOT.md`、`tests/olp_contract.rs`。
- 不向活板 `OUTER_LOOP_REVIEW.md` 写入任何内容,不生成任何 `ACK(` 开头的行。
- 不新增 Cargo 依赖,不新增 MCP 工具,不改 `~/.octos/outer/mcp/` 路径语义。
- 脚本状态不得落 /tmp(测试通过 `OLP_EVO_STATE` 指向临时目录是测试夹具,不是缺省)。

## 排除范围

- retro 子命令与 skill 卡改动(阶段 1,operator-tier)。
- events.jsonl 实例自动发现(阶段 2)。
- octos 侧新增事件 producer(阶段 2,REQ-OLP-OBS 修订)。
- 指标脚本 `olp-evo-metrics.sh` 与回放夹具制作。
- 改判与 R2 打回的固定定式修订(规程改动,operator-tier)。

## 完成条件

场景: 三源各一条新触发行落三张卡(critical)
  标签: critical
  测试: olp_evo_harvest_produces_cards_from_three_sources
  假设 夹具活板含一行 ACK(blocked),events.jsonl 含一行 kind=turn_error,MCP 审计板含一条 report_blocked 条目
  当 以空状态目录运行 olp-evo-harvest.sh
  那么 进化黑板新增三张以 ### EVO- 开头的卡
  并且 编号为 EVO-0001、EVO-0002、EVO-0003
  并且 每张卡含 envelope 与 symptom 字段行

场景: 重复运行不重复落卡
  测试: olp_evo_harvest_is_idempotent_on_rerun
  假设 三源场景已运行一次
  当 再次运行 olp-evo-harvest.sh
  那么 进化黑板字节不变
  并且 cursor.json 字节不变

场景: 采集从不触碰活板
  标签: critical
  测试: olp_evo_harvest_never_writes_review_board_or_ack
  假设 任意夹具
  当 运行 olp-evo-harvest.sh
  那么 活板 sha256 与运行前一致
  并且 进化黑板中不存在以 ACK( 开头的行

场景: docs 冻结快照被忽略
  测试: olp_evo_harvest_ignores_docs_snapshot
  假设 docs/OUTER_LOOP_REVIEW.md 含一行新的 ACK(blocked) 而活板无新行
  当 运行 olp-evo-harvest.sh
  那么 进化黑板不新增卡

场景: 来源截断后重置游标且不重复
  测试: olp_evo_harvest_resets_cursor_on_truncation
  假设 已采集过的 events.jsonl 被截断为只剩最后一行且该行已采集
  当 运行 olp-evo-harvest.sh
  那么 stderr 含 truncated:
  并且 进化黑板不新增卡
  并且 cursor.json 中该源 lines 等于 1

场景: 活板缺失即失败
  测试: olp_evo_harvest_fails_without_review_board
  假设 仓库目录下不存在 .octos/OUTER_LOOP_REVIEW.md
  当 运行 olp-evo-harvest.sh
  那么 退出码为 2
  并且 进化黑板与状态目录均不存在

场景: dry-run 零写入
  测试: olp_evo_harvest_dry_run_writes_nothing
  假设 夹具含一条新触发行
  当 以 --dry-run 运行 olp-evo-harvest.sh
  那么 stdout 含 ### EVO-
  并且 进化黑板与状态目录均不存在

场景: 缺省可选来源缺失时跳过并退出 0
  测试: olp_evo_harvest_skips_missing_optional_sources
  假设 只有活板存在,OLP_EVO_EVENTS 为空且 OLP_EVO_MCP_BOARD 指向不存在的路径
  当 运行 olp-evo-harvest.sh
  那么 退出码为 0
  并且 stderr 含 skip:

场景: 记录目录与首两条记录就位
  测试: olp_evo_records_dir_backfilled_from_issues
  假设 仓库检出
  当 读取 knowledge/context/evolution/
  那么 存在 README.md、FLAW-template.md、memory.md、operators.md
  并且 FLAW-001.md 的 frontmatter 含 issues/2236
  并且 FLAW-002.md 的 frontmatter 含 issues/2237

场景: olp-init 为未忽略 .octos 的项目追加 EVOLUTION.md 忽略
  测试: olp_evo_init_appends_evolution_gitignore
  假设 一个临时 git 仓库,.gitignore 不含 .octos
  当 运行 scripts/olp-init.sh
  那么 .gitignore 含 .octos/EVOLUTION.md
