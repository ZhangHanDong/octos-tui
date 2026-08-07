spec: task
name: "/loop list 输出清单与 loop id"
inherits: project
tags: [tui, autonomy, loop, ux]
estimate: 0.25d
---

## 意图

实测(2026-08-03):`/loop list` 只把结果写进内存镜像并在状态栏显示一句
"Loop list refreshed: N loop(s)",**loop id 从未渲染到任何界面**。而
`/loop pause|resume|delete|fire-now` 四个命令都要求 id 参数,用户因此无从获取
——这几个命令在 UI 上实际不可用。本任务让 `/loop list` 把清单渲染进转录:
每行给出 id、状态、节奏、下次触发/轮次、提示词摘要。

## 已定决策

- 输出仍存入有界的本地 activity 状态,但使用显式 `ActivityKind::Report`；
  transcript renderer 在 agent-task grouping、settled collapse 与 Tool preview
  之前独立渲染 Report。Report 标题和 `detail` 正文均进入转录,正文按原始换行
  完整输出,窄终端只换行、不截断,且不受 `expanded_tool_outputs` 影响。
- Report 不计入 agent action 数,不显示为 `Agent task completed`,不使用 Tool
  的 `output_preview` 或 Ctrl+O 展开语义；即使当前 turn 已结束、空会话尚无消息、
  或根本没有 active session,最近的 `/loop list` Report 仍在 transcript 可见并可复制。
- 每行格式: `<id>  <status>  <cadence>[ · 第 N 轮][ · 下次 X]  <prompt 摘要>`;
  各段缺失时省略,不虚构。prompt 摘要复用既有 `autonomy_loop_label` 的截断
  规则,保持与自主指示行一致。
- 同一时刻 transcript 只保留**最新一份** loop 清单 Report:每次 `/loop list`
  在写入前移除既有同 title 的 Report(含其他会话/全局的旧份)。清单是"当前
  状态快照"而非日志,堆积多份旧快照只会误导;这也与"最近的 Report 可见"
  的措辞一致。
- 空列表也要有明确输出(不能只有状态栏一句),提示 `/loop <提示词>` 如何创建。
- 状态栏原有的 "Loop list refreshed: N loop(s)" 保留(它是操作确认),
  本任务只补转录输出。

## 边界

### Allowed Changes
- src/store.rs
- src/app.rs
- src/app/transcript_build.rs
- src/model.rs
- src/app/tests.rs
- locales/en.yml
- locales/zh.yml
- specs/**

### Forbidden
- 不改变 loop 的创建/暂停/恢复/删除协议与命令解析。
- 不改变自主指示行与状态栏 chip 的既有渲染。
- 不新增 crate 依赖。

## 排除范围

- (2026-08-08 更新)可选中的 loops 菜单已由上游 main 实现。融合语义:
  **限定查询**开菜单(可操作)+ Report 落转录(可复制);**全局查询**只出
  Report——菜单读的是活跃会话镜像,无会话时它只会以"不可用"抢占状态栏。
  菜单本身的行为不属于本合约,由上游测试钉住。

## 完成条件

场景: 列表清单渲染进转录并包含 loop id
  测试: loop_list_pushes_transcript_entry_with_ids
  假设 会话中存在两个 loop,id 分别为 loop-aaa 与 loop-bbb
  当 应用 loop 列表结果
  那么 转录中新增一条本地活动
  并且 其内容同时包含 loop-aaa 与 loop-bbb
  并且 包含各自的状态文本
  并且 两个 loop 分别占据报告正文行
  并且 不包含 Agent task completed 分组标题

场景: 清单行包含节奏与提示词摘要
  测试: loop_list_entry_shows_cadence_and_prompt
  假设 一个 self-paced 模式、提示词为 "请你完成这本书" 的活跃 loop
  当 应用 loop 列表结果
  那么 清单内容包含 self-paced 字样
  并且 包含提示词摘要文本

场景: 窄终端保持报告内容可见
  测试: loop_list_report_wraps_without_hiding_ids
  假设 默认未开启 `expanded_tool_outputs` 且终端宽度为 52 列
  当 应用包含两个 loop 的列表结果并渲染 transcript
  那么 两个 loop id 均出现在真实终端 buffer
  并且 Report 不显示 Tool preview 的隐藏行提示

场景: 重复执行只保留最新一份报告
  测试: repeated_loop_list_keeps_only_latest_report
  假设 已执行过一次 /loop list,又以更新后的结果再执行一次
  当 渲染 transcript
  那么 activity 中只有一个 Report 项
  并且 屏幕上该 loop id 只出现一行
  并且 内容反映最新一次结果

场景: 空列表也给出可操作提示
  测试: empty_loop_list_still_explains_how_to_create
  假设 会话中没有任何 loop
  当 应用 loop 列表结果
  那么 转录中新增一条本地活动
  并且 其内容包含 /loop 创建提示

场景: 没有 active session 也显示全局清单
  测试: loop_list_report_renders_without_active_session
  假设 TUI 当前没有任何 session
  当 应用全局 loop 列表结果
  那么 真实终端 buffer 包含返回的 loop id
  并且 不显示 Agent task completed 分组标题
