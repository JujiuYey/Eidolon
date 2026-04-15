---
title: feat: add agent safety boundaries
type: feat
status: active
date: 2026-04-15
origin: docs/brainstorms/2026-04-15-agent-safety-boundaries-requirements.md
---

# feat: add agent safety boundaries

## Overview

Add a first-phase safety boundary model for agents by introducing `workDirectory` on agent profiles, snapshotting it into conversations, surfacing it in UI, and establishing a reusable hard path-boundary helper for future local-workspace operations. This phase deliberately stops short of full sandboxing and focuses on trustworthy scope, capability gating, and migration-safe persistence.

## Problem Frame

The current agent model defines provider, model, prompt, MCP services, and tool selections, but it does not define which local project an agent is supposed to operate within. That leaves the product with no explicit workspace boundary and makes it difficult to safely expand beyond chat into file access, project scanning, or command execution. The requirements doc already establishes that this boundary should be real product behavior rather than mere prompt guidance (see origin: `docs/brainstorms/2026-04-15-agent-safety-boundaries-requirements.md`).

## Requirements Trace

- R1. Add `workDirectory` to each agent profile.
- R2. Snapshot the effective working directory into each conversation at creation time.
- R3. Treat working directory as runtime boundary context, not prompt-only metadata.
- R5. Agents without `workDirectory` remain chat-safe only.
- R7. Explain boundary meaning clearly in agent management UI.
- R8. Surface active boundary context in workspace/conversation UI.
- R9. Prioritize hard path-boundary enforcement before stronger sandbox claims.
- R10. Avoid implying that `workDirectory` equals full sandboxing.

## Scope Boundaries

- Do not implement containers, OS sandboxing, or VM isolation in this phase.
- Do not redesign the full permission system for every future tool type.
- Do not add multi-root workspace support or project switching flows.
- Do not claim MCP-tool boundary enforcement that the current runtime does not actually perform.

## Context & Research

### Relevant Code and Patterns

- `src/types/agent/index.ts` already defines the canonical frontend `AgentProfile`, `AgentProfileInput`, and conversation snapshot types.
- `src-tauri/src/models/agent_profile.rs`, `src-tauri/src/db/repositories/agent_profile.rs`, and `src-tauri/src/commands/agent_profile.rs` are the existing pattern for profile persistence and validation.
- `src-tauri/src/models/agent_conversation.rs` and `src-tauri/src/db/repositories/agent_conversation.rs` already snapshot model- and prompt-level runtime fields into conversations.
- `src/services/agent-profile-storage.ts` handles Tauri <-> frontend shape conversion and the one-time legacy localStorage migration.
- `src/views/agent/components/AgentProfileEditor.vue` is the current profile management surface and the natural place to collect and explain `workDirectory`.
- `src/views/workspace/index.vue`, `src/views/workspace/components/AgentWorkspaceChat.vue`, and `src/layout/app-sidebar/recent-conversations/index.vue` already expose conversation-bound runtime context and can surface the effective boundary.
- `src/services/project-files.ts` already defines a frontend service shape for project-file operations, but the current `src-tauri` tree does not yet implement or register matching commands. That means this plan should establish the boundary model before those capabilities are exposed.
- `src-tauri/src/commands/agent_conversation.rs` currently executes plain model completion and does not yet wire MCP services or tool execution into the runtime. This phase should not pretend otherwise.

### Institutional Learnings

- No `docs/solutions/` knowledge base exists yet in this repository, so there are no stored institutional learnings to incorporate.

### External References

- Anthropic computer-use docs: https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/computer-use-tool
- Anthropic bash tool docs: https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/bash-tool
- Anthropic code execution docs: https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/code-execution-tool
- OpenAI Codex security advisory on workspace-boundary bypass: https://github.com/openai/codex/security/advisories/GHSA-w5fx-fh39-j5rw
- OpenAI Codex repository overview: https://github.com/openai/codex

## Key Technical Decisions

- Store `workDirectory` as an optional raw filesystem path in phase one. The current codebase has no project/workspace entity to bind against, and inventing one now would expand scope before boundary behavior is validated.
- Preserve backward compatibility by treating missing `workDirectory` as `null`/empty and classifying that profile as chat-safe only.
- Snapshot `workDirectory` into conversations alongside the existing immutable runtime fields so that later profile edits do not silently change the scope of in-progress conversations.
- Add a dedicated Tauri-side path-boundary helper now, even though project-file commands are not yet registered, so future local-workspace operations have one canonical containment check rather than ad hoc path filtering.
- Keep MCP/tool-specific enforcement out of this phase because the current runtime does not yet execute MCP-backed local tools. This plan should establish the boundary model first, not over-claim runtime isolation that does not exist.

## Open Questions

### Resolved During Planning

- Should phase one store a raw filesystem path or introduce a higher-level project entity immediately?: Use a raw filesystem path first. It fits the current data model, minimizes schema churn, and still supports later evolution into a project/workspace abstraction.
- How should the plan handle local-workspace enforcement when project-file commands are not yet wired into `src-tauri`?: Land the boundary helper and policy classification now, then treat future local-workspace commands as required consumers of that helper before they become user-facing.

