# Agent Workspace Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dedicated three-pane agent workspace with Tauri-backed conversations and single-turn real agent execution, while keeping `/agent` as the profile management surface.

**Architecture:** Add a new Tauri conversation storage slice that persists conversation metadata, snapshot configuration, and messages. Expose conversation CRUD and a workspace-specific `send_agent_conversation_message` command, then build a new `/agent/workspace` frontend route with mail-style panes: agent list, conversation list, and live chat panel.

**Tech Stack:** Vue 3 + TypeScript, Tauri v2 invoke API, Rust + serde + LocalJsonStore, rig-core + rmcp

---

## File Map

### Create
- `src-tauri/src/models/agent_conversation.rs` — Rust models for conversation snapshot and persisted messages
- `src-tauri/src/db/repositories/agent_conversation.rs` — Tauri repository for conversations and message history
- `src-tauri/src/commands/agent_conversation.rs` — conversation CRUD and runtime commands
- `src/services/agent-conversation.ts` — frontend async service for workspace data
- `src/views/agent/workspace.vue` — new workspace route entry
- `src/views/agent/workspace/components/AgentWorkspaceShell.vue` — mail-style three-column layout
- `src/views/agent/workspace/components/AgentWorkspaceList.vue` — left pane agent selector
- `src/views/agent/workspace/components/AgentConversationList.vue` — middle pane conversation list
- `src/views/agent/workspace/components/AgentWorkspaceChat.vue` — right pane real chat panel

### Modify
- `src/router/index.ts` — add `/agent/workspace` route and redirect old detail route
- `src/layout/app-sidebar/menu/index.vue` — add workspace navigation entry
- `src/types/agent/index.ts` — replace old in-memory conversation shape with workspace-safe types and DTOs
- `src/services/agent-profile-storage.ts` — consume current async profile service in workspace
- `src/views/agent/index.vue` — add entry into workspace from management page
- `src/views/agent/detail.vue` — retire mock detail route by redirecting into workspace with selected agent
- `src-tauri/src/models/mod.rs` — export new models
- `src-tauri/src/db/repositories/mod.rs` — export new repository
- `src-tauri/src/commands/mod.rs` — export new command module
- `src-tauri/src/lib.rs` — register new Tauri commands

### Verify
- `CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation`
- `pnpm build`

---

## Task 1: Add Tauri Conversation Models and Repository

**Files:**
- Create: `src-tauri/src/models/agent_conversation.rs`
- Create: `src-tauri/src/db/repositories/agent_conversation.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/db/repositories/mod.rs`

- [ ] **Step 1: Write the failing Rust repository tests**

Cover:
- conversation creation snapshots the selected agent profile
- conversation list is filtered by `agent_profile_id` and sorted by `updated_at desc`
- message append updates `conversation.updated_at`
- message list preserves chronological order

```rust
#[test]
fn create_conversation_snapshots_agent_profile() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let store = LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
    let profile_repo = AgentProfileRepository::new(&store);
    let conversation_repo = AgentConversationRepository::new(&store);

    let agent_id = profile_repo
        .upsert(&AgentProfile {
            name: "Filesystem Assistant".to_string(),
            provider_id: "openai".to_string(),
            model_id: "gpt-4.1".to_string(),
            system_prompt: "Manage local files safely.".to_string(),
            enabled_mcp_service_ids: vec!["mcp_filesystem".to_string()],
            enabled_tool_keys: vec!["mcp_filesystem:read_file".to_string()],
            ..Default::default()
        })
        .expect("agent should persist");

    let profile = profile_repo.get(&agent_id).expect("lookup should succeed").expect("profile should exist");
    let conversation = conversation_repo
        .create_from_profile(&profile)
        .expect("conversation should be created");

    assert_eq!(conversation.agent_profile_id, agent_id);
    assert_eq!(conversation.snapshot_model_id, "gpt-4.1");
    assert_eq!(conversation.snapshot_enabled_tool_keys, vec!["mcp_filesystem:read_file"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
```

Expected: FAIL because conversation models and repository do not exist yet.

- [ ] **Step 3: Implement conversation models and repository**

