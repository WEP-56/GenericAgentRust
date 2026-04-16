# GenericAgentRust

`GenericAgentRust` 是对 Python 版 GenericAgent 的 `Rust + Tauri + React` 重构实验。

目标不是单纯把界面换成桌面端，而是逐步把原项目的核心能力迁过来：

- 更轻的本地桌面形态
- 更清晰的前后端边界
- 保留 GenericAgent 的多轮 tool-calling、工作记忆、SOP 注入和计划态执行能力

当前仓库已经不是 Tauri 模板，但也还没有达到 Python 版的完整功能等价。它现在更准确的定位是：

> 一个正在持续迁移中的、后端驱动的 GenericAgent 桌面版。

## 当前状态

### 已可用

- Tauri 桌面壳 + React 聊天界面
- LLM 配置管理
- 流式输出
- 后端驱动的多轮 agent loop
- 基础工具：
  - `code_run`
  - `file_read`
  - `file_patch`
  - `file_write`
  - `web_scan`
  - `web_execute_js`
  - `ask_user`
  - `update_working_checkpoint`
  - `start_long_term_update`
- `memory/` 与 `assets/` 已接入运行时
- 基础 plan mode 控制：
  - working checkpoint 持久化
  - `plan_sop` 的显式进入方式
  - plan hint
  - 基础完成拦截
  - `_keyinfo / _intervene / _stop` 外部干预文件

### 仍在迁移

- Python 版 `GenericAgentHandler` 的完整行为
- 验证子代理闭环
- 更完整的 turn-end callback / done hooks
- 浏览器接管真实用户会话
- Python 版里更成熟的 SOP / 记忆更新策略

如果你想看更细的迁移清单，直接看：

- [MIGRATION_STATUS.md](./MIGRATION_STATUS.md)
- [TODO_LIST.md](./TODO_LIST.md)

## 技术栈

- `Rust`
- `Tauri 2`
- `React 19`
- `TypeScript`
- `Vite`

## 目录结构

```text
GenericAgentRust/
├─ src/                    # React 前端
├─ src-tauri/
│  ├─ src/                 # Tauri 命令层
│  ├─ core/src/            # Agent runtime / memory / tools / llm / browser
│  └─ assets/              # sys_prompt / tools_schema / insight 模板等
├─ MIGRATION_STATUS.md     # 迁移现状说明
├─ TODO_LIST.md            # 迁移待办
└─ README.md
```

## 快速启动

### 1. 环境要求

- Windows
- Node.js 20+
- Rust stable
- WebView2

建议先确认：

```bash
npm -v
node -v
rustc -V
cargo -V
```

### 2. 安装依赖

```bash
cd E:\GA-rust\GenericAgentRust
npm install
```

### 3. 启动开发环境

```bash
npm run tauri dev
```

### 4. 构建前端

```bash
npm run build
```

### 5. 检查 Rust 后端

```bash
cd src-tauri
cargo check
```

## 使用方式

首次启动后，先到设置页配置：

- `LLM Provider`
- `Base URL`
- `API Key`
- `Model`
- `Workspace Directory`
- `Memory Directory`

然后回到聊天页发送任务。

当前推荐把它当作：

- 本地代码/文件代理
- 多轮 tool-calling 实验环境
- GenericAgent Rust 重构版的迁移验证平台

## 当前实现重点

这个仓库当前最核心的部分在：

- [src-tauri/core/src/agent.rs](./src-tauri/core/src/agent.rs)
- [src-tauri/core/src/tools.rs](./src-tauri/core/src/tools.rs)
- [src-tauri/core/src/memory.rs](./src-tauri/core/src/memory.rs)
- [src-tauri/src/main.rs](./src-tauri/src/main.rs)

其中：

- `agent.rs` 负责后端 runtime、多轮执行、plan mode 基础控制、外部干预
- `tools.rs` 负责工具执行
- `memory.rs` 负责 system prompt、working checkpoint、SOP 注入
- `main.rs` 负责把 Tauri 命令层接到 runtime

## 与 Python 版的关系

这个仓库是基于本地参考项目的重构版。

当前设计思路不是重新发明一套 Agent，而是尽量复用原项目已经验证过的资产和协议：

- `tools_schema.json / tools_schema_cn.json`
- `sys_prompt.txt`
- `plan_sop.md`
- `memory_management_sop.md`
- `global_mem_insight_template.txt`

换句话说，这个仓库优先做的是：

> 把 GenericAgent 已有能力迁到 Rust 运行时，而不是在 Rust 里另造一个新 Agent。

## 已验证命令

当前仓库至少已经反复验证过：

```bash
npm run build
cd src-tauri && cargo check
```

## 已知限制

- 当前浏览器能力仍基于独立 `headless_chrome`，不是接管用户已有浏览器会话
- 计划态和验证态还未完全达到 Python 版水平
- 目前以 Windows 单机桌面端为主
- 前端打包体积还偏大

## 路线图

下一阶段优先项：

1. 完成 `GenericAgentHandler` 更完整的行为迁移
2. 补齐验证子代理流程
3. 继续完善长期记忆闭环
4. 替换当前浏览器模型，向 CDP / 会话接管靠拢

## License

当前仓库未单独声明新 License 时，请按你的上游项目和你自己的发布策略处理。
