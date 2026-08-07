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

- 输出走既有本地活动通道(`push_local_activity`),与 `/ps`、未知命令警告
  等本地反馈一致:一条 Progress 活动,标题为 loops 计数,detail 为多行清单
  ——因此它进入转录并落入 scrollback,可回看、可复制 id。
- 每行格式: `<id>  <status>  <cadence>[ · 第 N 轮][ · 下次 X]  <prompt 摘要>`;
  各段缺失时省略,不虚构。prompt 摘要复用既有 `autonomy_loop_label` 的截断
  规则,保持与自主指示行一致。
- 空列表也要有明确输出(不能只有状态栏一句),提示 `/loop <提示词>` 如何创建。
- 状态栏原有的 "Loop list refreshed: N loop(s)" 保留(它是操作确认),
  本任务只补转录输出。

## 边界

### Allowed Changes
- src/store.rs
- src/app.rs
- src/app/tests.rs
- locales/en.yml
- locales/zh.yml
- specs/**

### Forbidden
- 不改变 loop 的创建/暂停/恢复/删除协议与命令解析。
- 不改变自主指示行与状态栏 chip 的既有渲染。
- 不新增 crate 依赖。

## 排除范围

- 把 `/loop list` 做成可选中的菜单(选中即执行 pause/resume/delete)——
  后续增强,本任务先解决"拿不到 id"这个功能性阻塞。

## 完成条件

场景: 列表清单渲染进转录并包含 loop id
  测试: loop_list_pushes_transcript_entry_with_ids
  假设 会话中存在两个 loop,id 分别为 loop-aaa 与 loop-bbb
  当 应用 loop 列表结果
  那么 转录中新增一条本地活动
  并且 其内容同时包含 loop-aaa 与 loop-bbb
  并且 包含各自的状态文本

场景: 清单行包含节奏与提示词摘要
  测试: loop_list_entry_shows_cadence_and_prompt
  假设 一个 self-paced 模式、提示词为 "请你完成这本书" 的活跃 loop
  当 应用 loop 列表结果
  那么 清单内容包含 self-paced 字样
  并且 包含提示词摘要文本

场景: 空列表也给出可操作提示
  测试: empty_loop_list_still_explains_how_to_create
  假设 会话中没有任何 loop
  当 应用 loop 列表结果
  那么 转录中新增一条本地活动
  并且 其内容包含 /loop 创建提示
