# Agent Profile Tauri Storage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `AgentProfile` persistence from frontend `localStorage` to Tauri `LocalJsonStore` while keeping mock conversation history in frontend local storage.

**Architecture:** Add a new Rust-side `agent_profile` storage slice that mirrors the existing repository/command pattern used by MCP services and provider settings. Replace the frontend's synchronous profile CRUD with async Tauri-backed service calls plus a one-time localStorage migration path, while leaving conversation message persistence untouched.

**Tech Stack:** Vue 3 + TypeScript, Tauri v2 invoke API, Rust + serde + LocalJsonStore

---

## File Map

### Create
- `src-tauri/src/models/agent_profile.rs` — Rust `AgentProfile` DTO serialized through Tauri
- `src-tauri/src/db/repositories/agent_profile.rs` — `LocalJsonStore` repository for listing/upserting/deleting profiles
- `src-tauri/src/commands/agent_profile.rs` — Tauri commands exposing agent profile CRUD
- `src/services/agent-profile-storage.ts` — frontend async invoke wrapper + one-time migration logic
- `docs/superpowers/plans/2026-04-14-agent-profile-tauri-storage.md` — this implementation plan

### Modify
- `src-tauri/src/models/mod.rs` — export new Rust model module
- `src-tauri/src/db/repositories/mod.rs` — export new repository module
- `src-tauri/src/commands/mod.rs` — export new command module
- `src-tauri/src/lib.rs` — register new commands
- `src/services/agent-profile.ts` — keep only conversation message storage helpers
- `src/views/agent/index.vue` — async profile loading
- `src/views/agent/create.vue` — async save flow
- `src/views/agent/edit.vue` — async load/save flow
- `src/views/agent/detail.vue` — async profile loading before mock conversation setup

### Verify
- `cargo test --manifest-path src-tauri/Cargo.toml agent_profile`
- `pnpm build`

---

## Task 1: Add Rust AgentProfile Storage

