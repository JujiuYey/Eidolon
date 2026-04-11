# 代码分析 Agent 技术设计

## 1. 文档目标

本文档用于定义 `Eidolon` 内置代码分析 Agent 的落地方案。

本版本基于当前项目实际情况做了一个关键调整：

- 模型调用与 Agent 编排统一放在 Tauri Rust 侧，使用现有 `rig-core`
- 前端不使用 `ai` npm 包发起模型请求
- 前端安装的 `ai` / `ai-elements` 仅用于消息、工具状态、文件树等 UI 组件复用

目标能力：

- 面向本地项目做代码结构与业务逻辑分析
- 允许 Agent 读取文件、扫描目录、搜索内容、保存模块摘要
- 支持项目级记忆持久化
- 支持多轮问答与工具调用轨迹展示
- 尽量复用现有三栏布局与现有设置能力

---

## 2. 当前项目约束

结合现有仓库，设计必须遵循以下事实：

- 前端技术栈：Vue 3 + TypeScript + Pinia
- 桌面壳：Tauri 2
- AI 接入现状：Tauri Rust 侧已使用 `rig-core`
- 前端已引入 `ai-elements`，适合复用消息与工具展示组件
- 应用设置中已经存在全局 `projectPath`
- 当前路由和左侧应用菜单都比较轻量，适合新增独立 Agent 页面

因此，Agent 的职责划分应当是：

- Rust：Prompt 组装、模型请求、Tool loop、文件系统访问、记忆读写
- 前端：发起请求、接收流式事件、更新对话状态、渲染 UI

---

## 3. 总体架构

```txt
Vue Agent Page
  -> invoke(start_agent_run)
  -> listen(agent://*)

Tauri Command Layer
  -> Agent Runner
  -> rig-core Model
  -> Tool Registry
  -> Memory Store

Tool Registry
  -> scan/list/read/search project files
  -> save/load analyzed module summary

Memory Store
  -> project scoped JSON files
```

核心原则：

1. 前端不负责 Tool Call 主循环。
2. Tool 的真正执行位置在 Rust，避免前端和后端各维护一套能力。
3. 前端只维护运行时状态，项目记忆以本地 JSON 为准，不把完整历史长期塞进 `localStorage`。
4. `projectPath` 复用现有应用设置，不在 `agent` store 再维护第二份来源。

---

## 4. 技术选型

| 层级 | 技术 | 说明 |
| --- | --- | --- |
| 前端框架 | Vue 3 + TypeScript | 现有技术栈 |
| 状态管理 | Pinia | 现有技术栈 |
| UI 组件 | `ai-elements` + 当前 UI 组件库 | 用于消息、工具状态、文件树等展示 |
| Agent 编排 | `rig-core` | 统一放在 Tauri Rust 侧 |
| 后端 | Tauri 2 + Rust | 现有技术栈 |
| 文件遍历 | `walkdir` + `globset` | 比单纯 `glob` 更适合递归遍历和忽略目录 |
| 文本搜索 | `regex` | 搜索文件内容 |
| 记忆存储 | JSON 文件 | 使用 Tauri 应用数据目录 |
| 路径哈希 | `sha2` + `hex` | 按项目路径生成稳定目录名 |

说明：

- 不建议继续坚持纯 `glob` 方案。代码库扫描场景里，`walkdir` 负责递归遍历，`globset` 负责扩展名过滤，会更稳定。
- 不需要 `dirs` crate。Tauri 已经提供 `app.path()`。

---

## 5. 前端设计

### 5.1 页面与路由

建议新增独立页面：

```txt
src/views/agent/
├── index.vue
├── components/
│   ├── AgentLayout.vue
│   ├── ProjectFileTree.vue
│   ├── ConversationList.vue
│   ├── ChatPanel.vue
│   ├── MessageList.vue
│   ├── MessageComposer.vue
│   ├── ToolTimeline.vue
│   └── ModuleSummaryPanel.vue
└── composables/
    └── useAgentEvents.ts
```

对应改动：

