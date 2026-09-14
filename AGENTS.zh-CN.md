# 仓库指南（中文版）

> 适用范围：**Eidolon** 代码库的 AI 助手实用导引（Tauri 2 + Vue 3 桌面应用）。保持精炼；只有事实变化时才扩充。
> 英文版见 `AGENTS.md`。

---

## 项目概述

Eidolon 是一个 **Tauri 2 + Vue 3 桌面客户端**（产品名 "Eidolon"，包标识 `dev.eidolon.app`，版本 `0.1.0`），包含：

- 聊天驱动的智能体（Agent）工作台
- 类 Postman 的接口（API）客户端
- CRUD 代码生成（Go 后端 + 前端）
- AI 提供商 / MCP 服务配置

**技术栈**

- 前端：Vue 3.5 SFC、Vite 7、TypeScript 5.8、Pinia 3（带 `pinia-plugin-persistedstate`）、vue-router 4
- 后端：Rust crate `app_lib` 暴露 Tauri 命令；持久化使用 SQLite（bundled）；HTTP 使用 `reqwest`（rustls + http2 + stream + system-proxy）；LLM/MCP 使用 `rig-core` + `rmcp`
- UI 组件：reka-ui（Radix Vue）+ shadcn-vue（`new-york` 风格、neutral 基础色、lucide 图标）、Tailwind v4、vaul、sonner、motion-v、stream-markdown、shiki、mermaid、d2、katex
- 许可证：MIT（2025 Joesph Falkenberg）

---

## 架构与数据流

```
视图（Vue SFC） → 状态层（Pinia，setup 风格） → 服务层（TS） → Tauri 命令（Rust）
```

- **UI 层**：全部使用 `<script setup lang="ts">`；统一布局在 `src/layout/index.vue`（`SidebarProvider` + `AppSidebar` + `<RouterView/>`）；toast 通知来自 `vue-sonner`，挂载在 `src/App.vue`。
- **状态层**：Pinia setup 风格的 store 位于 `src/stores/`。`useApiClientStore` 是一个 1000 行的工厂（`createApiClientStore(options?)`），可以注入服务层以便在没有 Tauri 的情况下测试。执行与 AI 生成的状态机也在这里，各自带 `AbortController`。
- **服务层**：`src/services/**` 是**唯一**调用 `@tauri-apps/api/core::invoke<T>(cmd, args)` 的地方。映射器（`services/api-client/mappers.ts`、`agent-conversation.ts` 内联部分）在 Tauri snake_case DTO 与前端 camelCase 类型之间做转换。
- **IPC / Tauri**：Tauri 2 命令；请求参数使用 **snake_case**；响应直接透传或经映射。取消机制：store 创建 `AbortController` 与 `executionId` / `taskId`，调用 `service.cancelApiRequest(id)`（尽力而为）。
- **持久化**：只有 `useAppStore.settings`（`theme` + `themeColor`）通过 `pinia-plugin-persistedstate` 持久化到 `eidolon-app-settings`。会话存储在 Tauri/SQLite；旧版 localStorage 会话数据由 `services/agent-profile-storage.ts` 一次性迁移。

**前后端对应关系**

| 前端模块                              | 后端命令                                              |
|---------------------------------------|-------------------------------------------------------|
| `src/services/api-client/*`           | `commands/api_client.rs`、`api_request.rs`、`api_generate.rs`、`codegen.rs`、`test_connection.rs` |
| `src/services/agent-conversation.ts`  | `commands/agent_conversation.rs`                      |
| `src/services/agent-profile-storage.ts` | `commands/agent_profile.rs`                         |
| `src/services/provider_config.ts`     | `commands/model_config.rs`、`default_model.rs`、`test_connection.rs` |
| `src/services/mcp_service.ts`         | `commands/mcp_service.rs`、`services/mcp_service.rs`   |
| `src/services/codegen.ts`             | `commands/codegen.rs`、`services/codegen/*`           |
| `src/services/project-files.ts`       | `commands/app_paths.rs`、`services/work_directory.rs` |

---

## 关键目录