Model structure:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentConversation {
    pub id: String,
    pub agent_profile_id: String,
    pub title: String,
    pub snapshot_version: u32,
    pub created_from_profile_updated_at: i64,
    pub snapshot_agent_name: String,
    pub snapshot_provider_id: String,
    pub snapshot_model_id: String,
    pub snapshot_temperature: String,
    pub snapshot_max_tokens: String,
    pub snapshot_system_prompt: String,
    pub snapshot_enabled_mcp_service_ids: Vec<String>,
    pub snapshot_enabled_tool_keys: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: i64,
}
```

Repository split:
- metadata stored in `agent_conversations.json`
- messages stored in `agent_conversation_messages.json`

Key methods:

```rust
pub fn list_by_agent(&self, agent_profile_id: &str) -> Result<Vec<AgentConversation>, String>;
pub fn get(&self, conversation_id: &str) -> Result<Option<AgentConversation>, String>;
pub fn create_from_profile(&self, profile: &AgentProfile) -> Result<AgentConversation, String>;
pub fn delete(&self, conversation_id: &str) -> Result<String, String>;
pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<AgentConversationMessage>, String>;
pub fn append_message(&self, message: &AgentConversationMessage) -> Result<String, String>;
```

When appending a message:
- persist message
- update the parent conversation `updated_at`

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
```

Expected: PASS for repository coverage.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/agent_conversation.rs \
  src-tauri/src/db/repositories/agent_conversation.rs \
  src-tauri/src/models/mod.rs \
  src-tauri/src/db/repositories/mod.rs
git commit -m "feat: add Tauri agent conversation storage"
```

---

## Task 2: Expose Conversation CRUD and Runtime Commands

**Files:**
- Create: `src-tauri/src/commands/agent_conversation.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing command contract**

Target command surface:

```rust
#[tauri::command]
pub fn list_agent_conversations(store: tauri::State<'_, LocalJsonStore>, agent_profile_id: String) -> Result<Vec<AgentConversation>, String>;

#[tauri::command]
pub fn create_agent_conversation(store: tauri::State<'_, LocalJsonStore>, agent_profile_id: String) -> Result<AgentConversation, String>;

#[tauri::command]
pub fn get_agent_conversation(store: tauri::State<'_, LocalJsonStore>, conversation_id: String) -> Result<Option<AgentConversation>, String>;

#[tauri::command]
pub fn delete_agent_conversation(store: tauri::State<'_, LocalJsonStore>, conversation_id: String) -> Result<String, String>;

#[tauri::command]
pub fn list_agent_conversation_messages(store: tauri::State<'_, LocalJsonStore>, conversation_id: String) -> Result<Vec<AgentConversationMessage>, String>;

#[tauri::command]
pub async fn send_agent_conversation_message(store: tauri::State<'_, LocalJsonStore>, conversation_id: String, content: String) -> Result<AgentConversationMessage, String>;
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
```

Expected: FAIL or compile error until command registration exists.

- [ ] **Step 3: Implement conversation CRUD commands**

Follow the same repository-backed pattern already used for agent profiles:

```rust
#[tauri::command]
pub fn create_agent_conversation(
    store: tauri::State<'_, LocalJsonStore>,
    agent_profile_id: String,
) -> Result<AgentConversation, String> {
    let profile_repo = AgentProfileRepository::new(&store);
    let profile = profile_repo
        .get(&agent_profile_id)?
        .ok_or_else(|| format!("未找到 id 为 {} 的 Agent", agent_profile_id))?;

    let repo = AgentConversationRepository::new(&store);
    repo.create_from_profile(&profile)
}
```

- [ ] **Step 4: Implement the single-turn runtime command**

Use the conversation snapshot instead of mutable profile data:

```rust
pub async fn send_agent_conversation_message(
    store: tauri::State<'_, LocalJsonStore>,
    conversation_id: String,
    content: String,
) -> Result<AgentConversationMessage, String> {
    let repo = AgentConversationRepository::new(&store);
    let conversation = repo
        .get(&conversation_id)?
        .ok_or_else(|| "未找到会话".to_string())?;

    let user_message = repo.append_user_message(&conversation.id, &content)?;
    let history = repo.list_messages(&conversation.id)?;

    let assistant_text = run_snapshot_agent_turn(&store, &conversation, &history).await?;
    repo.append_assistant_message(&conversation.id, &assistant_text)
}
```

