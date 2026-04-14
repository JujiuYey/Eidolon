# Provider Config 重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 provider 配置架构从"数据散落各处 + ProviderKey 枚举硬编码"重构为"前端 Registry 驱动 + Tauri 只存用户数据 + 支持动态拉取模型列表"。

**Architecture:** 前端维护一份 `PROVIDER_REGISTRY`（内置 provider 元数据，含 icon/defaultUrl/defaultModels），作为唯一数据源；Tauri 端将 `ProviderKey` 枚举替换为普通字符串 id，`ModelProviderSettingDoc` 去掉 catalog 嵌套（模型列表单独管理）；新增"拉取 /v1/models"Tauri command，支持用户按需刷新模型列表；UI 改为从 Tauri 读取已保存配置并合并 Registry 元数据展示。

**Tech Stack:** Vue 3 + TypeScript（前端）、Rust + Tauri v2（后端）、reqwest（HTTP 拉取模型列表）

---

## 文件改动地图

### 新建
- `src/config/provider-registry.ts` — 内置 provider 元数据注册表（唯一数据源）
- `src/services/provider_config.ts` — Tauri invoke 封装（替换旧 model_config.ts 中的 provider 相关接口）
- `src/stores/provider.ts` — Pinia store，管理 provider 配置列表 + 已选 provider

### 修改
- `src-tauri/src/models/model_config.rs` — 去掉 `ProviderKey` 枚举，改为 `provider_id: String`；去掉 `catalog`；精简结构
- `src-tauri/src/db/repositories/model_config.rs` — 适配新结构，key 改为 `provider_id`
- `src-tauri/src/commands/model_config.rs` — 新增 `fetch_provider_models` command
- `src-tauri/src/lib.rs` — 注册新 command
- `src/services/model_config.ts` — 更新 TS 类型，去掉旧字段
- `src/views/app-setting/_components/provider-config/_shared/const.ts` — 删除死代码 `AI_PROVIDERS`，改为从 registry 导出
- `src/views/app-setting/_components/provider-config/_shared/provider-icons.ts` — 简化，只保留 icon 导入映射
- `src/views/app-setting/_components/provider-config/_components/provider-list.vue` — 改为从 store 读取数据
- `src/views/app-setting/_components/provider-config/_components/provider-config-panel.vue` — 重构表单逻辑，接入 store
- `src/views/app-setting/_components/provider-config/index.vue` — 接入 store

---

## Task 1: 建立前端 Provider Registry（唯一数据源）

**Files:**
- Create: `src/config/provider-registry.ts`
- Modify: `src/views/app-setting/_components/provider-config/_shared/provider-icons.ts`

### 目标
消灭当前 provider 元数据的 5 处重复定义，用一份 registry 驱动所有地方。

- [ ] **Step 1: 新建 `src/config/provider-registry.ts`**