| 路径                              | 用途                                                                          |
|-----------------------------------|-------------------------------------------------------------------------------|
| `src/main.ts`                     | 应用入口：创建 Vue 应用、安装 Pinia 与 router、挂载 `#app`。                  |
| `src/App.vue`                     | 根组件：`<RouterView/>` + vue-sonner 的 `<Toaster/>`。                        |
| `src/router/index.ts`             | 路由（见下表），`/` → `/agent`。                                              |
| `src/layout/`                     | 应用骨架：`index.vue` + `app-sidebar/{logo,menu,recent-conversations,footer}`。|
| `src/views/`                      | 业务页面（一个路由一个文件夹），内含 `components/`、`__tests__/`、`utils/`。 |
| `src/stores/`                     | Pinia setup 风格 store。`api-client.ts` 最大；另有 `agent-workspace.ts`、`app.ts`。|
| `src/services/`                   | **唯一**调用 `invoke()` 的地方；按业务分子目录（`api-client/` 等）。          |
| `src/composables/`                | 自动导入的 hooks（`useConfirm`、`useTheme`、`useAppPaths`）。                 |
| `src/components/`                 | `ui/*`（shadcn-vue）、`ai-elements/*`（注册表副本）、`sag/*`（自研）、`ConfirmDialog.vue`。|
| `src/types/`                      | 手写 DTO + 自动生成的 `auto-import.d.ts`、`auto-import-components.d.ts`。      |
| `src/enum/`                       | 由 `unplugin-auto-import` 自动导入的枚举。                                   |
| `src/utils/`、`src/lib/`          | 纯辅助（`theme`、`crypto`、`helpers`，`cn()` 在 `lib/utils.ts`）。            |
| `src/config/provider-registry.ts` | 静态 `PROVIDER_REGISTRY`（minimax、volcengine、deepseek、ollama）。          |
| `src-tauri/src/commands/`         | 每个命令族一个文件，通过 `lib.rs` 中 `tauri::generate_handler!` 注册。        |
| `src-tauri/src/services/`         | 后端业务逻辑（HTTP、代码生成、MCP、工作目录）。                               |
| `src-tauri/src/db/`               | 通过 `rusqlite` 操作 SQLite，`repositories/` 与 `db/migrations/` 下的 SQL 迁移。|
| `src-tauri/src/models/`           | Rust DTO（snake_case）；前端在 `src/types/` 与 `services/api-client/types.ts` 中镜像。|
| `src-tauri/templates/`            | Rust 代码生成使用的 Tera 模板（非用户脚本）。                                 |
| `src-tauri/capabilities/default.json` | Tauri 2 默认窗口权限。                                                   |
| `docs/`                           | 设计 + 方案文档（`api-client-design.md`、`api-client-implementation-plan.md`）。|

**路由**

| 路径                              | 视图                                       |
|-----------------------------------|--------------------------------------------|
| `/agent`                          | `views/agent/index.vue`（智能体列表）      |
| `/agent/new`                      | `views/agent/create.vue`                   |
| `/agent/:id/edit`                 | `views/agent/edit.vue`                     |
| `/agent/:id`                      | 重定向到 `/agent/workspace?agent=<id>`     |
| `/agent/workspace`                | `views/workspace/index.vue`                |
| `/index`                          | `views/mail/index.vue`（示例视图）         |
| `/codegen`                        | `views/codegen/index.vue`                  |
| `/api-client`                     | `views/api-project/index.vue`              |
| `/api-client/projects/:id`        | `views/api-client-workspace/index.vue`（路由名 `api-client-project`） |
| `/app-setting`                    | `views/app-setting/index.vue`              |
| `/:pathMatch(.*)*`                | `pages/errors/404.vue`                     |

---

## 开发命令

> **包管理器**：**pnpm@10.12.4**（仅有 `pnpm-lock.yaml`，不要引入 npm/yarn lockfile）。

