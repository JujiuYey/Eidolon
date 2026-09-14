# Repository Guidelines

> Scope: practical orientation for AI assistants working in the **Eidolon** codebase (Tauri 2 + Vue 3 desktop app). Keep it concise; expand only when reality changes.

---

## Project Overview

Eidolon is a **Tauri 2 + Vue 3 desktop client** (product name "Eidolon", bundle id `dev.eidolon.app`, version `0.1.0`) that ships a chat-driven agent workspace, a Postman-like API client, CRUD code generation (Go backend + frontend), and AI provider / MCP configuration.

- Frontend: Vue 3.5 SFC, Vite 7, TypeScript 5.8, Pinia 3 (with `pinia-plugin-persistedstate`), vue-router 4.
- Backend: Rust crate `app_lib` exposing Tauri commands; SQLite (bundled) for persistence; HTTP via `reqwest` (rustls + http2 + stream + system-proxy); LLM/MCP via `rig-core` + `rmcp`.
- UI primitives: reka-ui (Radix Vue) + shadcn-vue (`new-york` style, neutral base, lucide icons), Tailwind v4, vaul, sonner, motion-v, stream-markdown, shiki, mermaid, d2, katex.
- License: MIT (2025 Joesph Falkenberg).

---

## Architecture & Data Flow

```
View (Vue SFC)  →  Store (Pinia, setup-style)  →  Service (TS)  →  Tauri Command (Rust)
```

- **UI layer**: `<script setup lang="ts">` everywhere; shared layout is `src/layout/index.vue` (`SidebarProvider` + `AppSidebar` + `<RouterView/>`); toast notifications come from `vue-sonner` mounted in `src/App.vue`.
- **Store layer**: Pinia setup stores in `src/stores/`. `useApiClientStore` is a 1kLoC factory (`createApiClientStore(options?)`) that accepts an injectable service surface so it can be tested without Tauri. State machines for execution and AI generation live here with their own `AbortController`s.
- **Service layer**: `src/services/**` is the **only** place that calls `@tauri-apps/api/core::invoke<T>(cmd, args)`. Mappers (`services/api-client/mappers.ts`, inline in `agent-conversation.ts`) translate between Tauri snake_case DTOs and frontend camelCase types.
- **IPC / Tauri**: Tauri 2 commands; payloads use **snake_case**; responses are either passed through or mapped. Cancellation: store creates `AbortController` + `executionId` / `taskId`, fires `service.cancelApiRequest(id)` (best-effort).
- **Persistence**: Only `useAppStore.settings` (`theme` + `themeColor`) is persisted via `pinia-plugin-persistedstate` under key `eidolon-app-settings`. Conversations live in Tauri/SQLite; legacy localStorage conversation data is migrated once by `services/agent-profile-storage.ts`.

Key modules (frontend ↔ backend):

| Frontend module                                | Backend command(s)                                       |
|------------------------------------------------|----------------------------------------------------------|
| `src/services/api-client/*`                    | `commands/api_client.rs`, `api_request.rs`, `api_generate.rs`, `codegen.rs`, `test_connection.rs` |
| `src/services/agent-conversation.ts`           | `commands/agent_conversation.rs`                         |
| `src/services/agent-profile-storage.ts`        | `commands/agent_profile.rs`                              |
| `src/services/provider_config.ts`              | `commands/model_config.rs`, `default_model.rs`, `test_connection.rs` |
| `src/services/mcp_service.ts`                  | `commands/mcp_service.rs`, `services/mcp_service.rs`     |
| `src/services/codegen.ts`                      | `commands/codegen.rs`, `services/codegen/*`              |
| `src/services/project-files.ts`                | `commands/app_paths.rs`, `services/work_directory.rs`    |

---

## Key Directories