- `src/router/index.ts` 新增 `/agent`
- `src/layout/app-sidebar/menu/index.vue` 新增“代码分析”菜单
- 可以把 `/index` 重定向到 `/agent`，也可以保留旧页面作为 demo

### 5.2 UI 布局

可以复用现有 mail 页面三栏思路，但不直接复制其业务结构。

建议布局：

- 左栏：项目文件树 + 文件过滤
- 中栏：对话列表
- 右栏：当前对话、工具轨迹、模块摘要

其中右栏内部建议再拆成上下区：

- 上半区：消息流与工具过程
- 下半区：输入框与当前模块摘要

### 5.3 Agent Store

文件建议：

```txt
src/stores/agent.ts
src/types/agent/index.ts
src/services/agent.ts
```

推荐类型：

```ts
export interface AgentSettings {
  maxHistoryMessages: number
  maxToolRounds: number
  fileExtensions: string
  ignoreDirs: string
  maxFileContentLength: number
}

export interface AgentToolTrace {
  id: string
  name: string
  argsText: string
  resultText?: string
  status: 'pending' | 'running' | 'success' | 'error'
}

export interface AnalyzedModule {
  path: string
  summary: string
  updatedAt: string
}

export interface AgentMessage {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  createdAt: number
  status?: 'streaming' | 'done' | 'error'
  toolTraces?: AgentToolTrace[]
}

export interface AgentConversation {
  id: string
  title: string
  messages: AgentMessage[]
  createdAt: number
  updatedAt: number
}
```

Store 设计重点：

- `projectPath` 不放在 `agent` store，直接从 `useAppStore().settings.projectPath` 读取
- `files`、`selectedFilePath`、`activeRunId` 属于运行时状态
- `conversations`、`analyzedModules` 的权威来源是 Rust memory JSON
- `Pinia persist` 只建议保留轻量 UI 状态

推荐状态：

```ts
const files = ref<string[]>([])
const selectedFilePath = ref<string | null>(null)
const conversations = ref<AgentConversation[]>([])
const currentConversationId = ref<string | null>(null)
const analyzedModules = ref<Record<string, AnalyzedModule>>({})
const isAnalyzing = ref(false)
const activeRunId = ref<string | null>(null)
const settings = ref<AgentSettings>({ ...defaultAgentSettings })
```

推荐 action：

- `initializeAgentProject()`
- `loadMemory()`
- `createConversation()`
- `selectConversation(id)`
- `sendMessage(content)`
- `cancelRun()`
- `clearMemory()`
- `refreshFiles()`
- `openFile(path)`

### 5.4 前端服务层

前端新增统一服务层，不直接在组件里写 `invoke`：

```txt
src/services/agent.ts
```

职责：

- 调用 `start_agent_run`
- 调用 `cancel_agent_run`
- 调用 `load_agent_memory`
- 调用 `clear_agent_memory`
- 调用 `scan_project_files`
- 调用 `read_project_file`
- 订阅 `agent://*` 事件

### 5.5 前端事件处理

推荐使用 Tauri event 机制做运行中状态同步。

事件建议：

- `agent://run-started`
- `agent://message-delta`
- `agent://tool-started`
- `agent://tool-finished`
- `agent://analysis-saved`
- `agent://run-finished`
- `agent://run-error`

说明：

- 如果当前 `rig-core` 接入链路不方便做 token 级 streaming，可以先只发步骤级事件
- UI 协议仍然保留 `message-delta`，后续可平滑升级到流式输出

### 5.6 交互流程

```txt
进入 Agent 页面
  -> 检查 projectPath 和 AI 配置
  -> 扫描项目文件
  -> 加载项目记忆

用户发送问题
  -> 前端调用 start_agent_run
  -> Rust 启动 Agent Runner
  -> Runner 调模型并按需执行工具
  -> Tauri 向前端发事件
  -> 前端更新消息与工具状态
  -> Runner 保存 memory
  -> 前端刷新 analyzedModules / conversations
```

---

## 6. Rust 后端设计

