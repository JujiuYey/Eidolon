---
title: 接口请求工具执行计划
status: proposed
created: 2026-09-08
origin: docs/api-client-design.md
---

# 接口请求工具执行计划

## 1. 目标与边界

本计划把 `docs/api-client-design.md` 中的首版行为拆成可实施的工程单元。目标是在现有 Vue + Tauri 应用中增加“接口请求”页面，使用 Rust 端 SQLite 仓库保存请求数据，由 Rust 端执行 HTTP 请求，并复用现有模型配置完成可撤销的 JSON 请求体生成。

本次实现不迁移现有 JSON 数据，不把 SQLite 暴露给前端，不在 WebView 直接请求目标接口，也不实现设计文档列出的暂缓能力（multipart、WebSocket、SSE、GraphQL、OAuth、脚本、批量执行、断言、团队同步、OpenAPI/cURL 导入导出及 AI 连续调用）。

## 2. 当前代码约束

- 路由集中在 `src/router/index.ts`，应用导航集中在 `src/layout/app-sidebar/menu/index.vue`。
- 前端服务通过 `@tauri-apps/api/core` 的 `invoke` 调用 Rust command；现有转换和错误处理模式可参考 `src/services/agent-conversation.ts`、`src/services/provider_config.ts`。
- Rust 入口和 command 注册在 `src-tauri/src/lib.rs`、`src-tauri/src/commands/mod.rs`；现有持久化由 `src-tauri/src/db/local_store.rs` 和 repositories 负责。
- 模型配置与聊天调用分别参考 `src-tauri/src/commands/model_config.rs`、`src-tauri/src/commands/default_model.rs`、`src-tauri/src/commands/conversation.rs`。请求体生成应使用独立默认模型键，不能复用或改写聊天默认模型语义。
- 当前 `src-tauri/Cargo.toml` 没有 SQLite 和通用 HTTP 客户端依赖；驱动/ORM、HTTP 客户端及取消机制需要在实现阶段先做小范围可行性验证，再锁定依赖。

## 3. 关键决策

1. **边界分层**：Vue 只维护编辑草稿、候选 JSON、显示状态和用户确认；Rust 负责输入校验、变量解析、请求构建、HTTP 执行、响应上限和 SQLite 事务。
2. **四种状态分离**：请求定义、编辑草稿、AI 候选、执行快照分别建模。保存只写请求定义；应用 AI 只替换草稿；发送冻结草稿和环境；迟到结果按请求/执行 ID 丢弃。
3. **列表字段不用对象表示**：Query、Headers、表单行均保存为带 `enabled` 和顺序的数组，允许重复键。数据库中的灵活部分用 JSON 文本保存，关联、排序、时间等字段独立列出。
4. **凭据最小暴露**：AI 上下文默认不含 Headers、环境变量值或历史响应；历史请求和认证头脱敏。首版明文落盘的敏感环境变量必须在 UI 中明确提示“遮罩不等于加密”。
5. **HTTP 语义固定**：仅 HTTP/HTTPS；默认不重试、不自动跟随重定向、不持久化 Cookie；4xx/5xx 是正常响应，连接失败/超时/取消是执行错误；响应读取设上限，历史响应另设更小上限。
6. **迁移可追溯**：SQLite 迁移递增、事务化且不修改已执行迁移；项目、分组、请求及历史的级联删除必须在单一事务中完成。

## 4. 实施单元与顺序

### U1：数据模型、SQLite 初始化与迁移

**范围**：`src-tauri/Cargo.toml`、`src-tauri/src/db/`、`src-tauri/src/models/`、`src-tauri/src/lib.rs`。

新增 API Client 专用数据库状态、连接初始化和 `schema_migrations`。在应用数据目录创建 `api-client.sqlite`，启用外键，建立 `api_projects`、`api_groups`、`api_requests`、`api_environments`、`api_request_histories` 及必要索引。定义 Rust 序列化模型和统一错误边界；不改变 `LocalJsonStore` 及既有 JSON 文件。

**测试文件与场景**：`src-tauri/src/db/api_client.rs` 或对应迁移模块的单元测试；验证首次建库、重复启动幂等、外键约束、迁移失败回滚、项目/分组/请求删除的级联事务、最近 100 条历史清理策略。

### U2：项目、分组、请求、环境仓库与 commands

**范围**：`src-tauri/src/commands/api_client.rs`、`src-tauri/src/db/repositories/api_*.rs`、`src-tauri/src/models/api_client.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`。

实现项目/分组/请求/环境的增删改查、排序和移动；创建项目自动创建默认分组；校验请求与环境属于同一项目；保存请求定义时保留 AI 生成要求与参考内容。删除操作返回可供 UI 展示的关联数量或明确错误。command 输入输出使用稳定的 snake_case Rust DTO，前端服务层负责 camelCase 映射。

