spec: task
name: "进化环阶段 1:retro 简报脚本、改判/R2 记档固定行形、issue 模板与 skill/BOOT 接入"
tags: [olp, evolution, harness, retro]
satisfies: [REQ-OLP-EVO-RETRO]
estimate: 1d
---

## 意图

阶段 0 之后诊断仍是外环人肉:读卡、数复发、手写记录骨架。本任务给外环一个机械的 retro 入口
`scripts/olp-evo-retro.sh`:读取上次 retro 之后的卡,按候选分组、数复发、给层提示,输出一份带
记录骨架的简报;判断(归层、锚定、立案)仍由持主审锁的外环做。同时把改判与 R2 记档定成固定行形
并让采集哨收它们,补 issue 模板,把 retro 步骤写进 skill 卡 outer 模式与 BOOT §7。不改运行时
代码,不改协议,不写 ACK。

## 已定决策

- 用法 `scripts/olp-evo-retro.sh <repo-root> [--dry-run]`;状态目录复用阶段 0 的
  `<OLP_EVO_STATE 或 ~/.octos/outer/evo>/<sha256(realpath) 前 16 位>/`,新增 `retro.json`
  (`{"last_id":N,"runs":[{"ts":…,"cards":N,"candidates":N,"brief":"<path>"}]}`)与 `retro/`
  目录;进化黑板路径可由 `OLP_EVO_BOARD` 覆盖(缺省 `<repo>/.octos/EVOLUTION.md`)。
- 卡片解析:以 `### EVO-NNNN（` 开头的行起一张卡,直到下一张;必需字段行 `trigger:`、
  `identity:`、`symptom:`,缺任一即 `malformed-card: EVO-NNNN` 到 stderr 并跳过。解析与分组用
  python3 标准库,bash 只做参数、路径与落盘。
- 候选键 = `trigger` + `|` + 归一化 symptom:小写 → 删除十六进制串(≥ 6 位)与纯数字串 → 删除形如
  `/…/…` 的路径片段 → 空白折叠 → 去首尾空白 → 截前 60 字符。
- `recurrence_hint`:从 `identity:` 行取来源锚点——`board:` 取第 2 段(条目编号),`events:` 取第 4 段
  (goal_id|slug|session),`mcp:` 取第 4 段(ask id);去重计数;锚点为 `-` 的各计一次。
- 层提示固定表:ack_blocked、ack_wontdo、goal_blocked、goal_budget_limited、escalation → `Lifecycle`;
  report_blocked、ask_outer_timeout → `Tooling·Context`;turn_error → `Execution·Lifecycle`;
  override → `规程·任务书`;r2_record → `Verification`;未知 trigger → `?`。