```typescript
import deepseekIcon from '@/assets/model-icon/deepseek.svg';
import minimaxIcon from '@/assets/model-icon/minimax.svg';
import ollamaIcon from '@/assets/model-icon/ollama.svg';
import volcengineIcon from '@/assets/model-icon/volcengine.svg';

export interface ProviderModelMeta {
  id: string;
  name: string;
  capabilities: {
    chat: boolean;
    vision: boolean;
    tool_call: boolean;
    reasoning: boolean;
    embedding: boolean;
  };
}

export interface ProviderMeta {
  /** 唯一标识，与 Tauri 存储的 provider_id 一致 */
  id: string;
  name: string;
  /** 打包进前端的 SVG icon（内置 provider 专用） */
  icon?: string;
  /** 用户看到的官网链接 */
  website: string;
  /** 默认 base_url，用户可覆盖 */
  defaultBaseUrl: string;
  /** 内置的默认模型列表，作为首次使用时的兜底 */
  defaultModels: ProviderModelMeta[];
  /** API 协议类型，决定调用方式 */
  apiType: 'openai-compatible' | 'ollama';
}

export const PROVIDER_REGISTRY: ProviderMeta[] = [
  {
    id: 'minimax',
    name: 'MiniMax',
    icon: minimaxIcon,
    website: 'https://platform.minimaxi.com/',
    defaultBaseUrl: 'https://api.minimaxi.com/v1',
    apiType: 'openai-compatible',
    defaultModels: [
      { id: 'MiniMax-M2.7', name: 'MiniMax-M2.7', capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false } },
      { id: 'MiniMax-M2.7-highspeed', name: 'MiniMax-M2.7-highspeed', capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false } },
      { id: 'MiniMax-Text-01', name: 'MiniMax-Text-01', capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false } },
    ],
  },
  {
    id: 'volcengine',
    name: '火山引擎',
    icon: volcengineIcon,
    website: 'https://www.volcengine.com/',
    defaultBaseUrl: 'https://ark.cn-beijing.volces.com/api/v3',
    apiType: 'openai-compatible',
    defaultModels: [
      { id: 'doubao-pro-32k', name: 'Doubao Pro 32K', capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false } },
      { id: 'doubao-lite-32k', name: 'Doubao Lite 32K', capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false } },
    ],
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    icon: deepseekIcon,
    website: 'https://www.deepseek.com/',
    defaultBaseUrl: 'https://api.deepseek.com/v1',
    apiType: 'openai-compatible',
    defaultModels: [
      { id: 'deepseek-chat', name: 'DeepSeek Chat', capabilities: { chat: true, vision: false, tool_call: true, reasoning: false, embedding: false } },
      { id: 'deepseek-reasoner', name: 'DeepSeek Reasoner', capabilities: { chat: true, vision: false, tool_call: false, reasoning: true, embedding: false } },
    ],
  },
  {
    id: 'ollama',
    name: 'Ollama',
    icon: ollamaIcon,
    website: 'https://ollama.com/',
    defaultBaseUrl: 'http://127.0.0.1:11434',
    apiType: 'ollama',
    defaultModels: [
      { id: 'llama3', name: 'Llama 3', capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false } },
      { id: 'qwen2', name: 'Qwen 2', capabilities: { chat: true, vision: false, tool_call: false, reasoning: false, embedding: false } },
    ],
  },
];

/** 通过 id 查找内置 provider 元数据，找不到返回 undefined */
export function findProviderMeta(id: string): ProviderMeta | undefined {
  return PROVIDER_REGISTRY.find(p => p.id === id);
}
```

- [ ] **Step 2: 简化 `src/views/app-setting/_components/provider-config/_shared/provider-icons.ts`**

将文件改为只从 registry 导出，不再重复定义：

```typescript
export { PROVIDER_REGISTRY, findProviderMeta } from '@/config/provider-registry';
export type { ProviderMeta, ProviderModelMeta } from '@/config/provider-registry';
```

- [ ] **Step 3: 删除 `src/views/app-setting/_components/provider-config/_shared/const.ts` 中的死代码**

将整个文件内容替换为空导出（`AI_PROVIDERS` 完全没有被使用，直接删除）：

```typescript
// 此文件已废弃，所有 provider 元数据统一在 @/config/provider-registry 维护
export {};
```

- [ ] **Step 4: Commit**

```bash
git add src/config/provider-registry.ts \
  src/views/app-setting/_components/provider-config/_shared/provider-icons.ts \
  src/views/app-setting/_components/provider-config/_shared/const.ts
git commit -m "refactor: introduce PROVIDER_REGISTRY as single source of truth for provider metadata"
```

---

## Task 2: 重构 Rust 端数据模型（去掉枚举，精简结构）

**Files:**
- Modify: `src-tauri/src/models/model_config.rs`
- Modify: `src-tauri/src/db/repositories/model_config.rs`

### 目标
- 用 `provider_id: String` 替换 `ProviderKey` 枚举，支持任意字符串（含用户自定义 provider）
- 去掉 `catalog`（模型列表不再存在 provider 配置里，模型由前端 registry 提供兜底，动态拉取后缓存在前端 store）
- 去掉 `selected_model_id`（这是运行时状态，不属于 provider 配置）

