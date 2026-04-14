# Agent Workspace Runtime Design

## Summary

This design replaces the current mock agent detail page with a real agent workspace built around three panes:

- left: agent list
- middle: conversations for the selected agent
- right: the live chat panel for the selected conversation

The workspace is a dedicated route separate from the existing `/agent` management page. Conversations and messages are persisted in Tauri. Each conversation is permanently bound to one agent and captures a snapshot of that agent's runtime configuration at creation time.

## Scope

### In Scope

- Keep `/agent` as the configuration and management surface for agent profiles
- Add a separate workspace route for real agent conversations
- Support multiple conversations per agent
- Persist conversations and conversation messages in Tauri
- Create one real tool-enabled rig agent turn per user message
- Load model, system prompt, MCP services, and tool selections from the conversation snapshot

### Out of Scope

- Reusing the current mock conversation page
- Allowing a conversation to switch agents mid-stream
- Auto-selecting an agent or conversation when the workspace opens
- Restoring the last active workspace selection
- Token streaming in the first version
- Background runs, resumable runs, or event-streamed agent execution

## Product Decisions

### Route Structure

- `/agent` remains the agent profile management page
- add a new workspace route such as `/agent/workspace`

### Initial State

- opening the workspace does not auto-select any agent
- no conversation is created automatically
- the user must first select an agent, then create or open a conversation

### Conversation Ownership

- one `AgentProfile` can own many conversations
- one conversation belongs to exactly one `AgentProfile`
- once created, a conversation never changes its bound agent

### Snapshot Rule

When a conversation is created, the backend stores a runtime snapshot copied from the selected agent profile. Later edits to the agent profile affect only new conversations, not existing ones.

## Information Architecture

The workspace layout follows the mail-style three-column structure.

### Left Pane: Agent List

Responsibilities:

- list available agents
- select the current agent
- show an empty state if no agents exist

Non-responsibilities:

- do not show message history
- do not auto-create a conversation
- do not edit the selected agent in place

### Middle Pane: Conversation List

Responsibilities:

- list conversations for the selected agent only
- sort by `updated_at desc`
- create a new conversation for the selected agent
- select an existing conversation
- show a scoped empty state when the selected agent has no conversations

Non-responsibilities:

- do not mix conversations from multiple agents
- do not allow changing the bound agent of an existing conversation

### Right Pane: Chat Panel

Responsibilities:

- display messages for the selected conversation
- send one user message at a time
- reflect the persisted message history from Tauri

Non-responsibilities:

- do not manage agent selection
- do not manage conversation creation
- do not write mock assistant replies in the frontend

## Data Model

The workspace adds two Tauri-owned entities.

### AgentConversation

Recommended fields:

- `id`
- `agent_profile_id`
- `title`
- `snapshot_version`
- `created_from_profile_updated_at`
- `snapshot_agent_name`
- `snapshot_provider_id`
- `snapshot_model_id`
- `snapshot_temperature`
- `snapshot_max_tokens`
- `snapshot_system_prompt`
- `snapshot_enabled_mcp_service_ids`
- `snapshot_enabled_tool_keys`
- `created_at`
- `updated_at`

Notes:

- `title` can start as a generic value such as `新对话`
- snapshot fields are the runtime truth for this conversation
- `updated_at` is used to drive middle-pane ordering

### AgentConversationMessage

Recommended fields:

- `id`
- `conversation_id`
- `role` (`user` | `assistant` | `system`)
- `content`
- `status` (`done` for the first version)
- `tool_traces` (optional, reserved for later)
- `created_at`

Notes:

- message ordering is determined by `created_at`
- the first version does not require incremental assistant updates

## Tauri Commands

The backend API should separate entity CRUD from runtime actions.

### Conversation Metadata

- `list_agent_conversations(agent_profile_id)`
- `create_agent_conversation(agent_profile_id)`
- `get_agent_conversation(conversation_id)`
- `delete_agent_conversation(conversation_id)`

### Conversation Messages

- `list_agent_conversation_messages(conversation_id)`

### Runtime Action

- `send_agent_conversation_message(conversation_id, content)`

## Runtime Flow

### Creating a Conversation

1. frontend provides `agent_profile_id`
2. Tauri loads the latest agent profile
3. Tauri creates an `AgentConversation`
4. Tauri copies the profile's runtime fields into the snapshot columns
5. Tauri persists the conversation and returns it

### Sending a Message

1. frontend calls `send_agent_conversation_message(conversation_id, content)`
2. Tauri loads the conversation and its snapshot
3. Tauri persists a new user message
4. Tauri loads existing messages for that conversation as history
5. Tauri constructs a tool-enabled rig agent using:
   - snapshot model
   - snapshot system prompt
   - snapshot MCP service ids
   - snapshot tool keys
6. Tauri executes a single request/response turn
7. Tauri persists one assistant message
8. Tauri updates `conversation.updated_at`
9. frontend reloads the selected conversation and message list

## Rig Integration Direction

The runtime path should use a tool-enabled rig agent, not the current generic default-model chat command.

The effective runtime configuration comes from the conversation snapshot, not from the latest mutable agent profile.

The first version should favor a simple single-turn execution path:

- no streaming
- no background queue
- no pause/resume
- no incremental tool event transport to the frontend

## Frontend Changes

### Remove

- the current mock-only agent detail page as the main conversation experience

### Add

- a new workspace route and page
- three-pane workspace components modeled after the mail layout
- empty states for:
  - no agent selected
  - no conversation selected
  - no conversations for selected agent

### Keep

- `/agent` profile management flows
- create/edit profile pages

## Error Handling

Explicitly handle:

- selected agent deleted while workspace is open
- selected conversation deleted while open
- snapshot references MCP services that no longer exist
- rig execution fails after user message persistence

For the first version, when runtime execution fails:

- keep the user message
- append an assistant error message or show a recoverable error state
- do not discard the conversation

## Testing

### Rust

- repository tests for conversation creation, sorting, and deletion
- repository tests for message persistence and ordering
- tests confirming conversation snapshot values are copied from agent profile
- tests confirming `send_agent_conversation_message` uses snapshot-backed runtime config

### Frontend

- verify workspace can render with no selected agent
- verify selecting an agent loads only its conversations
- verify creating a conversation produces a new row in the middle pane
- verify sending a message updates the right pane and conversation ordering
- verify `/agent` management remains independent from the workspace

### Build Verification

- `cargo test --manifest-path src-tauri/Cargo.toml agent_conversation`
- `pnpm build`

## Implementation Notes

- reuse the existing Tauri `LocalJsonStore` repository/command pattern
- do not overload the current `send_conversation_message` command; create a workspace-specific runtime command
- keep the workspace state model simple: selected agent id, selected conversation id, and fetched collections
- defer streaming and richer run-state orchestration until after the basic workspace and persistence loop is stable