### 6.1 Cargo 依赖

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri = { version = "2.10.3", features = [] }
rig-core = "0.33.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs"] }
walkdir = "2"
globset = "0.4"
regex = "1"
sha2 = "0.10"
hex = "0.4"
log = "0.4"
```

### 6.2 模块结构

```txt
src-tauri/src/
├── commands/
│   ├── mod.rs
│   └── agent.rs
├── models/
│   ├── mod.rs
│   └── agent.rs
├── services/
│   ├── mod.rs
│   ├── agent_runner.rs
│   ├── agent_tools.rs
│   └── memory_store.rs
└── utils/
    ├── mod.rs
    └── path.rs
```

说明：

- `commands/agent.rs` 只做 Tauri command 边界
- `services/agent_runner.rs` 负责 rig-core 调度和 Tool loop
- `services/agent_tools.rs` 负责文件扫描、读取、搜索、保存摘要
- `services/memory_store.rs` 负责 JSON 持久化

### 6.3 数据结构

建议统一返回结构化类型，不要用 `Result<String, String>` 包住 JSON 字符串。

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentSettings {
    pub max_history_messages: usize,
    pub max_tool_rounds: usize,
    pub file_extensions: String,
    pub ignore_dirs: String,
    pub max_file_content_length: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub file_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LineMatch {
    pub line_number: usize,
    pub content: String,
    pub is_target: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub file: String,
    pub matches: Vec<LineMatch>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleSummary {
    pub path: String,
    pub summary: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentConversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<AgentMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentMemoryData {
    pub conversations: Vec<AgentConversation>,
    pub analyzed_modules: std::collections::HashMap<String, ModuleSummary>,
}
```

关键约束：

- `analyzed_modules` 的 key 必须是项目内相对路径，不要用模块名
- Rust 与前端的数据结构尽量一一对应，减少转换层

### 6.4 Tauri Commands

对外命令建议分成两类。

第一类：给前端页面直接调用。

```rust
#[tauri::command]
async fn scan_project_files(
    project_path: String,
    settings: AgentSettings,
) -> Result<Vec<FileEntry>, String>;

#[tauri::command]
async fn read_project_file(
    project_path: String,
    file_path: String,
    start_line: Option<usize>,
    end_line: Option<usize>,
    max_chars: Option<usize>,
) -> Result<String, String>;

#[tauri::command]
async fn search_project_files(
    project_path: String,
    keyword: String,
    file_pattern: Option<String>,
    context_lines: Option<usize>,
    settings: AgentSettings,
) -> Result<Vec<SearchResult>, String>;

#[tauri::command]
async fn load_agent_memory(project_path: String) -> Result<AgentMemoryData, String>;

#[tauri::command]
async fn clear_agent_memory(project_path: String) -> Result<(), String>;

#[tauri::command]
async fn start_agent_run(
    app: tauri::AppHandle,
    request: StartAgentRunRequest,
) -> Result<StartAgentRunResponse, String>;

#[tauri::command]
async fn cancel_agent_run(run_id: String) -> Result<(), String>;
```

第二类：只给 Agent 内部调用，不暴露给前端。

- `save_analysis_internal`
- `list_directory_internal`
- `scan_files_internal`
- `read_file_internal`
- `search_files_internal`
- `save_memory_internal`

这样做的目的：

- UI 文件树、文件预览可以复用同一套文件服务
- Agent Tool 和页面接口共享底层实现
- Tool 能力不需要通过前端再绕一次

### 6.5 运行时事件模型

推荐事件载荷：

```rust
pub struct AgentEventPayload {
    pub run_id: String,
    pub conversation_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
}
```

事件类型建议：

- `run_started`
- `assistant_delta`
- `tool_started`
- `tool_finished`
- `analysis_saved`
- `run_finished`
- `run_error`

### 6.6 Memory 存储

记忆不再放在固定 `~/.Eidolon/memory/`，而是遵循 Tauri 目录体系。

推荐路径：