| Path                              | Purpose                                                                                      |
|-----------------------------------|----------------------------------------------------------------------------------------------|
| `src/main.ts`                     | App entry: creates the Vue app, installs Pinia + router, mounts `#app`.                      |
| `src/App.vue`                     | Root component: `<RouterView/>` + `<Toaster/>` from vue-sonner.                              |
| `src/router/index.ts`             | Routes (see "Routing" below); `/` → `/agent`.                                                |
| `src/layout/`                     | App shell: `index.vue` + `app-sidebar/{logo,menu,recent-conversations,footer}`.              |
| `src/views/`                      | Feature pages (one folder per route). Co-located `components/`, `__tests__/`, `utils/`.      |
| `src/stores/`                     | Pinia setup stores. `api-client.ts` is the largest; `agent-workspace.ts`; `app.ts`.           |
| `src/services/`                   | ONLY place that calls `invoke()`. Sub-folders per feature (`api-client/`, …).                |
| `src/composables/`                | Auto-imported hooks (`useConfirm`, `useTheme`, `useAppPaths`).                               |
| `src/components/`                 | `ui/*` (shadcn-vue), `ai-elements/*` (registry copy), `sag/*` (in-house), `ConfirmDialog.vue`.|
| `src/types/`                      | Hand-written DTOs + generated `auto-import.d.ts`, `auto-import-components.d.ts`.             |
| `src/enum/`                       | Enums auto-imported by `unplugin-auto-import`.                                               |
| `src/utils/`, `src/lib/`          | Pure helpers (`theme`, `crypto`, `helpers`, `cn()` in `lib/utils.ts`).                       |
| `src/config/provider-registry.ts` | Static `PROVIDER_REGISTRY` (minimax, volcengine, deepseek, ollama).                           |
| `src-tauri/src/commands/`         | One file per command family; surfaced through `tauri::generate_handler!` in `lib.rs`.        |
| `src-tauri/src/services/`         | Backend business logic (HTTP, codegen, MCP, work directory).                                 |
| `src-tauri/src/db/`               | SQLite via `rusqlite` + `repositories/` + SQL migrations under `db/migrations/`.             |
| `src-tauri/src/models/`           | Rust DTOs (snake_case). Frontend mirrors them in `src/types/` + `services/api-client/types.ts`. |
| `src-tauri/templates/`            | Tera templates used by Rust codegen (not user-runnable).                                     |
| `src-tauri/capabilities/default.json` | Tauri 2 permissions for the default window.                                              |
| `docs/`                           | Design + plan docs (`api-client-design.md`, `api-client-implementation-plan.md`).            |

Routing (frontend):

| Path                              | View                                  |
|-----------------------------------|---------------------------------------|
| `/agent`                          | `views/agent/index.vue` (profile list)|
| `/agent/new`                      | `views/agent/create.vue`              |
| `/agent/:id/edit`                 | `views/agent/edit.vue`                |
| `/agent/:id`                      | redirect → `/agent/workspace?agent=<id>` |
| `/agent/workspace`                | `views/workspace/index.vue`           |
| `/index`                          | `views/mail/index.vue` (sample view)  |
| `/codegen`                        | `views/codegen/index.vue`             |
| `/api-client`                     | `views/api-project/index.vue`         |
| `/api-client/projects/:id`        | `views/api-client-workspace/index.vue` (name `api-client-project`) |
| `/app-setting`                    | `views/app-setting/index.vue`         |
| `/:pathMatch(.*)*`                | `pages/errors/404.vue`                |

---

## Development Commands

> Package manager: **pnpm@10.12.4** (`pnpm-lock.yaml` only — do not introduce npm/yarn lockfiles).

```bash
pnpm install                # install deps (pnpm only)
pnpm dev                    # vite dev server (http://127.0.0.1:59415, strictPort)
pnpm build                  # vue-tsc -b && vite build
pnpm preview                # vite preview (serves dist/)
pnpm tauri                  # delegate to @tauri-apps/cli (dev / build / info)
pnpm test                   # vitest run (one-shot, node env)
pnpm exec vitest            # watch mode
pnpm lint:eslint            # eslint "src/**/*.{vue,ts,tsx}" --fix (ESLint cache → node_modules/.cache/eslint/)
pnpm lint:ui                # eslint src/components/ui/**/*.{vue,ts,tsx} --fix (shadcn-vue generated)
pnpm lint:ui2               # eslint src/components/ai-elements/**/*.{vue,ts,tsx} --fix
pnpm lint:lint-staged       # runs lint-staged (pre-commit hook wiring)
```

Tauri dev runs `pnpm dev` first (`beforeDevCommand` in `tauri.conf.json`), then opens the native window. Tauri build runs `pnpm build` first (`beforeBuildCommand`).

There is **no Prettier**; formatting comes from `@antfu/eslint-config` (2-space indent, single quotes, semicolons, jsx on, stylistic on).

There is **no coverage tool installed**. Add `@vitest/coverage-v8` or `@vitest/coverage-istanbul` if you need it.

---

## Code Conventions & Common Patterns

**File / directory naming**

- Vue files: kebab-case (e.g. `api-request-editor.vue`).
- TS files: camelCase.
- Subdirectories prefixed with `_` (e.g. `app-setting/_components/`) are structural/non-routable.

**Vue 3 SFC**

- Always `<script setup lang="ts">`. **No Options API** in `src/views/`.
- Props: `defineProps<Props>()` typed object. Emits: `defineEmits<{ (e: 'name', v: Type): void }>()`.

**Pinia stores**

