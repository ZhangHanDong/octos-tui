spec: task
name: "Loops 菜单每 loop 一行 + 动作子菜单"
inherits: project
tags: [tui, autonomy, loop, menu, ux]
estimate: 0.25d
---

## 意图

实测(2026-08-09,用户截图):上游 loops 菜单按"每个 loop × 每个动作 = 一行"
渲染,一个 loop 出三行、每行都以相同的 id 和状态开头,动作词藏在行中间——
用户直接误读为"三个同名 loop"。三个 loop 就是九行,无法扫读。

改为两级:清单层**每个 loop 恰好一行**(区分信息前置:图标、id、状态、节奏、
提示词摘要),回车进入该 loop 的**动作子菜单**(pause/resume/fire-now/delete)。

## 已定决策

- 清单行动作为 `LocalAction::OpenLoopActions(loop_id)`:记录目标到
  `AppState.loop_actions_target`,压栈打开 `MENU_LOOP_ACTIONS`。Esc 沿既有
  菜单栈语义逐层返回。
- 子菜单头部为**只读详情行**(完整状态·节奏 + 提示词摘要,`non_selectable`),
  光标自动跳过;动作行沿用原 `RunSlashCommand("/loop <verb> <id>")` 派发路径,
  保持能力门控与关栈语义不变。
- 动词按状态裁剪:active → pause/fire-now/delete;paused → resume/fire-now/
  delete;其他 → delete。paused 也可 fire-now(与服务端 control_loop 一致)。
- 目标 loop 在子菜单打开期间消失(他处删除/过期)时,子菜单显示明确的
  "已不存在" 说明而非报错动词;`loop_actions_target` 不主动清理——它仅在
  `MENU_LOOP_ACTIONS` 在栈上时被读取,下次打开前必被覆写。
- 空清单行为不变(仍为可用菜单 + 创建提示行,见 task-loop-list-transcript)。

## 边界

### Allowed Changes
- src/menu/providers.rs
- src/menu/types.rs
- src/menu/registry.rs
- src/model.rs
- src/store.rs
- locales/en.yml
- locales/zh.yml
- specs/**

### Forbidden
- 不改变 `/loop <verb>` 斜杠命令的解析与派发协议。
- 不改变 loops 菜单的打开时机(用户显式 `/loop list`,见 task-loop-list-transcript)。
- 不新增 crate 依赖。

## 排除范围

- 清单行内显示下次触发倒计时(菜单构建无时钟上下文,自主指示行已提供)。
- 跨会话全局菜单(菜单数据源仍为活跃会话镜像)。

## 完成条件

场景: 清单层每个 loop 恰好一行
  测试: loops_menu_one_row_per_loop_opens_action_submenu
  假设 存在一个 active 与一个 paused loop
  当 构建 loops 菜单
  那么 恰好有两行 loop 行
  并且 每行动作为打开对应 loop 的动作子菜单
  并且 行文本包含 id 与状态

场景: 子菜单按目标 loop 提供动词
  测试: loop_actions_menu_offers_verbs_for_the_target
  假设 目标为一个 active loop
  当 构建动作子菜单
  那么 标题包含该 loop id
  并且 pause 行派发针对该 id 的斜杠命令
  并且 至少两行只读详情行

场景: paused loop 提供 resume 而非 pause
  测试: loop_actions_menu_for_paused_loop_offers_resume
  假设 目标为一个 paused loop
  当 构建动作子菜单
  那么 存在 resume 行
  并且 不存在 pause 行

场景: 目标消失时子菜单明确说明
  测试: loop_actions_menu_says_gone_when_target_vanished
  假设 目标 id 不在当前 loop 镜像中
  当 构建动作子菜单
  那么 返回不可用说明而非动词行

场景: 激活清单行打开子菜单并记录目标
  测试: activating_loop_row_opens_action_submenu
  假设 loops 菜单中选中某个 loop 行
  当 激活该行
  那么 `loop_actions_target` 为该 loop id
  并且 活跃菜单为 `MENU_LOOP_ACTIONS`
