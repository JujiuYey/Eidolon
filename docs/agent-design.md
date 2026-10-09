# Agent（智能体）功能设计归档

> 状态：**已下架（2026-10-09）**。移除方案为「前端整体下架、后端保留」。
> 完整可运行代码保存在 `archive/agent-feature` 分支（基于 `516bb7b`）；本文档保证重做时不需要翻旧代码也能还原设计。

---

## 1. 功能定位

Chat-driven agent workspace：用户创建 **Agent Profile**（绑定模型服务 + 系统提示词 + 工作目录 + MCP/工具选择），从 Profile 发起**会话**与模型多轮对话。会话创建时对 Profile 拍**快照**，后续 Profile 修改不影响已开始的会话。

## 2. 架构与数据流

```
View (views/agent, views/workspace)
  → Store (stores/agent-workspace.ts, setup-style)
    → Service (services/agent-conversation.ts / agent-profile.ts / agent-profile-storage.ts)
      → Tauri Command (commands/agent_conversation.rs / agent_profile.rs)
        → Repository (db/repositories/agent_conversation.rs / agent_profile.rs)
          → LocalJsonStore（应用数据目录下的 JSON 文件，非 SQLite）
```

- 参数走 Tauri snake_case，前端 `toCamelCase*` mapper 转换（`services/agent-conversation.ts` 内联）。
- 所有 agent 数据存 **JSON 文件而非 SQLite**（`db/local_store.rs::LocalJsonStore`，pretty JSON，整文件读写 + 内存缓存）：

| 文件 | 内容 |
|---|---|
| `agent_profiles.json` | 全部 Agent Profile |
| `agent_conversations.json` | 全部会话（含快照字段） |
| `agent_conversation_messages.json` | 全部消息（按 conversation_id 关联） |

- 遗留迁移：Profile 最初存 localStorage（key `eidolon.agent_profiles`），`services/agent-profile-storage.ts` 负责「读取旧数据 → upsert 进 Tauri → 写 `eidolon.agent_profiles.migrated_to_tauri` 标记」的一次性迁移。

## 3. 数据模型

### AgentProfile（`types/agent/index.ts`）

```ts
interface AgentProfile {
  id: string;
  name: string;
  description: string;
  providerId: string;      // 对应 provider_settings 里的一条（应用设置页配置）
  modelId: string;
  temperature: string;     // 字符串存储，发送时 parse
  maxTokens: string;
  systemPrompt: string;
  workDirectory: string;   // 工作目录，文件树/检索的根
  enabledMcpServiceIds: string[];  // ⚠️ 仅存储，聊天后端未消费（见 §7）
  enabledToolKeys: string[];       // ⚠️ 同上
  createdAt: number;
  updatedAt: number;
}
```

### AgentWorkspaceConversation（会话 = Profile 快照）

```ts
interface AgentWorkspaceConversation {
  id: string;
  agentProfileId: string;
  title: string;
  snapshotVersion: number;
  createdFromProfileUpdatedAt: number;  // 快照时 Profile 的 updatedAt，可用于判断“Profile 已变化”
  snapshotAgentName: string;
  snapshotProviderId: string;
  snapshotModelId: string;
  snapshotTemperature: string;
  snapshotMaxTokens: string;
  snapshotSystemPrompt: string;
  snapshotWorkDirectory: string;
  snapshotEnabledMcpServiceIds: string[];
  snapshotEnabledToolKeys: string[];
  createdAt: number;
  updatedAt: number;
}
```

快照机制（Rust `AgentConversationRepository::create_from_profile`）：创建会话时从 Profile 拷贝全部配置。**设计意图**：会话与 Profile 版本解耦；`createdFromProfileUpdatedAt` 用于前端提示「Profile 已更新，是否应用新配置」（该提示未实现，属遗留 TODO）。

### AgentWorkspaceMessage

```ts
interface AgentWorkspaceMessage {
  id: string;
  conversationId: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  status: 'done' | 'error';   // 模型调用失败时，错误文案作为 assistant 消息入库，status=error
  createdAt: number;
}
```

### 遗留类型（未曾落地，重做时可参考其设计意图）

`types/agent/index.ts` 里还有一套**事件流式 Agent Run** 的类型：`StartAgentRunResponse`、`AgentEventPayload`、`RunStartedEventData`、`AssistantDeltaEventData`、`PlanCreatedEventData`、`StepStarted/FinishedEventData`、`ToolStarted/FinishedEventData`、`AnalysisSavedEventData`、`RunFinishedEventData`，以及 `AgentExecutionPlan`（计划-步骤模型）、`AgentToolTrace`（工具调用轨迹）、`AnalyzedModule`（代码库分析记忆）。配套的后端（start_agent_run / 事件推送 / 规划器）**从未实现**，只在 legacy `views/agent/detail.vue`（localStorage + setTimeout mock）时代存在。这套类型描述的是「目标 → 计划 → 分步执行 → 工具轨迹 → 模块记忆」的 agent 化方向，重做时是最有参考价值的部分。

## 4. 后端设计（commands/agent_conversation.rs，保留未删）

命令清单（仍在 `lib.rs` 注册，可直接调用）：