- [ ] **Step 1: 重写 `src-tauri/src/models/model_config.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 单个平台的用户配置文档。
/// provider_id 作为文档的 key（等价于之前的 ProviderKey::as_str()）。
/// 不再存储模型列表（由前端 registry 提供兜底，动态拉取后缓存于前端 store）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// 平台唯一标识，内置平台使用固定字符串（"minimax" 等），
    /// 用户自定义平台可使用任意非空字符串。
    pub provider_id: String,

    /// 该平台在当前应用中是否启用。
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// API 密钥，Ollama 等无需 key 的平台留空即可。
    #[serde(default)]
    pub api_key: String,

    /// 覆盖默认 base_url，留空时由前端 registry 提供默认值。
    #[serde(default)]
    pub base_url: String,

    /// 文档创建时间。
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,

    /// 文档最后更新时间。
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_true() -> bool {
    true
}
```

- [ ] **Step 2: 重写 `src-tauri/src/db/repositories/model_config.rs`**

```rust
use std::cell::RefCell;
use std::collections::HashMap;

use crate::db::local_store::{now, LocalJsonStore};
use crate::models::model_config::ProviderConfig;

const FILENAME: &str = "provider_configs";

pub struct ProviderConfigRepository<'a> {
    store: &'a LocalJsonStore,
    cache: RefCell<HashMap<String, ProviderConfig>>,
}

impl<'a> ProviderConfigRepository<'a> {
    pub fn new(store: &'a LocalJsonStore) -> Self {
        let cache = store
            .read(FILENAME)
            .unwrap_or_else(|_| HashMap::default());

        Self {
            store,
            cache: RefCell::new(cache),
        }
    }

    pub fn list(&self) -> Result<Vec<ProviderConfig>, String> {
        let cache = self.cache.borrow();
        let mut results: Vec<_> = cache.values().cloned().collect();
        results.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        Ok(results)
    }

    pub fn upsert(&self, config: &ProviderConfig) -> Result<String, String> {
        if config.provider_id.trim().is_empty() {
            return Err("provider_id 不能为空".to_string());
        }

        let key = config.provider_id.clone();
        let mut doc = config.clone();

        if doc.created_at.is_none() {
            doc.created_at = Some(now());
        }
        doc.updated_at = Some(now());

        self.cache.borrow_mut().insert(key.clone(), doc);
        self.store.write(FILENAME, &*self.cache.borrow())?;

        Ok(key)
    }

    pub fn delete(&self, provider_id: &str) -> Result<String, String> {
        if self.cache.borrow_mut().remove(provider_id).is_none() {
            return Err(format!("未找到 provider_id 为 {} 的配置", provider_id));
        }

        self.store.write(FILENAME, &*self.cache.borrow())?;
        Ok(provider_id.to_string())
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models/model_config.rs \
  src-tauri/src/db/repositories/model_config.rs
git commit -m "refactor(rust): replace ProviderKey enum with string provider_id, remove catalog from stored config"
```

---

## Task 3: 更新 Tauri Commands（新增 fetch_provider_models）

**Files:**
- Modify: `src-tauri/src/commands/model_config.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

### 目标
- CRUD commands 适配新的 `ProviderConfig` 结构
- 新增 `fetch_provider_models` command：通过 `GET /v1/models`（OpenAI 兼容）或 Ollama Tags API 拉取模型列表

- [ ] **Step 1: 重写 `src-tauri/src/commands/model_config.rs`**

```rust
use crate::db::local_store::LocalJsonStore;
use crate::db::repositories::model_config::ProviderConfigRepository;
use crate::models::model_config::ProviderConfig;
use serde::{Deserialize, Serialize};

#[tauri::command]
pub fn list_provider_configs(
    store: tauri::State<'_, LocalJsonStore>,
) -> Result<Vec<ProviderConfig>, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.list()
}

#[tauri::command]
pub fn upsert_provider_config(
    store: tauri::State<'_, LocalJsonStore>,
    config: ProviderConfig,
) -> Result<String, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.upsert(&config)
}

#[tauri::command]
pub fn delete_provider_config(
    store: tauri::State<'_, LocalJsonStore>,
    provider_id: String,
) -> Result<String, String> {
    let repo = ProviderConfigRepository::new(&store);
    repo.delete(&provider_id)
}