- Setup-style: `defineStore('id', () => { ... })`.
- Export a factory + `useXStore` when the store has non-trivial state — tests inject the factory with fakes (`createApiClientStore({ service: fake })`).
- Dirty tracking: `lastSavedRequestSnapshot` + `diffDraftAgainstSaved` controls `isDirty`; navigation refuses to drop unsaved changes without explicit `discardUnsaved`.

**Services / IPC**

- Only `src/services/**` calls `invoke()`. Stores never do.
- Tauri argument keys are snake_case; TS function names are camelCase.
- Every backend DTO has a `TauriXxx` type in `services/api-client/types.ts` (or inline) and a `toFrontend*` / `toTauri*` mapper.
- Cancellation: pair every long-running command with a `cancel*` command (`cancel_api_request`, abort via JS `AbortController` before the Tauri-side abort propagates).

**Composables**

- `useConfirm()` is a **module-scoped reactive singleton** (no `provide`/`inject`). Mount `<ConfirmDialog :open="confirm.state.open" @update:open="confirm.onOpenChange" @confirm="confirm.onConfirm" @cancel="confirm.onCancel" />` once in the app shell.
- `useTheme()` is a thin pass-through over `useAppStore`.

**Error handling**

- Funnel failures into `toast.error(getErrorMessage(err, fallback))`; `getErrorMessage` handles `Error | string | object` shapes.
- Destructive actions go through `useConfirm()` (or `sag-confirm` in settings pages).
- API-client tree exposes deletion **impact summaries** (`TauriDeletionSummary`) before confirming.

**Async / cancellation**

- Stores own `AbortController`s for execution and AI generation; pass an `executionId` / `taskId` to the backend cancel command on user cancel.
- Conversational request id (`conversationRequestId` in `useAgentWorkspaceStore`) invalidates stale loads when the active conversation changes.

**Auto-imports** (`unplugin-auto-import`, `unplugin-vue-components`)

- Vue macros + `src/composables/**/*.ts` + `src/enum/**/*.ts` + `src/store/**/*.ts` are auto-imported → `src/types/auto-import.d.ts`.
- `src/components/ui/*` are auto-registered → `src/types/auto-import-components.d.ts`.
- The generated dts files are checked in but ignored by ESLint; regenerate by running dev once after adding new composables.

**Theming**

- Tailwind v4 + `data-theme-color` attribute on `<html>`; `utils/theme.ts` mutates the DOM; persistence via `useAppStore`.

**AI generation flow**

- Typed state machine on `ApiClientAiCandidate.status` (`'generating' | 'success' | 'error' | 'cancelled'`).
- Helpers `buildAiCandidate*` live in `src/views/api-client/utils/ai-helpers.ts` (pure, unit-tested).
- AI results are bound to a `requestId` (results never leak across requests).

---

## Important Files

| Purpose                         | Path                                                            |
|---------------------------------|-----------------------------------------------------------------|
| Vue entry                       | `src/main.ts`, `src/App.vue`                                    |
| Router                          | `src/router/index.ts`                                           |
| App shell                       | `src/layout/index.vue`, `src/layout/app-sidebar/index.vue`      |
| Global confirm                  | `src/composables/use-confirm.ts`, `src/components/ConfirmDialog.vue` |
| Settings store (persisted)      | `src/stores/app.ts`                                             |
| API client store (factory)      | `src/stores/api-client.ts`                                      |
| Agent workspace store           | `src/stores/agent-workspace.ts`                                 |
| API client services             | `src/services/api-client/{projects,groups,requests,environments,history,execution,ai,types,mappers,index}.ts` |
| Agent services                  | `src/services/{agent-conversation,agent-profile-storage}.ts`    |
| Provider / MCP services         | `src/services/{provider_config,mcp_service,default_model}.ts`   |
| Codegen service                 | `src/services/codegen.ts`                                       |
| Pure API-client helpers         | `src/views/api-client/utils/{request,response,ai}-helpers.ts`   |
| API client DTOs                 | `src/types/api-client/index.ts`                                 |
| Provider registry               | `src/config/provider-registry.ts`                               |
| shadcn-vue config               | `components.json`                                               |
| Tauri config                    | `src-tauri/tauri.conf.json`                                     |
| Rust entry / lib                | `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`                 |
| Tauri commands                  | `src-tauri/src/commands/*.rs`                                   |
| Tauri capabilities              | `src-tauri/capabilities/default.json`                           |
| Design docs                     | `docs/api-client-design.md`, `docs/api-client-implementation-plan.md` |

---

## Runtime / Tooling Preferences