| 命令 | 说明 |
|---|---|
| `list_agent_conversations(agentProfileId)` | 按 Profile 列会话 |
| `list_recent_agent_conversations(limit?)` | 跨 Profile 最近会话（原侧边栏用） |
| `create_agent_conversation(agentProfileId)` | 从 Profile 拍快照建会话 |
| `get_agent_conversation(id)` / `delete_agent_conversation(id)` | 查/删 |
| `list_agent_conversation_messages(id)` | 拉历史消息 |
| `send_agent_conversation_message(id, content)` | 发消息 + 模型回复（见下） |
| `list/get/upsert/delete_agent_profile` | Profile CRUD |

`send_agent_conversation_message` 流程：

1. `append_user_message` 入库；
2. `run_snapshot_agent_turn`：
   - 从 `provider_settings`（JSON 存储，应用设置页维护）按 `snapshot_provider_id` 找配置，校验 `enabled` / `api_key` / `base_url`；
   - 用 **rig-core 的 OpenAI 兼容 client**（自定义 base_url），模型 = `snapshot_model_id`；
   - 消息组装：snapshot system prompt + 历史（跳过最后一条 user）+ 最后一条 user 作为 prompt；
   - snapshot 的 temperature / maxTokens 非空则 parse 后附加；
   - **非流式** `request.send().await`，提取纯文本（`AssistantContent::Text` 拼接）；
3. 成功 → `append_assistant_message(status="done")`；失败 → 错误文案入库 `status="error"`。

已知局限（重做时的改进点）：

- 非流式输出（无 delta 推送；类型层的 `AssistantDeltaEventData` 是当时的流式设想）；
- **没有 MCP 工具调用**：`enabledMcpServiceIds` / `enabledToolKeys` 在 Profile 与快照里存了，但聊天链路完全不消费；
- 无上下文长度控制（全量历史拼接）；错误作为消息入库，前端靠 `status` 区分展示。

另有 `commands/conversation.rs`（`send_conversation_message`，无状态单轮聊天，rig + default_model 兜底）——前端零调用的死代码，下架时保留在后端。

## 5. 前端设计

- 路由（已删）：`/agent`（列表）、`/agent/new`、`/agent/:id/edit`、`/agent/workspace?agent=&conversation=`；`/` 曾重定向到 `/agent`（现改指 `/mail`）。`/agent/:id` 曾重定向到 workspace。
- `views/agent/index.vue`：Profile 列表 + 入口。
- `views/agent/{create,edit}.vue`：包 `AgentProfileEditor`（609 行，本功能最大组件：provider/model 选择依赖 `default_model` 与 provider 配置服务、系统提示词、工作目录、MCP 服务多选、工具 keys）。
- `views/workspace/index.vue` + `AgentWorkspaceChat.vue`：会话界面；监听 route query（`agent` 无 conversation 时自动建会话；`conversation` 直达）；`useAgentWorkspaceStore` 用 `conversationRequestId` 递增使陈旧加载失效（切会话竞态防护，重做时值得照抄的模式）。
- `views/agent/components/ProjectFileTree.vue`：工作目录文件树，走 `services/project-files.ts`（`types/project-files`，**独立于 agent，仍在**）；`views/agent/components/{ChatPanel,AgentConversationPanel,MessageList,ToolTimeline}.vue`：聊天 UI，依赖 `components/ai-elements/` 组件库（已随功能下架，见 §6）。
- 侧边栏「最近会话」组件已于下架前先行移除（`layout/app-sidebar/recent-conversations/`）。

## 6. ai-elements 组件库

`src/components/ai-elements/`（约 12k 行）是 AI Elements registry 的拷贝（Conversation、Message、PromptInput、ChainOfThought、Tool 等），**仅被 agent 的 ChatPanel / AgentConversationPanel 使用**，与 `src/components/ui`（shadcn-vue，自动注册）不同，它是显式 import。已随功能删除；重做时建议直接从上游 registry 拉新版而非从归档恢复。

## 7. MCP 集成现状（重做时的核心待办）

- MCP **管理**完整保留在「应用设置 → MCP 服务」（`views/app-setting/_components/mcp-service/`，后端 `services/mcp_service.rs`，rmcp：child-process + streamable-http，支持 discover）。
- Agent 侧只做到「Profile 勾选启用哪些 MCP 服务」并随快照存储；**聊天链路从未把这些服务的工具接进模型**（`run_snapshot_agent_turn` 纯文本补全，无 tool 定义、无工具执行循环）。重做时需要：拉取启用服务的工具列表 → 组装 rig tool 定义 → 解析模型 tool call → rmcp 执行 → 回填结果（`AgentToolTrace` / `ToolTimeline` 就是为此准备的 UI）。

## 8. 恢复指引

- 完整代码：`git checkout archive/agent-feature`（或 `git diff main archive/agent-feature -- src/views/agent` 这类按路径取回）。
- 后端无需恢复：命令、仓储、模型、`lib.rs` 注册全部保留，JSON 数据文件仍在用户数据目录。
- 前端重做建议：按 §2 数据流新建 store/service（模式照旧）；聊天 UI 重新拉 ai-elements 新版；优先补齐流式输出与 MCP 工具调用；快照刷新提示（§3 的 TODO）可顺手做。
- 下架时同步更新过 `AGENTS.md`（路由表、模块表、产品定位描述），恢复时记得反向同步。