/// 从 OpenAI 兼容接口（GET /v1/models）拉取模型 id 列表。
/// 对 Ollama 则调用 GET /api/tags。
/// api_type 传 "ollama" 时走 Ollama 接口，其他值走 OpenAI 兼容接口。
#[tauri::command]
pub async fn fetch_provider_models(
    base_url: String,
    api_key: String,
    api_type: String,
) -> Result<Vec<String>, String> {
    if api_type == "ollama" {
        fetch_ollama_models(&base_url).await
    } else {
        fetch_openai_models(&base_url, &api_key).await
    }
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

async fn fetch_openai_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("接口返回错误: {}", resp.status()));
    }

    let body: OpenAIModelsResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let mut ids: Vec<String> = body.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    Ok(ids)
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("接口返回错误: {}", resp.status()));
    }

    let body: OllamaTagsResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let mut names: Vec<String> = body.models.into_iter().map(|m| m.name).collect();
    names.sort();
    Ok(names)
}
```

- [ ] **Step 2: 检查 `src-tauri/src/commands/mod.rs` 是否需要更新导出**

查看当前文件内容，确认 `model_config` 模块已导出。如果文件里有 `pub mod model_config;` 则无需改动。

- [ ] **Step 3: 在 `src-tauri/src/lib.rs` 注册新 commands，删除旧 commands**

找到 `.invoke_handler(tauri::generate_handler![...])` 的位置，将旧的 `model_config` commands 替换为新的：

```rust
// 替换掉：
// commands::model_config::list_model_configs,
// commands::model_config::create_model_config,
// commands::model_config::update_model_config,
// commands::model_config::delete_model_config,
// commands::model_config::set_default_config,

// 改为：
commands::model_config::list_provider_configs,
commands::model_config::upsert_provider_config,
commands::model_config::delete_provider_config,
commands::model_config::fetch_provider_models,
```

- [ ] **Step 4: 在 `src-tauri/Cargo.toml` 确认 `reqwest` 依赖存在**

运行以下命令检查：

```bash
grep "reqwest" src-tauri/Cargo.toml
```

如果没有，添加：

```toml
reqwest = { version = "0.12", features = ["json"] }
```

- [ ] **Step 5: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | head -50
```

预期：无 error（warning 可忽略）

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/model_config.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(rust): add fetch_provider_models command, refactor CRUD to use ProviderConfig"
```

---

## Task 4: 更新前端 Service 层 + 新建 Pinia Store

**Files:**
- Modify: `src/services/model_config.ts` → 重命名语义，改为 `src/services/provider_config.ts`（旧文件保留空转发避免破坏其他可能的引用）
- Create: `src/stores/provider.ts`

- [ ] **Step 1: 将 `src/services/model_config.ts` 替换为新接口**

```typescript
import { invoke } from '@tauri-apps/api/core';

/** Tauri 持久化的 provider 配置（只存用户数据，不含模型列表） */
export interface ProviderConfig {
  provider_id: string;
  enabled: boolean;
  api_key: string;
  base_url: string;
  created_at?: string;
  updated_at?: string;
}

export async function listProviderConfigs(): Promise<ProviderConfig[]> {
  return invoke<ProviderConfig[]>('list_provider_configs');
}

export async function upsertProviderConfig(config: ProviderConfig): Promise<string> {
  return invoke<string>('upsert_provider_config', { config });
}

export async function deleteProviderConfig(providerId: string): Promise<string> {
  return invoke<string>('delete_provider_config', { providerId });
}

export async function fetchProviderModels(params: {
  baseUrl: string;
  apiKey: string;
  apiType: string;
}): Promise<string[]> {
  return invoke<string[]>('fetch_provider_models', {
    baseUrl: params.baseUrl,
    apiKey: params.apiKey,
    apiType: params.apiType,
  });
}

export async function testAiConnection(request: {
  api_key: string;
  base_url: string;
  model: string;
}): Promise<void> {
  await invoke('test_ai_connection', { request });
}
```

- [ ] **Step 2: 新建 `src/stores/provider.ts`**

```typescript
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { PROVIDER_REGISTRY, findProviderMeta } from '@/config/provider-registry';
import type { ProviderMeta } from '@/config/provider-registry';
import {
  listProviderConfigs,
  upsertProviderConfig,
  deleteProviderConfig,
  fetchProviderModels,
} from '@/services/model_config';
import type { ProviderConfig } from '@/services/model_config';

