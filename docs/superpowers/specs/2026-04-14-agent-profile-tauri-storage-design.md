# Agent Profile Tauri Storage Design

## Summary

This change migrates `AgentProfile` persistence from frontend `localStorage` to Tauri `LocalJsonStore` while keeping agent conversation messages in frontend `localStorage` for now. The goal is to make agent configuration durable and owned by the desktop backend without changing the current mock conversation runtime.

## Scope

### In Scope

- Persist `AgentProfile` records through Tauri commands backed by `LocalJsonStore`
- Support `list`, `get`, `upsert`, and `delete` for agent profiles
- Update frontend agent pages to load and save profile data through async Tauri invokes
- Migrate legacy frontend-stored agent profiles into Tauri the first time the new service loads
- Keep the existing mock conversation page behavior unchanged apart from reading profile data from Tauri

### Out of Scope

- Replacing the mock conversation flow with a real rig runtime
- Persisting agent conversations in Tauri
- Streaming agent runs, tool traces, or runtime events from Rust to the frontend
- Changing MCP discovery or provider/model configuration behavior beyond what agent profiles already reference

## Current State

- Frontend agent profiles are stored in `localStorage` through `src/services/agent-profile.ts`
- Frontend conversation history is also stored in `localStorage`
- Tauri already persists other configuration domains such as MCP services and provider settings through `LocalJsonStore` repositories and commands
- Rust already depends on `rig-core` and `rmcp`, but the current agent detail page is still a mock conversation flow

## Desired End State

- `AgentProfile` becomes a Tauri-owned configuration entity, following the same storage pattern as MCP services
- Frontend pages no longer treat agent profile reads and writes as synchronous local operations
- Existing local agent profiles are preserved via one-time migration into Tauri-backed storage
- Conversation messages remain local for this phase so runtime work can be implemented separately later

## Architecture

### Rust Side

Add a new `agent_profile` model, repository, and command set that mirrors the existing MCP service structure:

- `src-tauri/src/models/agent_profile.rs`
- `src-tauri/src/db/repositories/agent_profile.rs`
- `src-tauri/src/commands/agent_profile.rs`

Storage uses `LocalJsonStore` with a dedicated file such as `agent_profiles.json`.

The repository is responsible for:

- normalizing incoming profile fields
- generating an id for new profiles when needed
- sorting `list` results by `updated_at` descending
- writing the normalized profile map back to `LocalJsonStore`

The command layer exposes:

- `list_agent_profiles`
- `get_agent_profile`
- `upsert_agent_profile`
- `delete_agent_profile`

### Frontend Side

Split the current frontend service responsibilities:

- `AgentProfile` CRUD moves to async Tauri-backed service functions
- conversation message storage remains in frontend local storage for now

The frontend agent pages become async:

- agent list page loads profiles on mount / activation through Tauri
- create page saves through Tauri
- edit page fetches the selected profile through Tauri and saves through Tauri
- detail page fetches the selected profile through Tauri before loading mock conversation history

## Migration Strategy

Use a one-time frontend-driven migration:

1. Load Tauri agent profiles
2. If Tauri already has any profiles, use them and skip migration
3. If Tauri is empty, read legacy agent profiles from frontend `localStorage`
4. Upsert each legacy profile into Tauri storage
5. Mark migration complete in frontend local storage so the import does not repeat

This keeps Tauri as the only source of truth after migration while preserving existing user-created profiles.

## Data Shape

The Tauri-side `AgentProfile` shape should match the frontend shape closely so migration is mechanical:

- `id`
- `name`
- `description`
- `provider_id`
- `model_id`
- `temperature`
- `max_tokens`
- `system_prompt`
- `enabled_mcp_service_ids`
- `enabled_tool_keys`
- `created_at`
- `updated_at`

Rust naming can stay idiomatic snake_case while the frontend invoke layer maps to existing TypeScript fields.

## UI and Behavior Changes

The visible UI should not materially change in this phase. The main difference is loading behavior:

- agent pages may need loading states where they currently assume synchronous data
- edit and detail pages should handle missing profiles after async fetch, just as they already do for absent local data

The mock conversation page still:

- creates frontend-only user/assistant messages
- persists conversation history locally
- reads only the selected profile from Tauri

## Error Handling

Handle the following explicitly:

- Tauri read/write failures show a toast and preserve the current page state where possible
- migration failures should not delete legacy local data
- malformed legacy local profiles should be skipped rather than blocking all migration
- detail and edit pages should continue to show their existing "not found" states if a profile cannot be loaded

## Testing

### Rust

- repository tests for id generation, normalization, sort order, and persistence
- command-level smoke coverage if existing command tests in this area are lightweight
- migration-relevant tests for round-tripping `enabled_mcp_service_ids` and `enabled_tool_keys`

### Frontend

- no new frontend test harness is required for this phase
- verify behavior through `pnpm build`
- manually verify list, create, edit, detail, and reload flows against Tauri-backed data
- manually verify that an existing local profile is imported on first launch after the migration

## Implementation Notes

- Reuse the existing `LocalJsonStore` and repository pattern rather than introducing a new persistence abstraction
- Keep the migration code isolated to the frontend agent profile service so it can be removed cleanly in a later cleanup pass
- Do not start wiring rig runtime creation in this change; that belongs to the next phase once profile persistence is stable