**测试文件与场景**：`src-tauri/src/commands/api_client.rs` 或 repositories 对应测试；覆盖默认分组、跨项目移动拒绝、重复排序稳定性、空名称拒绝、删除级联、环境归属校验、保存后重新读取字段完整性。

### U3：前端请求工作区与编辑状态

**范围**：`src/router/index.ts`、`src/layout/app-sidebar/menu/index.vue`、`src/views/api-client/index.vue` 及其组件目录、`src/services/api-client.ts`、`src/stores/api-client.ts`、`src/types/api-client.ts`。

增加路由和导航入口，构建项目→分组→请求的单层工作区。实现请求名称、方法、URL、Params、Headers、Body、环境选择、保存/发送入口；键值行使用数组并支持启用开关、重复键和顺序。切换请求/项目或离开页面时实现保存、放弃、继续编辑决策；未保存发送不覆盖已保存定义。

**测试文件与场景**：`src/stores/api-client.test.ts`、必要的纯函数测试文件；覆盖草稿初始化、切换请求时的未保存分支、发送不写入定义、重复键保序、粘贴完整 URL 后 Query 拆分、禁用行不进入快照、空项目/分组入口。

### U3A：Web 端交付面与交互细节

**范围**：`src/views/api-client/index.vue`、`src/views/api-client/components/`、`src/services/api-client.ts`、`src/stores/api-client.ts`、`src/types/api-client.ts`、`src/router/index.ts`、`src/layout/app-sidebar/menu/index.vue`。

Web 端按现有布局和 shadcn-vue UI 组件组织为三列/分区工作区：左侧项目、分组、请求树与搜索；中间请求编辑器；底部或右侧响应与历史；AI 生成使用可收起面板。组件边界至少包括：

- `ApiClientPage`：加载项目、维护当前项目/请求和页面级未保存离开守卫。
- `ApiClientTree`：项目/分组/请求列表、新建、重命名、删除、排序和移动入口。
- `ApiRequestEditor`：名称、环境、方法、URL、保存/发送/取消，Params、Headers、Body 标签页。
- `KeyValueEditor`：启用开关、键、值、重复行、拖动或按钮排序、空行处理。
- `BodyEditor`：无请求体、JSON、原始文本、URL 编码表单；JSON 编辑区提供语法错误定位。
- `ApiResponsePanel`：执行中、正常响应、HTTP 错误、网络错误、超时、取消、超限和二进制提示；JSON 格式化与原文切换。
- `RequestHistoryPanel`：历史列表、快照摘要、恢复、清空和脱敏/截断标记。
- `AiBodyGeneratorPanel`：生成要求、参考内容、当前 Body 勾选、模型选择、生成/取消、候选编辑、校验、应用和撤销。

状态管理明确区分加载状态、保存状态、发送状态、生成状态、当前请求 ID 和任务 ID。所有异步回调先校验 ID 再写入 store；切换请求时清理不再属于当前请求的响应和候选。错误统一转为用户可读 toast/inline 提示，不能把 Rust 原始调试堆栈直接作为唯一反馈。窄窗口下允许响应区域折叠，但不能隐藏发送取消、未保存提示和错误信息。

**Web 端测试文件与场景**：`src/views/api-client/components/*.test.ts`、`src/stores/api-client.test.ts`、`src/stores/api-client-ai.test.ts`、必要的 URL/变量/脱敏纯函数测试。覆盖路由和导航高亮、空状态入口、项目树选择、键值行增删排序和重复键、Body 类型切换、未保存离开决策、保存/发送/取消按钮状态、响应状态分类、JSON 格式化失败回退、历史恢复不自动发送、AI 候选不自动应用、任务竞态和 5 MiB/1 MiB 文案标记。若仓库尚未引入组件测试运行器，实现前先确认测试工具选型；在此之前至少保证 store 和纯函数测试可独立运行。

### U4：Rust HTTP 执行管线与响应展示

**范围**：`src-tauri/src/services/api_http.rs`、`src-tauri/src/commands/api_request.rs`、相关模型/command 注册；前端 `src/services/api-client.ts`、`src/stores/api-client.ts`、响应与历史组件。

先验证所选 HTTP 客户端的异步取消、超时、重定向策略和流式读取上限，再实现 URL/Query/Header/Body 构建。变量解析只接受 HTTP/HTTPS，缺失变量阻止发送；JSON Body 替换变量后校验语法。每次执行使用独立 ID，支持取消；读取响应最多 5 MiB，历史最多保存 1 MiB并标记截断。响应 UI 展示状态码、耗时、大小、Headers、原文，并为合法 JSON 提供格式化视图；二进制只展示类型和大小。