```bash
pnpm install                # 安装依赖（仅 pnpm）
pnpm dev                    # vite 开发服务器（http://127.0.0.1:59415，strictPort）
pnpm build                  # vue-tsc -b && vite build
pnpm preview                # vite preview（服务 dist/）
pnpm tauri                  # 转发给 @tauri-apps/cli（dev / build / info）
pnpm test                   # vitest run（单次，node 环境）
pnpm exec vitest            # watch 模式
pnpm lint:eslint            # eslint "src/**/*.{vue,ts,tsx}" --fix（缓存 → node_modules/.cache/eslint/）
pnpm lint:ui                # eslint src/components/ui/**/*.{vue,ts,tsx} --fix（shadcn-vue 生成代码）
pnpm lint:ui2               # eslint src/components/ai-elements/**/*.{vue,ts,tsx} --fix
pnpm lint:lint-staged       # 运行 lint-staged（pre-commit 钩子接线）
```

Tauri dev 会先执行 `pnpm dev`（`tauri.conf.json` 中的 `beforeDevCommand`），再打开原生窗口；Tauri build 会先执行 `pnpm build`（`beforeBuildCommand`）。

**没有 Prettier**；格式化由 `@antfu/eslint-config` 提供（2 空格缩进、单引号、分号、jsx on、stylistic on）。

**没有覆盖率工具**；如需请自行添加 `@vitest/coverage-v8` 或 `@vitest/coverage-istanbul`。

---

## 代码规范与常用模式

**文件 / 目录命名**

- Vue 文件：kebab-case（如 `api-request-editor.vue`）。
- TS 文件：camelCase。
- 以 `_` 开头的子目录（如 `app-setting/_components/`）表示结构性 / 非路由。

**Vue 3 SFC**

- 一律 `<script setup lang="ts">`。`src/views/` 中**没有** Options API。
- Props：`defineProps<Props>()` 类型化对象。Emits：`defineEmits<{ (e: 'name', v: Type): void }>()`。

**Pinia store**

- setup 风格：`defineStore('id', () => { ... })`。
- 当 store 状态复杂时，同时导出工厂 + `useXStore`，测试通过工厂注入假实现（如 `createApiClientStore({ service: fake })`）。
- 脏数据跟踪：`lastSavedRequestSnapshot` + `diffDraftAgainstSaved` 控制 `isDirty`；未保存的更改必须显式 `discardUnsaved` 才能切换。

**服务层 / IPC**

- 只有 `src/services/**` 调用 `invoke()`。store 不要直接调用。
- Tauri 参数键使用 snake_case；TS 函数名使用 camelCase。
- 每个后端 DTO 在 `services/api-client/types.ts`（或内联）都有 `TauriXxx` 类型，并配有 `toFrontend*` / `toTauri*` 映射。
- 取消机制：每个长命令都配对一个 `cancel*` 命令（`cancel_api_request`），先在前端 `AbortController` 触发，再尝试在 Tauri 端中止。

**Composables**

- `useConfirm()` 是**模块级响应式单例**（不依赖 `provide` / `inject`）。在应用根挂一次：`<ConfirmDialog :open="confirm.state.open" @update:open="confirm.onOpenChange" @confirm="confirm.onConfirm" @cancel="confirm.onCancel" />`。
- `useTheme()` 是对 `useAppStore` 的薄封装。

**错误处理**

- 失败一律走 `toast.error(getErrorMessage(err, fallback))`；`getErrorMessage` 处理 `Error | string | object`。
- 危险操作统一走 `useConfirm()`（设置页用 `sag-confirm`）。
- 接口客户端的树形删除会先展示**影响摘要**（`TauriDeletionSummary`）。

**异步 / 取消**

- store 持有执行与 AI 生成各自的 `AbortController`，并把 `executionId` / `taskId` 传给后端取消命令。
- `useAgentWorkspaceStore` 用 `conversationRequestId` 标记请求，使旧的会话加载失效。

**自动导入**（`unplugin-auto-import`、`unplugin-vue-components`）

- Vue 宏 + `src/composables/**/*.ts` + `src/enum/**/*.ts` + `src/store/**/*.ts` 自动导入 → `src/types/auto-import.d.ts`。
- `src/components/ui/*` 自动注册 → `src/types/auto-import-components.d.ts`。
- 生成出的 dts 已纳入版本控制但被 ESLint 忽略；新增 composable 后跑一次 dev 即可重新生成。