/** 合并后的完整 provider 视图（元数据 + 用户配置） */
export interface ProviderView {
  /** 来自 ProviderConfig.provider_id 或 ProviderMeta.id */
  id: string;
  /** 显示名称，用户自定义 provider 可能只有 id */
  name: string;
  icon?: string;
  website?: string;
  defaultBaseUrl: string;
  /** 用户存储的配置，未保存过则为 null */
  config: ProviderConfig | null;
  /** 是否为内置 provider */
  isBuiltin: boolean;
}

export const useProviderStore = defineStore('provider', () => {
  const configs = ref<ProviderConfig[]>([]);
  const isLoading = ref(false);
  const selectedProviderId = ref<string>(PROVIDER_REGISTRY[0]?.id ?? '');

  /** 拉取过的模型列表缓存，key 为 provider_id */
  const modelListCache = ref<Record<string, string[]>>({});
  const modelListLoading = ref<Record<string, boolean>>({});

  /** 合并 registry 元数据和已存配置 */
  const providerViews = computed<ProviderView[]>(() => {
    // 内置 provider 先排列
    const builtinViews: ProviderView[] = PROVIDER_REGISTRY.map(meta => ({
      id: meta.id,
      name: meta.name,
      icon: meta.icon,
      website: meta.website,
      defaultBaseUrl: meta.defaultBaseUrl,
      config: configs.value.find(c => c.provider_id === meta.id) ?? null,
      isBuiltin: true,
    }));

    // 用户自定义 provider（不在 registry 里的）
    const customViews: ProviderView[] = configs.value
      .filter(c => !findProviderMeta(c.provider_id))
      .map(c => ({
        id: c.provider_id,
        name: c.provider_id,
        icon: undefined,
        website: undefined,
        defaultBaseUrl: c.base_url,
        config: c,
        isBuiltin: false,
      }));

    return [...builtinViews, ...customViews];
  });

  const selectedView = computed(() =>
    providerViews.value.find(v => v.id === selectedProviderId.value) ?? null,
  );

  async function loadConfigs() {
    isLoading.value = true;
    try {
      configs.value = await listProviderConfigs();
    } finally {
      isLoading.value = false;
    }
  }

  async function saveConfig(config: ProviderConfig) {
    await upsertProviderConfig(config);
    await loadConfigs();
  }

  async function removeConfig(providerId: string) {
    await deleteProviderConfig(providerId);
    await loadConfigs();
  }

  async function refreshModelList(providerId: string) {
    const view = providerViews.value.find(v => v.id === providerId);
    if (!view) return;

    const meta = findProviderMeta(providerId);
    const baseUrl = view.config?.base_url || view.defaultBaseUrl;
    const apiKey = view.config?.api_key ?? '';
    const apiType = meta?.apiType ?? 'openai-compatible';

    modelListLoading.value = { ...modelListLoading.value, [providerId]: true };
    try {
      const models = await fetchProviderModels({ baseUrl, apiKey, apiType });
      modelListCache.value = { ...modelListCache.value, [providerId]: models };
    } finally {
      modelListLoading.value = { ...modelListLoading.value, [providerId]: false };
    }
  }

  /** 获取某 provider 的模型列表：优先用拉取缓存，fallback 用 registry 默认列表 */
  function getModelList(providerId: string): string[] {
    if (modelListCache.value[providerId]) {
      return modelListCache.value[providerId];
    }
    const meta = findProviderMeta(providerId);
    return meta?.defaultModels.map(m => m.id) ?? [];
  }

  return {
    configs,
    isLoading,
    selectedProviderId,
    providerViews,
    selectedView,
    modelListCache,
    modelListLoading,
    loadConfigs,
    saveConfig,
    removeConfig,
    refreshModelList,
    getModelList,
  };
});
```

- [ ] **Step 3: Commit**

```bash
git add src/services/model_config.ts src/stores/provider.ts
git commit -m "feat(frontend): add ProviderConfig service types and provider Pinia store"
```

---

## Task 5: 重构 `provider-list.vue`（从 store 读取，展示配置状态）

**Files:**
- Modify: `src/views/app-setting/_components/provider-config/_components/provider-list.vue`

- [ ] **Step 1: 重写 `provider-list.vue`**

```vue
<script setup lang="ts">
import { onMounted } from 'vue';
import { useProviderStore } from '@/stores/provider';