- **Package manager**: pnpm only. Do not commit `package-lock.json` or `yarn.lock`.
- **Runtime targets**: Node 20+ for tooling, Rust `1.77.2+` (declared in `src-tauri/Cargo.toml`), Tauri 2.x CLI. The frontend build assumes **Tauri 2 only** (`@tauri-apps/api ^2.10`, plugins v2).
- **Vite dev server**: bound to `127.0.0.1:59415` with `strictPort: true` (matches `devUrl` in `tauri.conf.json`).
- **TS path alias**: `@/*` → `./src/*` (identical in `tsconfig.app.json`, `vite.config.ts`, `vitest.config.ts`).
- **Strict TS flags enabled**: `strict`, `noUnusedLocals`, `noUnusedParameters`, `noUncheckedIndexedAccess`, `noFallthroughCasesInSwitch`, `noUncheckedSideEffectImports`.
- **Window**: default `800x600`, resizable, not fullscreen, CSP `null` (permissive — keep it permissive or update both `tauri.conf.json` and capability files).
- **Cross-platform**: bundle targets `all`; icons include `.icns` and `.ico`. There is no `target_os`-gated Rust code in the current surface.
- **External services**: Tauri uses `system-proxy` for HTTP; AI providers are configured per provider via `PROVIDER_REGISTRY`; MCP via `rmcp` (child-process + streamable-http transports).
- **Editor**: VS Code; only `Vue.volar` is recommended. `formatOnSave` is on; ESLint flat config fixes on save (`source.fixAll.eslint`).
- **Agent tooling**: `.gitignore` excludes `.specstory/` and `.cursorindexingignore` (intentional).
- **MCP for shadcn-vue**: configured in `opencode.json` (server `shadcnVue`, command `npx shadcn-vue@latest mcp`).

---

## Testing & QA

**Framework**: Vitest ^1.6 in **node env** (no jsdom). DOM-touching code is currently untested in CI; keep new tests focused on pure modules.

**Locations** (enforced glob `src/**/__tests__/**/*.test.ts`):

- `src/composables/__tests__/use-confirm.test.ts`
- `src/views/api-client/__tests__/store.test.ts`
- `src/views/api-client/utils/__tests__/request-helpers.test.ts`
- `src/views/api-client/utils/__tests__/response-helpers.test.ts`
- `src/views/api-client/utils/__tests__/ai-helpers.test.ts`

**Rules** (project-wide):

- Tests live in `__tests__/` directories. `*.spec.ts` files outside that glob are **not** picked up.
- No setup file is configured; tests bootstrap their own mocks.
- Testable seams are intentional: `createApiClientStore({ service: fakeService })` lets you swap the Tauri surface for an in-memory fake. Follow this pattern for any new store that talks to the backend.
- Pure helpers in `src/views/api-client/utils/` and `src/utils/` are the easiest place to add coverage.
- Rust side has no Vitest-equivalent test harness configured; integration tests belong in `src-tauri/src` (`#[cfg(test)]` modules) or `src-tauri/tests/`.

**Run before yielding**:

```bash
pnpm exec vue-tsc -b        # type-check (matches the gate run by `pnpm build`)
pnpm test                   # one-shot vitest
pnpm lint:eslint            # lint src/**
```

**Manual verification surface** (no browser harness wired up): launch `pnpm tauri dev`, exercise the changed flow in the native window, and confirm behavior. The API client, agent workspace, codegen, and provider settings are all visible there.

**Coverage**: not measured. Add `@vitest/coverage-v8` if a coverage gate is introduced.

**Verification split** (project rule):

- The assistant **runs** headless verification itself: `pnpm exec vue-tsc -b`, `pnpm test`, `pnpm lint:eslint`, targeted Vitest specs, build, Rust `cargo check` / `cargo test` when backend is touched. No need to ask.
- The user **runs** all visual / interactive verification: launching `pnpm dev`, `pnpm tauri dev`, `pnpm preview`, or any browser/desktop UI. The assistant does **not** start dev servers or open the native window on the user's behalf.

## Known Quirks (worth knowing before editing)

- `src/views/agent/detail.vue` is legacy (localStorage conversations + setTimeout mock replies); new work belongs in `src/views/workspace/index.vue` and `src/stores/agent-workspace.ts`.
- `src/composables/user-commits.ts` is a stub (`fetchGitLog` returns `[]`); still auto-imported — harmless but don't rely on it.
- `src/types/auto-import.d.ts` references composables that don't yet exist on disk (`useMessageSender`, `useRepositories`, `useSettings`); they are inert until you actually create those files.
- `tsconfig.app.json` excludes `src/components/ui/drawer/**/*.vue` (Vaul drawer has a known typing issue — leave excluded unless you fix the upstream types).
- API client hard constraints (from `docs/api-client-implementation-plan.md`): requests **must** go through Rust (never WebView); credentials are **masked**, not encrypted; responses cap at **5 MiB**, histories at **1 MiB** with last 100 retained.