**Files:**
- Create: `src-tauri/src/models/agent_profile.rs`
- Create: `src-tauri/src/db/repositories/agent_profile.rs`
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/db/repositories/mod.rs`

- [ ] **Step 1: Write the failing Rust repository tests**

Create repository tests that prove:
- `upsert` generates `agent_` ids for new records
- `list` sorts by `updated_at` descending
- `delete` removes a persisted record
- `enabled_mcp_service_ids` and `enabled_tool_keys` round-trip

```rust
#[test]
fn upsert_generates_agent_id_and_persists_profile() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let store = LocalJsonStore::new(temp_dir.path().to_path_buf()).expect("store should be created");
    let repository = AgentProfileRepository::new(&store);

    let profile = AgentProfile {
        name: "Filesystem Assistant".to_string(),
        provider_id: "openai".to_string(),
        model_id: "gpt-4.1".to_string(),
        system_prompt: "Help me manage local files safely.".to_string(),
        enabled_mcp_service_ids: vec!["mcp_filesystem".to_string()],
        enabled_tool_keys: vec!["mcp_filesystem:read_file".to_string()],
        ..Default::default()
    };

    let profile_id = repository.upsert(&profile).expect("profile should be persisted");

    assert!(profile_id.starts_with("agent_"));

    let persisted: HashMap<String, AgentProfile> = store.read("agent_profiles").expect("profiles should be readable");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[&profile_id].enabled_mcp_service_ids, vec!["mcp_filesystem"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
```

Expected: FAIL because `agent_profile` model/repository do not exist yet.

- [ ] **Step 3: Add the Rust model and repository**

Implement a serde-friendly `AgentProfile` struct and repository matching the spec:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub temperature: String,
    #[serde(default)]
    pub max_tokens: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub enabled_mcp_service_ids: Vec<String>,
    #[serde(default)]
    pub enabled_tool_keys: Vec<String>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}
```

Repository shape:

```rust
const PROFILES_FILENAME: &str = "agent_profiles";

pub struct AgentProfileRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, AgentProfile>>,
}
```

Normalization rules:
- `name`, `provider_id`, `model_id`, `system_prompt` must be non-empty after trim
- `temperature` defaults to `"0.7"` if empty
- `max_tokens` defaults to `"4096"` if empty
- new ids use `agent_{nanoid!(10)}`
- `created_at` preserved on update
- `updated_at` always set to current Unix milliseconds

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
```

Expected: PASS for new `agent_profile` repository tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/agent_profile.rs \
  src-tauri/src/db/repositories/agent_profile.rs \
  src-tauri/src/models/mod.rs \
  src-tauri/src/db/repositories/mod.rs
git commit -m "feat: add Tauri storage for agent profiles"
```

---

## Task 2: Expose Tauri Commands For Agent Profiles

**Files:**
- Create: `src-tauri/src/commands/agent_profile.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing command integration expectation**

Add a lightweight compile-targeted test in the new command file or repository module that exercises repository-backed list/get/delete flow, or rely on a failing frontend invoke compile once command registration is added. The key expected API is:

```rust
#[tauri::command]
pub fn list_agent_profiles(store: tauri::State<'_, LocalJsonStore>) -> Result<Vec<AgentProfile>, String>;

#[tauri::command]
pub fn get_agent_profile(store: tauri::State<'_, LocalJsonStore>, profile_id: String) -> Result<Option<AgentProfile>, String>;

#[tauri::command]
pub fn upsert_agent_profile(store: tauri::State<'_, LocalJsonStore>, profile: AgentProfile) -> Result<String, String>;

#[tauri::command]
pub fn delete_agent_profile(store: tauri::State<'_, LocalJsonStore>, profile_id: String) -> Result<String, String>;
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
```

Expected: FAIL or compile error until command module and registration exist.

- [ ] **Step 3: Implement and register commands**

Follow the same pattern used by `mcp_service` and `model_config`:

```rust
#[tauri::command]
pub fn list_agent_profiles(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<AgentProfile>, String> {
    let repo = AgentProfileRepository::new(&store);
    repo.list()
}
```

Also update:

```rust
// src-tauri/src/commands/mod.rs
pub mod agent_profile;

// src-tauri/src/lib.rs
commands::agent_profile::list_agent_profiles,
commands::agent_profile::get_agent_profile,
commands::agent_profile::upsert_agent_profile,
commands::agent_profile::delete_agent_profile,
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
```

Expected: PASS with commands compiled and repository tests still green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/agent_profile.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/lib.rs
git commit -m "feat: expose agent profile CRUD commands"
```

---

## Task 3: Split Frontend Services And Add Migration

**Files:**
- Create: `src/services/agent-profile-storage.ts`
- Modify: `src/services/agent-profile.ts`

- [ ] **Step 1: Write the failing service API shape**

Define the new async frontend API that all pages will consume:

```ts
export async function listAgentProfiles(): Promise<AgentProfile[]>
export async function getAgentProfile(profileId: string): Promise<AgentProfile | null>
export async function upsertAgentProfile(input: AgentProfileInput): Promise<AgentProfile>
export async function deleteAgentProfile(profileId: string): Promise<string>
```

Migration helper target:

```ts
const LEGACY_AGENT_PROFILE_STORAGE_KEY = 'eidolon.agent_profiles';
const AGENT_PROFILE_MIGRATION_KEY = 'eidolon.agent_profiles.migrated_to_tauri';
```

- [ ] **Step 2: Run build to verify it fails before implementation**

Run:

```bash
pnpm build
```

Expected: FAIL after page imports are switched but before the new async service exists.

- [ ] **Step 3: Implement the Tauri-backed profile service and isolate conversation storage**

`src/services/agent-profile-storage.ts` should:
- invoke new Tauri commands with `@tauri-apps/api/core`
- perform one-time migration from legacy `localStorage`
- return mapped frontend `AgentProfile` values

Migration flow:

```ts
async function ensureAgentProfileMigration() {
  if (!canUseStorage() || window.localStorage.getItem(AGENT_PROFILE_MIGRATION_KEY) === 'done') {
    return;
  }

  const tauriProfiles = await invoke<AgentProfile[]>('list_agent_profiles');
  if (tauriProfiles.length > 0) {
    window.localStorage.setItem(AGENT_PROFILE_MIGRATION_KEY, 'done');
    return;
  }

  const legacyProfiles = readLegacyProfiles();
  for (const profile of legacyProfiles) {
    await invoke<string>('upsert_agent_profile', { profile: toTauriAgentProfile(profile) });
  }

  window.localStorage.setItem(AGENT_PROFILE_MIGRATION_KEY, 'done');
}
```

`src/services/agent-profile.ts` should keep only:
- `listAgentConversationMessages`
- `saveAgentConversationMessages`

- [ ] **Step 4: Run build to verify it passes**

Run:

```bash
pnpm build
```

Expected: PASS with the new service module available and conversation storage still working.

- [ ] **Step 5: Commit**

```bash
git add src/services/agent-profile-storage.ts src/services/agent-profile.ts
git commit -m "refactor: move agent profile storage behind Tauri service"
```

---

## Task 4: Refactor Agent Pages To Async Profile Loading

**Files:**
- Modify: `src/views/agent/index.vue`
- Modify: `src/views/agent/create.vue`
- Modify: `src/views/agent/edit.vue`
- Modify: `src/views/agent/detail.vue`

- [ ] **Step 1: Update pages to use async service contracts**

Refactor page responsibilities:

`src/views/agent/index.vue`

```ts
const profiles = ref<AgentProfile[]>([]);
const isLoading = ref(true);

async function loadProfiles() {
  try {
    profiles.value = await listAgentProfiles();
  } finally {
    isLoading.value = false;
  }
}
```

`src/views/agent/create.vue`

```ts
async function handleSave(value: AgentProfileInput) {
  const profile = await upsertAgentProfile(value);
  toast.success('Agent 已创建');
  router.push(`/agent/${profile.id}`);
}
```

`src/views/agent/edit.vue`

```ts
const profile = ref<AgentProfile | null>(null);
const isLoading = ref(true);

async function loadProfile() {
  profile.value = await getAgentProfile(profileId.value);
  isLoading.value = false;
}
```

`src/views/agent/detail.vue`

```ts
async function loadProfile() {
  profile.value = await getAgentProfile(profileId.value);
}

watch(profileId, async () => {
  await loadProfile();
  loadMessages();
}, { immediate: true });
```

- [ ] **Step 2: Preserve existing UX states**

Add or keep:
- loading placeholders for list/edit/detail
- existing “没有找到这个 Agent” state after async fetch completes
- existing mock conversation message persistence via `src/services/agent-profile.ts`

- [ ] **Step 3: Run build to verify it passes**

Run:

```bash
pnpm build
```

Expected: PASS with no TypeScript errors and async profile loading wired through all agent pages.

- [ ] **Step 4: Run focused Rust tests again**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
```

Expected: PASS to confirm backend storage still behaves after frontend integration.

- [ ] **Step 5: Commit**

```bash
git add src/views/agent/index.vue \
  src/views/agent/create.vue \
  src/views/agent/edit.vue \
  src/views/agent/detail.vue
git commit -m "refactor: load agent pages from Tauri profile storage"
```

---

## Task 5: Final Verification

**Files:**
- Modify: none

- [ ] **Step 1: Run full verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml agent_profile
pnpm build
```

Expected:
- Rust tests PASS
- Vite production build PASS

- [ ] **Step 2: Manual migration sanity check**

Verify manually:
- create an agent, reload app, confirm it still exists
- edit an agent, reload app, confirm edits persist
- existing mock conversation messages still remain local
- if legacy local agent profiles exist, first run imports them once

- [ ] **Step 3: Commit any remaining cleanups**

```bash
git status --short
```

Expected: no unexpected changes beyond the planned files.