const store = useProviderStore();

onMounted(() => {
  store.loadConfigs();
});
</script>

<template>
  <aside class="flex h-full w-[248px] shrink-0 flex-col border-r bg-muted/10 p-3">
    <nav class="space-y-1.5">
      <button
        v-for="view of store.providerViews"
        :key="view.id"
        type="button"
        class="flex w-full items-center gap-3 rounded-lg border px-3 py-3 text-left transition-colors"
        :class="store.selectedProviderId === view.id
          ? 'border-primary bg-primary/50 shadow-xs'
          : 'border-transparent hover:bg-primary/10'"
        @click="store.selectedProviderId = view.id"
      >
        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-background ring-1 ring-black/5">
          <img
            v-if="view.icon"
            :src="view.icon"
            :alt="view.name"
            class="h-6 w-6 object-contain"
          />
          <span
            v-else
            class="text-xs font-semibold text-muted-foreground"
          >
            {{ view.name.slice(0, 2).toUpperCase() }}
          </span>
        </div>

        <div class="min-w-0 flex-1">
          <p class="truncate text-sm font-medium text-foreground">
            {{ view.name }}
          </p>
          <!-- 已配置状态指示 -->
          <p
            v-if="view.config"
            class="truncate text-xs text-emerald-500"
          >
            已配置
          </p>
          <p
            v-else
            class="truncate text-xs text-muted-foreground"
          >
            未配置
          </p>
        </div>
      </button>
    </nav>
  </aside>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/views/app-setting/_components/provider-config/_components/provider-list.vue
git commit -m "refactor: provider-list reads from store, shows configured status"
```

---

## Task 6: 重构 `provider-config-panel.vue`（接入 store，支持动态模型列表）

**Files:**
- Modify: `src/views/app-setting/_components/provider-config/_components/provider-config-panel.vue`

### 目标
- 表单数据从 store 的 `selectedView` 派生，选择 provider 后自动填入已存配置或默认值
- 保存/删除直接调 store action
- 新增"刷新模型列表"按钮，调 `store.refreshModelList()`

- [ ] **Step 1: 重写 `provider-config-panel.vue`**

```vue
<script setup lang="ts">
import {
  Check,
  ExternalLink,
  Eye,
  EyeOff,
  RefreshCw,
  RotateCcw,
  Trash2,
} from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import SagConfirm from '@/components/sag/sag-confirm/index.vue';
import { getErrorMessage } from '@/utils/helpers';
import { useProviderStore } from '@/stores/provider';
import type { ProviderConfig } from '@/services/model_config';

const store = useProviderStore();

const showApiKey = ref(false);
const deleteConfirmOpen = ref(false);

// 表单状态：从 selectedView 派生，每次切换 provider 重置
const formData = ref<ProviderConfig>(buildFormData());

function buildFormData(): ProviderConfig {
  const view = store.selectedView;
  if (!view) {
    return { provider_id: '', enabled: true, api_key: '', base_url: '' };
  }
  if (view.config) {
    return { ...view.config };
  }
  return {
    provider_id: view.id,
    enabled: true,
    api_key: '',
    base_url: view.defaultBaseUrl,
  };
}

watch(
  () => store.selectedProviderId,
  () => {
    formData.value = buildFormData();
    showApiKey.value = false;
  },
  { immediate: true },
);

const selectedView = computed(() => store.selectedView);

const isConfigured = computed(() => Boolean(selectedView.value?.config));

const defaultBaseUrl = computed(() =>
  selectedView.value?.defaultBaseUrl ?? '',
);

const canRestoreBaseUrl = computed(() =>
  Boolean(defaultBaseUrl.value && formData.value.base_url !== defaultBaseUrl.value),
);

function restoreBaseUrl() {
  formData.value.base_url = defaultBaseUrl.value;
}