### Deferred to Implementation

- Which exact actions should count as high-risk for confirmation inside the allowed directory?: The initial list can be settled while touching real local-operation entry points, because the answer depends on which operations actually exist after codebase exploration.
- Should `workDirectory` allow relative paths, symlinks, or non-existent directories at save time?: This should be finalized while implementing normalization and validation, because it depends on the chosen cross-platform path handling approach.

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

```text
AgentProfile.workDirectory
  -> persisted in Tauri profile storage
  -> exposed in frontend profile service/types
  -> copied into AgentConversation snapshot at create time
  -> displayed in workspace UI as active scope
  -> used to classify the conversation/profile as:
       - chat-safe only (no workDirectory)
       - local-workspace scoped (has workDirectory)

Future local-workspace operations
  -> normalize requested path + effective workDirectory
  -> reject requests outside canonical boundary
  -> keep destructive actions separately gated
```

## Implementation Units

- [ ] **Unit 1: Add `workDirectory` to profile persistence**

**Goal:** Extend the agent profile model, storage, and frontend invoke layer so the system can save and load a working-directory boundary without breaking existing profiles.

**Requirements:** R1, R5

**Dependencies:** None

**Files:**
- Modify: `src/types/agent/index.ts`
- Modify: `src-tauri/src/models/agent_profile.rs`
- Modify: `src-tauri/src/db/repositories/agent_profile.rs`
- Modify: `src-tauri/src/commands/agent_profile.rs`
- Modify: `src/services/agent-profile-storage.ts`
- Test: `src-tauri/src/db/repositories/agent_profile.rs`

**Approach:**
- Add an optional `workDirectory` field to frontend and Rust profile types.
- Normalize the stored value consistently in the repository layer so blank strings do not masquerade as real boundaries.
- Keep legacy migration safe by defaulting missing historical values to empty/none rather than failing migration.

**Patterns to follow:**
- Existing profile normalization in `src-tauri/src/db/repositories/agent_profile.rs`
- Existing Tauri/TypeScript shape conversion in `src/services/agent-profile-storage.ts`

**Test scenarios:**
- Happy path: saving a profile with a valid directory round-trips through Tauri and returns the same value.
- Edge case: saving a profile with blank `workDirectory` persists as empty/none without breaking required-field validation.
- Edge case: legacy profiles missing the new field still list and load successfully.
- Error path: invalid profile saves still fail for the existing required fields and do not regress due to the new optional field.

**Verification:**
- Existing profiles remain readable, and newly saved profiles can carry a stable `workDirectory` value end-to-end.

- [ ] **Unit 2: Snapshot the effective working directory into conversations**

**Goal:** Make conversation scope immutable by copying the effective working directory into conversation snapshots when a new conversation is created.

**Requirements:** R2, R3, R8

**Dependencies:** Unit 1

**Files:**
- Modify: `src/types/agent/index.ts`
- Modify: `src-tauri/src/models/agent_conversation.rs`
- Modify: `src-tauri/src/db/repositories/agent_conversation.rs`
- Modify: `src/services/agent-conversation.ts`
- Modify: `src/stores/agent-workspace.ts`
- Test: `src-tauri/src/db/repositories/agent_conversation.rs`

**Approach:**
- Add snapshot fields for working-directory scope to the conversation model and conversion layers.
- Copy the profile-level boundary into the conversation at creation time, following the same snapshot rule already used for prompt/model/tool configuration.
- Expose the snapshot value to the workspace store so the UI always renders conversation-effective scope, not the latest mutable profile state.

**Patterns to follow:**
- Existing snapshot copying in `src-tauri/src/db/repositories/agent_conversation.rs`
- Existing conversation DTO conversion in `src/services/agent-conversation.ts`

**Test scenarios:**
- Happy path: creating a conversation from a profile with `workDirectory` copies that value into the snapshot.
- Edge case: creating a conversation from a chat-only profile stores an empty/none snapshot value without error.
- Edge case: editing the profile after conversation creation does not mutate the historical conversation snapshot.
- Integration: recent conversations and active workspace loading continue to work with the expanded conversation payload.

**Verification:**
- Conversations carry a stable working-directory boundary that does not drift when profiles change later.

- [ ] **Unit 3: Add boundary-aware Agent management and workspace UI**

**Goal:** Let users set, understand, and see the effective working-directory boundary without overstating the safety guarantees.

**Requirements:** R1, R5, R7, R8, R10

**Dependencies:** Unit 1, Unit 2

**Files:**
- Modify: `src/views/agent/components/AgentProfileEditor.vue`
- Modify: `src/views/agent/index.vue`
- Modify: `src/views/workspace/index.vue`
- Modify: `src/views/workspace/components/AgentWorkspaceChat.vue`
- Modify: `src/layout/app-sidebar/recent-conversations/index.vue`
- Test: none yet — verify through `pnpm build` and targeted manual checks; the repository currently has no frontend test harness