For the first version, keep `run_snapshot_agent_turn` simple:
- resolve provider settings from Tauri storage
- build one rig request from snapshot prompt + history
- return a single assistant text response
- if runtime fails after user persistence, append an error-style assistant message instead of dropping the conversation state

- [ ] **Step 5: Run test to verify it passes**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
```

Expected: PASS and commands compile.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/agent_conversation.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/lib.rs
git commit -m "feat: add agent workspace conversation commands"
```

---

## Task 3: Add Frontend Conversation Service and Types

**Files:**
- Create: `src/services/agent-conversation.ts`
- Modify: `src/types/agent/index.ts`

- [ ] **Step 1: Define the frontend types used by the workspace**

Add types that match the new Tauri models without overloading the old in-memory shape:

```ts
export interface AgentConversation {
  id: string;
  agentProfileId: string;
  title: string;
  snapshotVersion: number;
  createdFromProfileUpdatedAt: number;
  snapshotAgentName: string;
  snapshotProviderId: string;
  snapshotModelId: string;
  snapshotTemperature: string;
  snapshotMaxTokens: string;
  snapshotSystemPrompt: string;
  snapshotEnabledMcpServiceIds: string[];
  snapshotEnabledToolKeys: string[];
  createdAt: number;
  updatedAt: number;
}

export interface AgentConversationMessage {
  id: string;
  conversationId: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  status: 'done' | 'error';
  createdAt: number;
}
```

- [ ] **Step 2: Add the Tauri invoke service**

Create:

```ts
export async function listAgentConversations(agentProfileId: string): Promise<AgentConversation[]>
export async function createAgentConversation(agentProfileId: string): Promise<AgentConversation>
export async function getAgentConversation(conversationId: string): Promise<AgentConversation | null>
export async function deleteAgentConversation(conversationId: string): Promise<string>
export async function listAgentConversationMessages(conversationId: string): Promise<AgentConversationMessage[]>
export async function sendAgentConversationMessage(conversationId: string, content: string): Promise<AgentConversationMessage>
```

Map snake_case Tauri payloads into existing camelCase frontend conventions.

- [ ] **Step 3: Run build to verify types line up**

Run:

```bash
pnpm build
```

Expected: PASS or a small set of expected compile errors in the next task if workspace pages are not wired yet.

- [ ] **Step 4: Commit**

```bash
git add src/services/agent-conversation.ts src/types/agent/index.ts
git commit -m "feat: add agent workspace conversation service types"
```

---

## Task 4: Build the Workspace UI

**Files:**
- Create: `src/views/agent/workspace.vue`
- Create: `src/views/agent/workspace/components/AgentWorkspaceShell.vue`
- Create: `src/views/agent/workspace/components/AgentWorkspaceList.vue`
- Create: `src/views/agent/workspace/components/AgentConversationList.vue`
- Create: `src/views/agent/workspace/components/AgentWorkspaceChat.vue`
- Modify: `src/views/agent/index.vue`
- Modify: `src/router/index.ts`
- Modify: `src/layout/app-sidebar/menu/index.vue`
- Modify: `src/views/agent/detail.vue`

- [ ] **Step 1: Create the top-level workspace route and shell**

Base route wiring:

```ts
{
  path: '/agent/workspace',
  component: () => import('@/views/agent/workspace.vue'),
}
```

And repurpose the old detail route as a redirect helper:

```ts
{
  path: '/agent/:id',
  redirect: to => `/agent/workspace?agent=${to.params.id}`,
}
```

- [ ] **Step 2: Build the shell using the mail layout pattern**

The shell should mirror the existing mail structure:

```vue
<ResizablePanelGroup direction="horizontal" class="h-full items-stretch">
  <ResizablePanel :default-size="22" :min-size="16" :max-size="26">
    <AgentWorkspaceList ... />
  </ResizablePanel>
  <ResizableHandle with-handle />
  <ResizablePanel :default-size="30" :min-size="22" :max-size="38">
    <AgentConversationList ... />
  </ResizablePanel>
  <ResizableHandle with-handle />
  <ResizablePanel :default-size="48" :min-size="32">
    <AgentWorkspaceChat ... />
  </ResizablePanel>
</ResizablePanelGroup>
```

- [ ] **Step 3: Keep selection state intentionally simple**

Workspace state should be only:

```ts
const selectedAgentId = ref<string | null>(null);
const selectedConversationId = ref<string | null>(null);
```

