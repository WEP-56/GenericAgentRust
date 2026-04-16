# GenericAgentRust 迁移状态

本文档用于把 Python 参考实现 `E:\GA-rust\exampleProject` 与 Rust/Tauri 重构版 `E:\GA-rust\GenericAgentRust` 放在同一张表里，明确当前完成度、差距和下一步迁移顺序。

## 1. 参考项目的真实核心

Python 版不是“聊天框 + 几个工具”这么简单，核心能力主要在这条链上：

- `agentmain.py`：初始化 LLM、多入口运行模式、系统提示词、任务队列。
- `agent_loop.py`：多轮 tool-calling 主循环、`no_tool` 回退、上下文压缩。
- `ga.py`：工具实现、工作记忆、plan mode、长期记忆触发、turn-end 注入。
- `llmcore.py`：多后端 LLM 兼容、SSE 解析、上下文裁剪。
- `memory/`：L0/L1/L2/L3/L4 记忆与 SOP 库。
- `TMWebDriver.py` + `simphtml.py`：接入真实浏览器会话的 web automation 能力。

Rust 版如果要“完全复刻”，最终必须把上面这条链的行为也迁过去，而不只是把工具函数换成 Rust。

## 2. 当前 Rust 版状态

### 已成型

- Tauri 命令层已经具备：配置读写、LLM 请求、流式输出、工具调用。
- React 前端已经具备：聊天 UI、设置页、基础多轮 tool-calling loop。
- Rust core 已拆分：`config.rs / llm.rs / tools.rs / memory.rs / browser.rs`。
- 基础工具已有实现：`code_run / file_read / file_patch / file_write / web_scan / web_execute_js / ask_user`。

### 仍未等价

- Agent loop 仍在前端，且是简化版，未复刻 Python 版 `GenericAgentHandler` 行为。
- 工作记忆、长期记忆、plan mode、turn-end 注入均未形成完整闭环。
- 浏览器仍是独立拉起的 `headless_chrome`，不是接管用户现有浏览器会话。
- `memory/` 与 `assets/` 中大量文件已拷贝，但此前并未真正接到运行路径。

## 3. 能力对照表

| 模块 | Python 版 | Rust 版当前 | 结论 |
|---|---|---|---|
| LLM 多后端 + 流式输出 | 完整 | 已实现基础版 | 可用，但还缺上下文策略细节 |
| Agent Loop | 完整，含 `no_tool` / turn-end / plan | 前端简化 loop | 未完成 |
| code/file 工具 | 完整 | 基础版可用 | 大体可用 |
| ask_user | 完整中断流程 | 仅中断，等待恢复逻辑简化 | 部分完成 |
| working checkpoint | 完整 | 本次开始迁入 | 进行中 |
| long-term memory trigger | 完整 | 本次开始迁入入口 | 进行中 |
| SOP 自动注入 | 完整 | 本次开始迁入 | 进行中 |
| 浏览器接管真实会话 | 完整 | 未完成 | 关键缺口 |
| 多前端入口（Telegram/微信/飞书等） | 完整 | 未迁 | 暂不优先 |

## 4. 本轮已完成

本轮已经完成两个迁移包：

1. 拉齐 system prompt 构建逻辑。
2. 让 `memory/` 和 `assets/` 真正参与运行。
3. 补 `update_working_checkpoint` 与 `start_long_term_update` 两个核心工具入口。
4. 让前端 tool schema 至少覆盖这两个记忆工具。
5. 将简化版 agent loop 从前端下沉到 Rust 后端。
6. 由后端统一负责多轮 LLM 调用、工具执行、`no_tool` 回退和 turn hint 注入。

### 本轮后的状态变化

- 前端不再自己驱动多轮 tool loop，而是只负责发起请求和消费后端事件。
- 后端已经具备一个可运行的 agent runtime 雏形，后续可以在此基础上继续逼近 Python 版 `GenericAgentHandler`。
- 当前“Agent 是一个聊天 UI 里的循环逻辑”这件事已经被扭转，运行时重心开始回到 Rust 后端。
- runtime 已开始直接复用 `assets/tools_schema.json` / `tools_schema_cn.json`，不再维护一套手写工具契约。
- `code_run / web_execute_js / file_write` 已支持从回复正文代码块或 `<file_content>` 中提取实际执行内容，更接近 Python 版工具语义。
- 后端已开始维护 `<history>` 摘要和 `<key_info>` 锚点提示，为后续补齐完整 handler 铺路。
- 后端 runtime 已支持消费 `_keyinfo / _intervene / _stop` 外部干预文件，具备被外部调度和纠偏的基础能力。
- plan mode 已接入基础识别逻辑，可做 Plan Hint 提示、未验证完成声明拦截和 plan 文件剩余步骤检查。
- 已兼容 `plan_sop.md` 中的显式进入方式：`code_run + _inline_eval + handler.enter_plan_mode(...)`，并把 plan 状态持久化到 working checkpoint。
- plan mode 下的最大轮次已从普通任务的 15 轮提升为 80 轮，更接近参考实现的执行边界。

这样做的原因：

- 这是 Python 版“自治执行器”最核心的地基。
- 改动集中在 `memory.rs / tools.rs / main.rs / Chat.tsx`，边界清晰。
- 先把 prompt 和记忆链打通，再迁 handler / plan mode，风险更低。

## 5. 建议的后续迁移顺序

### Phase A

- 完成 system prompt、working checkpoint、长期记忆入口。
- 收口前后端 tool schema 差异。

### Phase B

- 把前端里的 agent loop 收回 Rust 后端。
- 迁 `no_tool` 回退、turn-end 注入、历史摘要、工作记忆更新策略。

### Phase C

- 迁 plan mode。
- 迁 `memory_management_sop` 驱动的长期记忆最小更新流程。

### Phase D

- 用 CDP/扩展桥接替代当前 `headless_chrome` 独立浏览器模型。
- 追平 `TMWebDriver + simphtml` 的真实浏览器控制能力。

## 6. 当前判断

截至本轮结束后，Rust 版整体完成度大致提升到 45% - 55%。

当前已经具备“后端驱动的基础执行器”，但距离 Python 版完整自治能力仍有明显差距，下一步重点仍然是补齐 `GenericAgentHandler` 行为而不是继续堆 UI。