**主题**

- Tailwind v4 + `<html>` 上的 `data-theme-color` 属性；`utils/theme.ts` 改 DOM；通过 `useAppStore` 持久化。

**AI 生成流程**

- 类型化状态机：`ApiClientAiCandidate.status` 取值 `'generating' | 'success' | 'error' | 'cancelled'`。
- 辅助函数 `buildAiCandidate*` 在 `src/views/api-client/utils/ai-helpers.ts`（纯函数，已单元测试）。
- AI 结果绑定 `requestId`（不会跨请求泄露）。

---

## 重要文件

| 用途                          | 路径                                                                                          |
|-------------------------------|-----------------------------------------------------------------------------------------------|
| Vue 入口                      | `src/main.ts`、`src/App.vue`                                                                  |
| 路由                          | `src/router/index.ts`                                                                         |
| 应用骨架                      | `src/layout/index.vue`、`src/layout/app-sidebar/index.vue`                                    |
| 全局确认弹窗                  | `src/composables/use-confirm.ts`、`src/components/ConfirmDialog.vue`                          |
| 设置 store（已持久化）        | `src/stores/app.ts`                                                                           |
| 接口客户端 store（工厂）      | `src/stores/api-client.ts`                                                                    |
| 智能体工作台 store            | `src/stores/agent-workspace.ts`                                                               |
| 接口客户端服务                | `src/services/api-client/{projects,groups,requests,environments,history,execution,ai,types,mappers,index}.ts` |
| 智能体服务                    | `src/services/{agent-conversation,agent-profile-storage}.ts`                                  |
| 提供商 / MCP 服务             | `src/services/{provider_config,mcp_service,default_model}.ts`                                 |
| 代码生成服务                  | `src/services/codegen.ts`                                                                     |
| 纯辅助函数                    | `src/views/api-client/utils/{request,response,ai}-helpers.ts`                                 |
| 接口客户端 DTO                | `src/types/api-client/index.ts`                                                               |
| 提供商注册表                  | `src/config/provider-registry.ts`                                                             |
| shadcn-vue 配置               | `components.json`                                                                             |
| Tauri 配置                    | `src-tauri/tauri.conf.json`                                                                   |
| Rust 入口 / lib               | `src-tauri/src/main.rs`、`src-tauri/src/lib.rs`                                               |
| Tauri 命令                    | `src-tauri/src/commands/*.rs`                                                                 |
| Tauri 权限                    | `src-tauri/capabilities/default.json`                                                         |
| 设计文档                      | `docs/api-client-design.md`、`docs/api-client-implementation-plan.md`                         |

---

## 运行时 / 工具偏好

- **包管理器**：仅 pnpm。不要提交 `package-lock.json` 或 `yarn.lock`。
- **运行时版本**：Node 20+、Rust `1.77.2+`（在 `src-tauri/Cargo.toml` 中声明）、Tauri 2.x CLI。前端构建仅支持 **Tauri 2**（`@tauri-apps/api ^2.10`、插件 v2）。
- **Vite 开发服务器**：绑定 `127.0.0.1:59415`，`strictPort: true`（与 `tauri.conf.json` 中 `devUrl` 一致）。
- **TS 路径别名**：`@/*` → `./src/*`（`tsconfig.app.json`、`vite.config.ts`、`vitest.config.ts` 保持一致）。
- **TS 严格选项已开启**：`strict`、`noUnusedLocals`、`noUnusedParameters`、`noUncheckedIndexedAccess`、`noFallthroughCasesInSwitch`、`noUncheckedSideEffectImports`。
- **窗口**：默认 `800x600`、可缩放、非全屏、CSP `null`（宽松策略；如要收紧需同时改 `tauri.conf.json` 与 capability 文件）。
- **跨平台**：bundle 目标 `all`；图标包含 `.icns` 和 `.ico`。当前没有按 `target_os` 区分的 Rust 代码。
- **外部服务**：Tauri HTTP 使用 `system-proxy`；AI 提供商通过 `PROVIDER_REGISTRY` 配置；MCP 使用 `rmcp`（子进程 + streamable-http 传输）。
- **编辑器**：VS Code；仅推荐 `Vue.volar`。`formatOnSave` 开启；ESLint flat config 保存时自动修复（`source.fixAll.eslint`）。
- **Agent 工具**：`/.gitignore` 排除 `.specstory/` 与 `.cursorindexingignore`（故意）。
- **shadcn-vue MCP**：在 `opencode.json` 中配置（服务器名 `shadcnVue`，命令 `npx shadcn-vue@latest mcp`）。