- 简报格式(Markdown):首行 `# retro <UTC RFC3339> · <项目键>`;摘要行 `cards: N` 与
  `candidates: N`;每候选一节 `## C<k> <trigger> · recurrence_hint=<n> · layer=<提示>`,含
  `key:`、`cards: EVO-…, EVO-…`、每张卡的 `source`/`envelope` 一行,以及一个 ```yaml 代码块的记录
  骨架(frontmatter 字段:kind: context、id: FLAW-NNN、title(取 symptom 前 60 字)、repo: TODO、
  layers: [<提示>]、status: open、severity: TODO、recurrence: <hint>、fingerprint: TODO、
  cards: [...])。
- 下一个 FLAW 编号:扫描 `<repo>/knowledge/context/evolution/FLAW-[0-9]*.md` 取最大编号加一,候选间
  递增;目录不存在或无记录时从 FLAW-001 起。
- 游标:非 dry-run 且卡片数 > 0 时,写简报后以临时文件 + rename 原子更新 `retro.json.last_id`
  为本次最大卡编号;dry-run 只打印简报到 stdout,不创建 `retro.json`/`retro/`。
- 无新卡:stdout `retro: 0 new card(s)`,退出 0,不写简报。
- 采集哨新增触发器(只改 `scripts/olp-evo-harvest.sh` 活板分支):活板行去掉前导 `> `、`**`、空白后
  以 `改判(` 开头 → `override`,以 `R2 记档(` 开头 → `r2_record`;identity 的 ACK 类型段分别为
  `override`/`r2`;symptom 取该行前 200 字符。
- `knowledge/context/evolution/ISSUE-template.md`:七节标题 `## Summary`、`## Environment`、
  `## Reproduction`、`## Root cause`、`## Expected behavior`、`## Tests requested`、`## Related`,
  每节一行占位说明;首部 frontmatter `repo:`、`evo:`、`layers:`、`severity:`。
- skill 卡:在"模式 outer"第 4 步之后新增第 5 步 "retro(进化环)",内容:触发时机(战役收官,或进化
  黑板新卡 ≥ 10 张)、命令 `scripts/olp-evo-harvest.sh <repo> && scripts/olp-evo-retro.sh <repo>`、
  简报处置(每次最多推进 3 条记录;跨 goal 复发 ≥ 2 或 S1 立案;issue 由 operator 发布或明示委托)、
  authority(未持 outer-duty 锁只读简报不写记录);description frontmatter 不改"三模式"文案。
- BOOT §7 新增小节 "进化环批注定式":改判行 `> 改判(作废 #N):<以本条为准的新指令>`、R2 记档行
  `> R2 记档(#N):<声称 vs 复验事实>`,各一例;写明"行首定式,正文提及不算"。
- `docs/OCTOLOOP_FEATURES.md` "结果与审计"节新增一条"进化环(阶段 0/1)":是什么、缺省状态
  (手动运行)、用户怎么看到(`.octos/EVOLUTION.md` 与 retro 简报);固定短语"外环私有工作纸,不写入
  OLP 信道矩阵,不升协议版本"。
- 测试:`tests/olp_evo_retro.rs`(Rust 集成测试,`std::process::Command` 调脚本,夹具复制到
  `std::env::temp_dir()` 唯一子目录,`OLP_EVO_STATE` 指向临时状态根;夹具放
  `fixtures/evolution/retro/`);采集哨新增触发器的测试加在 `tests/olp_evo_harvest.rs`;文档就位由
  `tests/olp_evo_retro.rs` 内读文件断言。不新增 Cargo 依赖;脚本只依赖 bash、coreutils、python3。

<!-- lint-ack: decision-coverage — 用法/状态形状/简报格式等决策由多个场景共同行使,不单列场景 -->

## 边界

### Allowed Changes
- scripts/olp-evo-retro.sh
- scripts/olp-evo-harvest.sh
- tests/olp_evo_retro.rs
- tests/olp_evo_harvest.rs
- fixtures/evolution/**
- knowledge/context/evolution/ISSUE-template.md
- knowledge/context/evolution/README.md
- .claude/skills/octoloop/SKILL.md
- docs/OLP_OUTER_BOOT.md
- docs/OCTOLOOP_FEATURES.md

### Forbidden
- 不改 `src/**` 任何运行时代码。
- 不改 `AGENTS.md`、`.octos/loop.md`、`docs/OUTER_LOOP_PROTOCOL.md`、`tests/olp_contract.rs`。
- 不改 skill 卡 description frontmatter 与"模式 init/inner"、"自主性纪律"章节。
- 不改 `knowledge/context/evolution/operators.md`、`FLAW-*.md`、`memory.md`。
- 不向审查活板写入任何内容,不生成任何 `ACK(` 开头的行;retro 脚本不写进化黑板、不写记录目录。
- 不新增 Cargo 依赖,不新增 MCP 工具。

## 排除范围

- retro 的判断部分(归层、锚定、立案、写记录)——由外环模型按简报手工完成。
- events.jsonl 新 producer(octos 侧,阶段 2)。
- 指标脚本、回放夹具、采集挂外环 watch 节拍(阶段 2/3)。
- `docs/OUTER_LOOP_PROTOCOL.md` 的任何改动。

## 完成条件

场景: 三张卡分成两个候选并数出复发(critical)
  标签: critical
  测试: olp_evo_retro_groups_cards_and_counts_recurrence
  假设 进化黑板含两张 trigger 为 ack_blocked、symptom 仅数字不同、identity 分别来自条目 12 与 13 的卡,和一张 trigger 为 turn_error 的卡
  当 运行 olp-evo-retro.sh
  那么 简报含 candidates: 2
  并且 ack_blocked 候选行含 recurrence_hint=2
  并且 turn_error 候选行含 layer=Execution

场景: 同一锚点重复不重复计数
  测试: olp_evo_retro_recurrence_dedups_same_anchor
  假设 进化黑板含两张 trigger 为 report_blocked、symptom 相同、identity 的 ask id 相同的卡
  当 运行 olp-evo-retro.sh
  那么 简报含 candidates: 1
  并且 该候选行含 recurrence_hint=1

场景: 记录骨架使用下一个 FLAW 编号
  测试: olp_evo_retro_skeleton_uses_next_flaw_id
  假设 仓库记录目录含 FLAW-001.md 与 FLAW-002.md,进化黑板含两个不同候选的卡
  当 运行 olp-evo-retro.sh
  那么 简报中出现 id: FLAW-003 与 id: FLAW-004
  并且 记录目录中不存在 FLAW-003.md

场景: 游标推进后重跑零新卡
  测试: olp_evo_retro_cursor_advances_and_rerun_is_empty
  假设 已对含 3 张卡的进化黑板运行过一次 retro
  当 再次运行 olp-evo-retro.sh
  那么 stdout 含 retro: 0 new card(s)
  并且 retro.json 的 last_id 等于 3
  并且 retro 目录中的简报文件数等于 1

场景: 新增卡只处理增量
  测试: olp_evo_retro_processes_only_new_cards
  假设 已 retro 过 3 张卡,进化黑板又追加 1 张卡
  当 运行 olp-evo-retro.sh
  那么 新简报含 cards: 1

场景: dry-run 零写入
  测试: olp_evo_retro_dry_run_writes_nothing
  假设 进化黑板含新卡且状态目录不存在 retro.json
  当 以 --dry-run 运行 olp-evo-retro.sh
  那么 stdout 含 candidates:
  并且 状态目录中不存在 retro.json
  并且 状态目录中不存在 retro 目录

场景: 畸形卡被报告并跳过
  测试: olp_evo_retro_malformed_card_reported_and_skipped
  假设 进化黑板含一张缺少 identity 行的卡与一张完整的卡
  当 运行 olp-evo-retro.sh
  那么 stderr 含 malformed-card:
  并且 简报含 candidates: 1
  并且 退出码等于 0

场景: 无新卡退出 0
  测试: olp_evo_retro_no_cards_exit_zero
  假设 进化黑板不存在或不含任何卡
  当 运行 olp-evo-retro.sh
  那么 退出码等于 0
  并且 stdout 含 retro: 0 new card(s)
  并且 状态目录中不存在 retro 目录

场景: 改判与 R2 记档行形触发采集
  测试: olp_evo_harvest_override_and_r2_lines_trigger
  假设 活板含一行 > 改判(作废 #40):以本条为准 与一行 > R2 记档(#41):声称 verified 复验不符
  当 运行 olp-evo-harvest.sh
  那么 进化黑板恰新增两张卡
  并且 两张卡的 trigger 行分别为 override 与 r2_record

场景: 正文提及改判不触发
  测试: olp_evo_harvest_override_prose_mention_does_not_trigger
  假设 活板正文含 主审改判(见上) 字样但无行首定式
  当 运行 olp-evo-harvest.sh
  那么 进化黑板中以 ### EVO- 开头的行数等于 0

场景: issue 模板、skill 卡与 BOOT 就位
  测试: olp_evo_retro_docs_in_place
  假设 仓库检出
  当 读取 ISSUE-template.md、SKILL.md、OLP_OUTER_BOOT.md、OCTOLOOP_FEATURES.md
  那么 模板含七节标题
  并且 SKILL.md 含 olp-evo-retro.sh 且 description 行仍含 三模式
  并且 OLP_OUTER_BOOT.md 含 改判(作废 # 与 R2 记档(
  并且 OCTOLOOP_FEATURES.md 含 外环私有工作纸