**Approach:**
- Add a `workDirectory` input in the Agent basic settings section with explanatory copy that distinguishes “scoped workspace” from “full sandbox”.
- Make chat-only posture explicit when no working directory is set, so users are not misled into expecting local-project actions.
- Surface the effective conversation scope in the workspace header or nearby context area using the conversation snapshot, not the mutable profile.

**Patterns to follow:**
- Existing profile form sections and save flow in `src/views/agent/components/AgentProfileEditor.vue`
- Existing workspace runtime context rendering in `src/views/workspace/components/AgentWorkspaceChat.vue`

**Test scenarios:**
- Happy path: create/edit Agent forms save a working directory and preserve it on reload.
- Happy path: a conversation created from a scoped Agent shows the effective working directory in the workspace.
- Edge case: an Agent with no working directory clearly appears chat-safe only and does not imply local workspace powers.
- Error path: UI copy does not claim sandboxing or isolation the product does not actually enforce.

**Verification:**
- Users can see and understand the effective workspace boundary before asking the agent to act.

- [ ] **Unit 4: Introduce reusable hard path-boundary enforcement utilities**

**Goal:** Establish a canonical Tauri-side containment check that future local-workspace operations must use before any project-file or command-execution surfaces become user-facing.

**Requirements:** R3, R4, R6, R9, R10

**Dependencies:** Unit 1

**Files:**
- Create: `src-tauri/src/services/work_directory.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/work_directory.rs`

**Approach:**
- Implement a small boundary helper that normalizes the effective working directory and requested paths, then decides whether the operation stays within the allowed root.
- Keep the API narrow and purpose-built so future command/file services can call it without inventing divergent security logic.
- Encode “boundary check” separately from “destructive action confirmation” so later high-risk operation policies can layer on top without weakening containment.

**Technical design:** *(directional guidance, not implementation specification)*

```text
assert_path_within_work_directory(work_directory, requested_path)
  -> canonicalize/normalize both paths
  -> reject if requested path escapes allowed root
  -> return normalized safe path for downstream consumers
```

**Patterns to follow:**
- Existing Rust-side service/module organization in `src-tauri/src/services`
- Existing focused repository tests in `src-tauri/src/db/repositories/agent_profile.rs`

**Test scenarios:**
- Happy path: paths inside the allowed directory are accepted and returned in normalized form.
- Edge case: nested child directories remain accepted.
- Error path: `..` traversal and sibling-directory escapes are rejected.
- Edge case: empty or missing working-directory input fails closed for local-workspace operations.
- Edge case: cross-platform path separator differences do not break containment checks.

**Verification:**
- The codebase has one reusable hard-boundary primitive ready for any future local-workspace command surface.

## System-Wide Impact
`
- **Interaction graph:** Agent profile save/load, conversation creation, workspace rendering, and future local-workspace command surfaces will all share the new boundary concept.
- **Error propagation:** Invalid or escaped local paths should fail at the boundary helper layer and bubble up as safe user-facing errors rather than leaking into downstream command/file logic.
- **State lifecycle risks:** Conversation snapshots must preserve historical working-directory context even if the underlying profile changes later.
- **API surface parity:** Every future local-workspace capability added to Tauri must consume the same boundary rule instead of introducing custom path filtering.
- **Integration coverage:** Backend repository tests will prove schema and snapshot behavior; frontend verification must confirm the UI communicates scope accurately even without a formal test harness.
- **Unchanged invariants:** This phase does not turn the current workspace into a true sandbox, and it does not change the current non-MCP, model-completion runtime path in `src-tauri/src/commands/agent_conversation.rs`.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Path normalization behaves differently across operating systems or symlinked directories | Keep boundary logic centralized in one Rust helper with explicit traversal-focused tests before wiring consumers |
| Users misread `workDirectory` as full sandboxing | Use explicit UI copy and preserve non-goal language in plan/docs |
| Existing profiles without a working directory feel regressed or confusing | Classify them as chat-safe only and surface that status clearly instead of failing hard |
| Future local-workspace features skip the shared boundary helper | Make the helper a required integration point and call it out in plan/system-wide impact |

## Documentation / Operational Notes

- Update the product copy around Agent creation/editing so “working directory”, “local project actions”, and “sandbox” are not conflated.
- When local project-file or shell capabilities become real user-facing features, their plans should explicitly reference this boundary model instead of redefining security behavior.

## Sources & References

- **Origin document:** `docs/brainstorms/2026-04-15-agent-safety-boundaries-requirements.md`
- Related code: `src/views/agent/components/AgentProfileEditor.vue`
- Related code: `src-tauri/src/models/agent_profile.rs`
- Related code: `src-tauri/src/models/agent_conversation.rs`
- External docs: https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/computer-use-tool
- External docs: https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/code-execution-tool
- External docs: https://github.com/openai/codex/security/advisories/GHSA-w5fx-fh39-j5rw