```txt
{app_data_dir}/memory/<project_hash>/memory.json
```

路径函数：

```rust
fn get_memory_dir(app: &tauri::AppHandle, project_path: &str) -> PathBuf
```

实现建议：

1. 对 `project_path` 做 SHA256
2. 使用 hash 前 8 到 12 位作为目录名
3. `memory.json` 存 conversations 和 analyzed_modules

说明：

- 这样和现有 AI 配置的 Tauri 路径风格一致
- 避免路径里出现平台相关非法字符

### 6.7 路径安全

所有文件工具都必须先做路径校验：

1. 将 `project_path` 和目标路径都 `canonicalize`
2. 确认目标路径仍在项目根目录内
3. 对前端只返回相对路径
4. 拒绝访问二进制大文件、软链接逃逸、项目外路径

---

## 7. Agent 编排设计

### 7.1 Prompt 生成位置

系统 Prompt 放在 Rust 中构造，而不是前端。

原因：

- Prompt 与 Tool 集合强相关，应该和 Agent Runner 放在一起
- 避免前端与后端各维护一份规则
- 后续如果要引入不同模型或不同模式，Rust 更容易统一治理

### 7.2 Prompt 内容

建议结构：

```txt
你是前端代码业务逻辑解读专家。

当前项目路径: <project_path>
当前已知文件: <top files>
已分析模块摘要:
- <relative path>: <summary>

分析原则:
1. 优先解释业务逻辑、模块职责和调用关系
2. 不要泛泛解释语法
3. 必要时主动读取文件或搜索代码
4. 大文件优先按行范围读取
5. 分析完成后保存模块摘要
6. 如果信息不足，明确说明还需要哪些文件
```

### 7.3 工具集合

推荐 Tool：

- `scan_files`
- `list_directory`
- `read_file`
- `search_files`
- `save_analysis`

建议约束：

- `read_file` 必须支持按行范围读取
- `read_file` 必须限制字符数
- `search_files` 必须限制返回文件数和匹配数
- `save_analysis` 只允许写入当前项目 memory

### 7.4 Tool Loop

MVP 逻辑：

```txt
load memory
append user message
for round in 1..=max_tool_rounds
  call model with current messages + tools
  if assistant returned final text
    persist memory
    emit run_finished
    break
  if assistant requested tools
    execute tools sequentially
    append tool results to conversation context
    continue
if exceeded round limit
  emit run_error("请缩小问题范围后重试")
```

说明：

- 先做串行工具执行，逻辑更稳定
- 后续如果确有性能瓶颈，再评估并行工具执行

### 7.5 历史消息裁剪

`maxHistoryMessages` 应在 Rust 侧生效。

裁剪建议：

- 保留最近 N 条消息
- `system prompt` 永远保留
- `analyzed_modules` 作为外部上下文注入，不依赖完整历史

---

## 8. 文件工具设计

### 8.1 扫描文件

扫描策略：

- 递归遍历项目目录
- 目录忽略来自 `ignoreDirs`
- 文件扩展名限制来自 `fileExtensions`
- 返回相对路径列表，供文件树和 Prompt 使用

建议限制：

- 扫描结果只保留文本代码文件
- 首次扫描只返回前端展示需要的基本字段
- 大仓库可以增加文件数上限和懒加载目录展开

### 8.2 读取文件

要求：

- 支持 `start_line` / `end_line`
- 支持 `max_chars`
- 自动裁切超长内容并在末尾附带截断提示

### 8.3 搜索文件

要求：

- 支持关键字搜索
- 支持文件模式过滤
- 支持上下文行
- 返回结构化结果，不返回原始拼接文本

---

## 9. 错误处理