Do not:
- auto-select an agent
- auto-create a conversation
- restore the last viewed state

- [ ] **Step 4: Add the management-page entry point**

Add a clear button from `/agent` into the new workspace:

```vue
<Button variant="outline" @click="router.push('/agent/workspace')">
  打开工作台
</Button>
```

Add a dedicated sidebar entry as well so users can reach the workspace without going through the management page.

- [ ] **Step 5: Run build to verify the workspace renders**

Run:

```bash
pnpm build
```

Expected: PASS with the new route, shell, and empty states wired.

- [ ] **Step 6: Commit**

```bash
git add src/views/agent/workspace.vue \
  src/views/agent/workspace/components/AgentWorkspaceShell.vue \
  src/views/agent/workspace/components/AgentWorkspaceList.vue \
  src/views/agent/workspace/components/AgentConversationList.vue \
  src/views/agent/workspace/components/AgentWorkspaceChat.vue \
  src/views/agent/index.vue \
  src/views/agent/detail.vue \
  src/router/index.ts \
  src/layout/app-sidebar/menu/index.vue
git commit -m "feat: add agent workspace shell and routing"
```

---

## Task 5: Wire Real Workspace Behavior

**Files:**
- Modify: `src/views/agent/workspace.vue`
- Modify: `src/views/agent/workspace/components/AgentWorkspaceList.vue`
- Modify: `src/views/agent/workspace/components/AgentConversationList.vue`
- Modify: `src/views/agent/workspace/components/AgentWorkspaceChat.vue`

- [ ] **Step 1: Load agents and scoped conversations**

Use the existing Tauri-backed profile service for the left pane:

```ts
const agents = ref<AgentProfile[]>([]);
const conversations = ref<AgentConversation[]>([]);

watch(selectedAgentId, async agentId => {
  selectedConversationId.value = null;
  conversations.value = agentId ? await listAgentConversations(agentId) : [];
});
```

- [ ] **Step 2: Implement conversation creation**

Middle-pane create action:

```ts
async function handleCreateConversation() {
  if (!selectedAgentId.value) return;
  const conversation = await createAgentConversation(selectedAgentId.value);
  conversations.value = [conversation, ...conversations.value];
  selectedConversationId.value = conversation.id;
}
```

- [ ] **Step 3: Load and send real messages**

Right-pane send flow:

```ts
async function handleSend(content: string) {
  if (!selectedConversationId.value || isReplying.value) return;
  isReplying.value = true;

  try {
    await sendAgentConversationMessage(selectedConversationId.value, content);
    messages.value = await listAgentConversationMessages(selectedConversationId.value);
    conversations.value = await listAgentConversations(selectedAgentId.value!);
  } finally {
    isReplying.value = false;
  }
}
```

Right-pane empty states:
- no agent selected
- no conversation selected
- selected conversation with zero messages

- [ ] **Step 4: Run verification**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
pnpm build
```

Expected:
- conversation tests PASS
- frontend build PASS

- [ ] **Step 5: Manual smoke test**

Verify manually:
- workspace opens with no default selection
- selecting an agent loads only that agent's conversations
- creating a conversation snapshots the agent and shows it in the middle list
- sending a message persists user and assistant messages
- editing the agent later does not mutate old conversation snapshot behavior

- [ ] **Step 6: Commit**

```bash
git add src/views/agent/workspace.vue \
  src/views/agent/workspace/components/AgentWorkspaceList.vue \
  src/views/agent/workspace/components/AgentConversationList.vue \
  src/views/agent/workspace/components/AgentWorkspaceChat.vue
git commit -m "feat: wire real agent workspace conversations"
```

---

## Task 6: Final Verification

**Files:**
- Modify: none

- [ ] **Step 1: Run the final checks**

Run:

```bash
CARGO_TARGET_DIR=/tmp/eidolon-cargo-test cargo test --manifest-path src-tauri/Cargo.toml agent_conversation
pnpm build
git status --short
```

Expected:
- Rust tests PASS
- build PASS
- status shows only expected plan/spec artifacts or a clean tree

- [ ] **Step 2: Record residual follow-ups**

If anything remains intentionally deferred, note it explicitly:
- streaming token output
- background run orchestration
- richer tool trace rendering
- restoring last workspace selection

These are deliberate post-v1 follow-ups, not hidden gaps.