const modelList = computed(() =>
  store.getModelList(store.selectedProviderId),
);

const isModelListLoading = computed(() =>
  store.modelListLoading[store.selectedProviderId] ?? false,
);

const isFormValid = computed(() =>
  Boolean(formData.value.base_url),
);

async function saveConfig() {
  try {
    await store.saveConfig(formData.value);
    toast.success('保存成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '保存失败'));
  }
}

function openDeleteConfirm() {
  if (!isConfigured.value) return;
  deleteConfirmOpen.value = true;
}

async function confirmDelete() {
  try {
    await store.removeConfig(store.selectedProviderId);
    formData.value = buildFormData();
    deleteConfirmOpen.value = false;
    toast.success('删除成功');
  } catch (error) {
    toast.error(getErrorMessage(error, '删除失败'));
  }
}

async function refreshModels() {
  // 先保存当前 base_url 和 api_key 到 formData，确保拉取用最新值
  try {
    await store.refreshModelList(store.selectedProviderId);
    toast.success('模型列表已更新');
  } catch (error) {
    toast.error(getErrorMessage(error, '拉取模型列表失败'));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col bg-background">
    <!-- 顶部标题栏 -->
    <div class="flex items-start justify-between gap-4 border-b px-8 py-6">
      <div class="min-w-0 space-y-3">
        <div class="flex items-center gap-3">
          <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-white shadow-xs ring-1 ring-black/5">
            <img
              v-if="selectedView?.icon"
              :src="selectedView.icon"
              :alt="selectedView.name"
              class="h-7 w-7 object-contain"
            />
            <span
              v-else
              class="text-sm font-semibold text-muted-foreground"
            >
              {{ (selectedView?.name ?? '?').slice(0, 2).toUpperCase() }}
            </span>
          </div>

          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="truncate text-2xl font-semibold tracking-tight text-foreground">
                {{ selectedView?.name ?? '选择平台' }}
              </h2>
              <a
                v-if="selectedView?.website"
                :href="selectedView.website"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  class="h-8 w-8 rounded-full text-muted-foreground"
                >
                  <ExternalLink class="h-4 w-4" />
                </Button>
              </a>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-3 pt-1">
        <span class="text-sm font-medium text-muted-foreground">启用</span>
        <Switch v-model="formData.enabled" />
      </div>
    </div>

    <!-- 内容区域 -->
    <ScrollArea class="min-h-0 flex-1">
      <div class="space-y-8 px-8 py-6">
        <!-- API 地址 -->
        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 地址</Label>

          <div class="flex flex-col gap-3 md:flex-row">
            <Input
              v-model="formData.base_url"
              type="text"
              placeholder="例如: https://api.openai.com/v1"
              class="h-11 rounded-xl"
            />
            <Button
              type="button"
              variant="outline"
              class="h-11 rounded-xl px-4 text-red-500 hover:text-red-500"
              :disabled="!canRestoreBaseUrl"
              @click="restoreBaseUrl"
            >
              <RotateCcw class="h-4 w-4" />
              重置
            </Button>
          </div>
        </section>

        <!-- API 密钥 -->
        <section class="space-y-3">
          <Label class="text-base font-semibold text-foreground">API 密钥</Label>

          <div class="relative">
            <Input
              v-model="formData.api_key"
              :type="showApiKey ? 'text' : 'password'"
              placeholder="请输入 API 密钥"
              class="h-11 rounded-xl pr-11"
            />
            <button
              type="button"
              class="absolute top-1/2 right-3 flex -translate-y-1/2 items-center text-muted-foreground transition hover:text-foreground"
              @click="showApiKey = !showApiKey"
            >
              <Eye v-if="!showApiKey" class="h-4 w-4" />
              <EyeOff v-else class="h-4 w-4" />
            </button>
          </div>
        </section>

        <!-- 模型列表 -->
        <section class="space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Label class="text-base font-semibold text-foreground">模型</Label>
              <span class="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {{ modelList.length }}
              </span>
            </div>

            <Button
              type="button"
              variant="outline"
              size="sm"
              class="h-9 rounded-xl gap-2"
              :disabled="isModelListLoading"
              @click="refreshModels"
            >
              <RefreshCw
                class="h-4 w-4"
                :class="{ 'animate-spin': isModelListLoading }"
              />
              拉取模型列表
            </Button>
          </div>

          <div v-if="modelList.length > 0" class="space-y-3">
            <div
              v-for="modelId of modelList"
              :key="modelId"
              class="flex w-full items-center justify-between gap-4 rounded-2xl border bg-background px-5 py-4"
            >
              <div class="flex min-w-0 items-center gap-3">
                <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-white ring-1 ring-black/5">
                  <img
                    v-if="selectedView?.icon"
                    :src="selectedView.icon"
                    :alt="selectedView.name"
                    class="h-5 w-5 object-contain"
                  />
                  <span
                    v-else
                    class="text-[10px] font-semibold text-muted-foreground"
                  >
                    {{ (selectedView?.name ?? '?').slice(0, 2).toUpperCase() }}
                  </span>
                </div>
                <span class="truncate text-[15px] font-medium text-foreground">
                  {{ modelId }}
                </span>
              </div>
            </div>
          </div>

          <div
            v-else
            class="rounded-2xl border border-dashed bg-muted/10 p-6 text-sm text-muted-foreground"
          >
            点击"拉取模型列表"获取该平台支持的模型，或使用默认列表。
          </div>
        </section>
      </div>
    </ScrollArea>

    <!-- 底部操作栏 -->
    <div class="flex flex-wrap items-center justify-end gap-3 border-t bg-background/95 px-8 py-4 backdrop-blur">
      <Button
        type="button"
        variant="destructive"
        class="h-10 rounded-xl"
        :disabled="!isConfigured"
        @click="openDeleteConfirm"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        删除配置
      </Button>

      <Button
        type="button"
        class="h-10 rounded-xl px-5"
        :disabled="!isFormValid"
        @click="saveConfig"
      >
        <Check class="mr-2 h-4 w-4" />
        保存配置
      </Button>
    </div>
  </div>

  <SagConfirm
    v-model:open="deleteConfirmOpen"
    title="确定删除吗？"
    description="删除后无法恢复"
    type="destructive"
    @confirm="confirmDelete"
  />
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/views/app-setting/_components/provider-config/_components/provider-config-panel.vue
git commit -m "refactor: provider-config-panel reads from store, supports dynamic model list refresh"
```

---

## Task 7: 更新 `index.vue`，清理 `ProviderType` 类型

**Files:**
- Modify: `src/views/app-setting/_components/provider-config/index.vue`

- [ ] **Step 1: 重写 `index.vue`**

```vue
<script setup lang="ts">
import { onMounted } from 'vue';
import { useProviderStore } from '@/stores/provider';
import ProviderList from './_components/provider-list.vue';
import ProviderConfigPanel from './_components/provider-config-panel.vue';

const store = useProviderStore();

onMounted(() => {
  store.loadConfigs();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card shadow-sm">
      <ProviderList />
      <ProviderConfigPanel v-if="store.selectedView" />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/views/app-setting/_components/provider-config/index.vue
git commit -m "refactor: provider-config index delegates to store, removes prop drilling"
```

---

## Task 8: 构建验证

- [ ] **Step 1: 前端类型检查**

```bash
npx vue-tsc --noEmit 2>&1 | head -60
```

预期：无 error

- [ ] **Step 2: Tauri 完整构建**

```bash
cargo tauri build --debug 2>&1 | tail -20
```

预期：build 成功，或只有 warning

- [ ] **Step 3: 启动应用手动验证**

```bash
cargo tauri dev
```

验证清单：
- [ ] 左侧列表展示 4 个内置 provider，均显示"未配置"
- [ ] 点击 DeepSeek，右侧展示 DeepSeek 图标和名称，base_url 已预填
- [ ] 填入 api_key，点击"保存配置"，toast 显示"保存成功"
- [ ] 刷新页面后，DeepSeek 列表项显示"已配置"
- [ ] 点击"拉取模型列表"，loading 状态正常，拉取成功后列表更新
- [ ] 点击"删除配置"，confirm 弹出，确认后配置清除

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: provider config refactor complete"
```