| 场景 | 处理方式 |
| --- | --- |
| 未设置项目路径 | 前端阻止发送，并提示去系统设置选择项目文件夹 |
| 未配置 AI 服务 | 前端阻止发送，并提示去 AI 设置配置模型 |
| 项目路径不存在 | Rust 返回明确错误，前端提示重新选择 |
| 文件读取失败 | 返回具体原因，如文件不存在、越权访问、编码失败 |
| 搜索无结果 | 返回空数组，由前端展示“未找到匹配内容” |
| 工具调用超时 | 结束当前 run，并写入错误消息 |
| 达到最大轮次 | 返回“请缩小问题范围后重试” |
| 模型服务不可用 | 返回供应商错误信息，并附带基础诊断提示 |
| memory 文件损坏 | 记录日志并回退到空 memory，不阻塞页面加载 |

---

## 10. 配置项

建议保留原始配置，但生效位置改为 Rust Runner。

| 配置项 | 默认值 | 说明 |
| --- | --- | --- |
| `maxHistoryMessages` | `30` | 参与模型上下文的历史消息条数 |
| `maxToolRounds` | `30` | 单次交互最大工具轮次 |
| `fileExtensions` | `.ts,.tsx,.js,.jsx,.vue` | 允许扫描的文件扩展名 |
| `ignoreDirs` | `node_modules,.git,dist` | 忽略目录 |
| `maxFileContentLength` | `15000` | 单次读取最大字符数 |

补充建议：

- 这些配置保存在前端 `agent` store 即可
- 每次启动 run 时随请求传给 Rust

---

## 11. 与当前项目的集成点

前端改动点：

- 新增 `src/views/agent/index.vue`
- 新增 `src/stores/agent.ts`
- 新增 `src/services/agent.ts`
- 新增 `src/types/agent/index.ts`
- `src/router/index.ts` 增加 Agent 路由
- `src/layout/app-sidebar/menu/index.vue` 增加 Agent 菜单

后端改动点：

- 新增 `src-tauri/src/commands/agent.rs`
- 新增 `src-tauri/src/models/agent.rs`
- 新增 `src-tauri/src/services/agent_runner.rs`
- 新增 `src-tauri/src/services/agent_tools.rs`
- 新增 `src-tauri/src/services/memory_store.rs`
- 更新 `src-tauri/src/commands/mod.rs`
- 更新 `src-tauri/src/models/mod.rs`
- 更新 `src-tauri/src/lib.rs`
- 扩展 `src-tauri/src/utils/path.rs`

现有能力复用：

- 项目路径直接复用 `useAppStore().settings.projectPath`
- AI 配置继续复用当前 AI 设置 Store
- 聊天 UI 可复用 `ai-elements` 组件
- 三栏可复用现有 `ResizablePanelGroup`

---

## 12. 实施顺序

### Phase 1：后端基础能力

1. 建立 `agent` 数据模型
2. 实现 memory store
3. 实现文件扫描、读取、搜索服务
4. 注册基础 Tauri commands

### Phase 2：Agent Runner

1. 封装 rig-core client 创建逻辑
2. 实现 Prompt 构造
3. 实现 Tool 注册与 Tool loop
4. 加入运行事件发射

### Phase 3：前端页面

1. 创建 `/agent` 页面
2. 接入三栏布局
3. 接入文件树、对话列表、消息区
4. 监听 Tauri 事件并更新 store

### Phase 4：体验增强

1. 工具轨迹可视化
2. 模块摘要面板
3. 取消运行
4. 空状态与错误提示优化

---

## 13. 最终结论

这套方案可以开工，但必须建立在以下前提上：

- Agent 的模型调用必须继续放在 Tauri Rust 侧，用 `rig-core`
- 前端不承担 Tool loop 和 Prompt 主逻辑
- 文件工具和记忆系统统一由 Rust 托管
- 前端只做状态管理、事件消费和界面呈现

相较于最初方案，本版的关键修正是：

1. 把 AI 编排重心从前端迁回 Rust
2. 把文件工具从“前端调工具”改成“Rust Agent 内部直接调工具”
3. 把 memory 路径改成遵循 Tauri 应用目录
4. 把 `projectPath` 的来源统一到现有应用设置
5. 把返回值从字符串 JSON 改成结构化类型

如果按本文档实施，技术路径和当前项目现状是一致的，后续也更容易维护。