**测试文件与场景**：`src-tauri/src/services/api_http.rs`、`src-tauri/src/commands/api_request.rs` 的测试；使用本地测试服务器或可注入 transport 覆盖 GET/POST JSON、重复 Query/Header、缺失变量、无效 JSON、4xx/5xx、连接失败、超时、取消、3xx 不跟随、响应大小上限、二进制响应。前端测试覆盖执行 ID 关联、迟到结果丢弃和响应格式化降级。

### U5：AI 请求体生成

**范围**：`src-tauri/src/commands/api_generate.rs`、必要的 AI service/model 文件、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`；前端 `src/services/api-client.ts`、`src/stores/api-client.ts`、AI 面板组件。

复用现有 provider/model 配置解析模式，新增请求体生成默认模型键和独立 command。上下文只包含用户勾选的当前 Body、生成要求、参考内容及有限的请求元数据；不带入凭据和响应历史。生成结果先作为候选，必须通过 JSON 语法校验才可应用；应用整体替换 Body 并提供一次撤销，原 Body 保持不变直到用户明确应用。绑定请求 ID 和生成任务 ID，取消或切换请求后迟到结果不得覆盖候选。

**测试文件与场景**：`src-tauri/src/commands/api_generate.rs` 的目标解析、上下文组装和结果提取测试；前端 `src/stores/api-client-ai.test.ts` 覆盖候选不自动应用、对象/数组/标量 JSON、无效 JSON禁止应用、取消/失败保留原内容、切换请求丢弃结果、应用后可撤销且标记未保存、未配置模型的引导错误。

### U6：历史快照、脱敏与恢复

**范围**：沿用 U1/U2 的 history repository 和 U4 的执行 command；前端历史列表/恢复组件及服务类型。

执行完成后写入实际请求快照、环境名称、响应元数据、脱敏后的请求/Headers、受限响应体、截断标记、耗时和执行状态。敏感变量与常见认证头不保存实际值；恢复历史只更新编辑器草稿，重新使用当前环境凭据解析，显示“不完整原始报文”标记，不自动发送。提供清空历史。

**测试文件与场景**：历史仓库测试覆盖脱敏、截断、最近 100 条、写入失败不吞掉已收到响应；前端测试覆盖恢复不自动发送、环境切换不改历史、历史状态和错误展示。

### U7：集成验收与文档同步

**范围**：`docs/api-client-design.md` 仅在实现完成后按实际行为更新；本执行计划保留为审批记录。

完成静态检查、Rust 测试和前端构建后，按设计文档三阶段验收：保存重启闭环；AI 生成/校验/应用/撤销；环境变量、取消/超时、响应上限、脱敏历史和恢复。任何未实现或因依赖限制调整的行为都回写设计文档和变更说明。

## 5. 依赖与闸门

1. U1 开始前锁定 SQLite 方案：优先选择能在当前 Tauri 构建目标稳定编译、支持事务和外键的驱动；不要因此迁移现有存储层。
2. U4 开始前用最小实验确认取消和响应流上限；若底层客户端无法安全中止读取，调整实现并在验收中明确取消语义。
3. U5 开始前确认现有 `rig-core` 调用可抽取为共享配置解析，但不改变现有聊天 command 的行为。
4. U3/U3A 与 U4 可并行设计，但集成前必须冻结跨端 DTO、错误码/错误文本和执行状态枚举。
5. U6 依赖 U1、U2、U4；U7 依赖全部单元完成。

## 6. 验收清单

- 新建项目后存在默认分组；项目、分组、请求、环境在重启后仍可读取。
- 请求定义、草稿、AI 候选、执行快照互不越权覆盖；未保存发送不会隐式保存。
- URL、Params、Headers、Body 类型和变量规则按设计工作，重复键与禁用行保持正确语义。
- Rust 端执行 HTTP，能区分正常 HTTP 错误与连接/超时/取消错误，并执行响应大小上限。
- AI 结果必须先预览和通过 JSON 语法校验，应用可撤销，迟到结果不会污染其他请求。
- 历史包含脱敏、截断和执行状态标记；恢复只更新编辑器，不自动发送。
- Web 端可从导航进入完整工作区，项目树、请求编辑器、响应面板、历史面板和 AI 面板在加载中、空状态、未保存、发送中、错误和窄窗口状态下均有可操作反馈。
- 运行 `pnpm build`、相关 Rust 单元测试及 lint/type 检查；检查结果和未运行项目在交付报告中如实记录。

## 7. 暂留实现问题

- SQLite 驱动/ORM 的具体选择、迁移目录位置和连接状态管理方式。
- HTTP 客户端的具体 crate、取消 token 与读取上限 API。
- Tauri command 是否需要事件通道传递执行进度，或由单次 command 等待结果即可满足首版。
- 请求体生成默认模型设置在现有默认模型页面中的展示位置和 key 命名；实现前需与现有设置 UI 的数据结构对齐。

这些问题属于实现前的技术验证，不改变 `docs/api-client-design.md` 已确定的产品范围和用户行为。
