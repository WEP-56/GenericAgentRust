# GenericAgentRust 迁移功能列表与进度

本文档用于追踪从 Python 版 GenericAgent 到 Rust (Tauri) 版的完整功能迁移进度。我们将按模块逐步完成，确保每个模块的功能健壮且代码优雅。

## 一、核心工具链完善 (Core Tools)
- [x] **1.1 完善 `code_run`**：增加超时控制 (Timeout)、输出防截断限制，支持长时间运行脚本的强制终止。
- [x] **1.2 完善文件操作**：
  - [x] 新增 `file_write` 工具（支持覆盖、追加等模式）。
  - [x] 在 `file_patch` 和 `file_write` 中实现 `{{file:path:startLine:endLine}}` 的自动展开引用逻辑。
- [ ] **1.3 `ask_user` 工具**：完善与前端的交互，支持中断当前 Agent 流程并等待用户输入。

## 二、流式输出与核心循环 (Streaming & Agent Loop)
- [x] **2.1 LLM 流式输出 (SSE)**：将后端的请求从阻塞改为 Stream，通过 Tauri Events 实时推送到 React 前端打字机效果显示。
- [x] **2.2 完善 Agent Loop**：
  - [x] 支持多轮工具连续调用 (Max Turns 限制)。
  - [x] 上下文长度控制与内容压缩（长代码块折叠，避免 Token 溢出）。
- [x] **2.3 Agent Loop 后端下沉（基础版）**：将多轮 LLM 调用、工具执行、`no_tool` 回退和 turn hint 注入迁移到 Rust 后端事件流中。
- [x] **2.4 Handler 基础行为对齐**：复用 `tools_schema*.json`，补齐 `<history>/<key_info>` 锚点注入、turn summary 回填，以及正文代码块 / `<file_content>` 提取。
- [x] **2.5 Handler 控制面基础**：支持 `_keyinfo / _intervene / _stop` 外部干预注入，并接入 plan mode 的基础识别、Plan Hint 与完成拦截。
- [x] **2.6 显式 Plan 进入机制**：兼容 `plan_sop.md` 中 `code_run({'_inline_eval': True, 'script': 'handler.enter_plan_mode(...)'})` 的秘密入口，并持久化 plan 状态。
- [ ] **2.7 完整 Handler 行为**：继续补齐 Python 版 `GenericAgentHandler` 的 done hooks、验证子代理流程和更细的 turn-end callback 细节。

## 三、记忆与 SOP 系统 (Memory & SOPs)
- [x] **3.1 长期记忆管理（基础接线）**：实现 `global_mem.txt`、`global_mem_insight.txt` 的本地读写与初始化模板落盘。
- [x] **3.2 SOP 注入（基础接线）**：在 System Prompt 中自动扫描并注入 `memory/` 目录下的 Markdown SOP 规范文档。
- [x] **3.3 工作记忆入口**：新增 `update_working_checkpoint`，把工作记忆持久化并在每轮 System Prompt 注入。
- [x] **3.4 长期记忆入口**：新增 `start_long_term_update`，可读取 `memory_management_sop.md` 触发后续记忆沉淀。
- [ ] **3.5 记忆闭环**：将工作记忆、turn-end 注入、长期记忆最小化更新策略并入统一的后端 Agent Handler。

## 四、浏览器自动化 (Web Automation)
- [ ] **4.1 浏览器控制桥接**：使用 Rust 实现基于 CDP (Chrome DevTools Protocol) 的浏览器连接（替代原版的 `TMWebDriver`）。
- [ ] **4.2 `web_scan`**：提取当前页面的精简 HTML (DOM 净化)，过滤无用标签。
- [ ] **4.3 `web_execute_js`**：在目标页面上下文中注入并执行 JavaScript。

## 五、聊天前端进阶功能 (Frontend Advanced)
- [ ] **5.1 Markdown 渲染增强**：支持代码高亮 (Syntax Highlighting) 和表格渲染。
- [ ] **5.2 状态提示**：工具运行时的 Loading 动画、消耗时间与折叠面板。
- [ ] **5.3 错误边界与重试 UI**：在前端展示底层网络或解析错误，并提供一键重试。

---

*注：当前阶段专注于 Windows 平台的单机应用适配，不涉及多平台聊天软件机器人（如微信、飞书等）的接入。*