---

## 测试与质量保障

**框架**：Vitest ^1.6，**node 环境**（无 jsdom）。当前 CI 不覆盖 DOM 相关代码；新写的测试建议集中在纯模块。

**位置**（glob 强制为 `src/**/__tests__/**/*.test.ts`）：

- `src/composables/__tests__/use-confirm.test.ts`
- `src/views/api-client/__tests__/store.test.ts`
- `src/views/api-client/utils/__tests__/request-helpers.test.ts`
- `src/views/api-client/utils/__tests__/response-helpers.test.ts`
- `src/views/api-client/utils/__tests__/ai-helpers.test.ts`

**规则**

- 测试放在 `__tests__/` 目录下。该 glob 之外的 `*.spec.ts` 文件**不会**被收集。
- 未配置 setup 文件；测试需自备 mock。
- 已有可测试接缝：`createApiClientStore({ service: fakeService })` 可把 Tauri 接口换成内存假实现。新写一个跟后端通信的 store 时，沿用这个模式。
- `src/views/api-client/utils/` 与 `src/utils/` 中的纯函数最容易加覆盖。
- Rust 侧没有 Vitest 等价物；集成测试放在 `src-tauri/src` 中的 `#[cfg(test)]` 模块或 `src-tauri/tests/`。

**交付前的验证**

```bash
pnpm exec vue-tsc -b        # 类型检查（与 `pnpm build` 的把关一致）
pnpm test                   # 单次跑 vitest
pnpm lint:eslint            # 校验 src/**
```

**手动验证**（未接入浏览器自动化）：启动 `pnpm tauri dev`，在原生窗口中走一遍改动路径并确认行为。接口客户端、智能体工作台、代码生成、提供商设置都能在这里看到。

**覆盖率**：未启用。若引入覆盖率门禁，请加上 `@vitest/coverage-v8`。

**验证分工**（项目约定）：

- **我来跑**：所有无头验证——`pnpm exec vue-tsc -b`、`pnpm test`、`pnpm lint:eslint`、指定 Vitest 套件、构建、涉及后端时的 `cargo check` / `cargo test`。无需你确认。
- **你来跑**：所有视觉与交互验证——启动 `pnpm dev`、`pnpm tauri dev`、`pnpm preview`，或在桌面端点 UI、看效果。我**不会**帮你起开发服务器或打开原生窗口。

---

## 已知坑（动手前要知道）

- `src/views/agent/detail.vue` 是旧实现（localStorage 会话 + setTimeout 模拟回复）；新工作请放到 `src/views/workspace/index.vue` 与 `src/stores/agent-workspace.ts`。
- `src/composables/user-commits.ts` 是空壳（`fetchGitLog` 返回 `[]`）；已被自动导入但不要依赖。
- `src/types/auto-import.d.ts` 引用了几个尚未存在的 composable（`useMessageSender`、`useRepositories`、`useSettings`）；在你真正创建这些文件前它们是无效的。
- `tsconfig.app.json` 排除了 `src/components/ui/drawer/**/*.vue`（Vaul drawer 有已知的类型问题，除非修上游类型，否则保持排除）。
- 接口客户端硬约束（来自 `docs/api-client-implementation-plan.md`）：请求**必须**走 Rust（不要走 WebView）；凭据**只做掩码**，不加密；响应上限 **5 MiB**，历史 **1 MiB**，仅保留最近 100 条。
