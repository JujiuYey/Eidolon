# CRUD Code Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a CRUD code generator in the Tauri desktop app that reads a Go backend resource file, parses its model and payload, and generates frontend TypeScript API files and React CRUD page components for the external `smp-web` project.

**Architecture:** Rust backend parses Go source files (regex-based, no full parser), uses Tera templates (compile-time嵌入 via `include_str!`) to generate TypeScript/React code, exposed via Tauri commands called from Vue frontend.

**Tech Stack:**
- **本项目 (sco-code-app):** Tauri 2.x, Rust (regex, tera crates), Vue 3, TypeScript
- **生成目标 (smp-web):** React, TypeScript, `@vef-framework-react/components`

> **注意:** 本工具的前端 UI 用 Vue 3 编写（与本项目一致），但**生成的代码**是 React TSX，输出到外部 smp-web 项目。

---

## File Structure

```
src-tauri/
├── src/
│   ├── lib.rs                      # Register Tauri commands
│   ├── main.rs                     # Entry point
│   ├── commands/
│   │   └── codegen.rs              # Tauri command: generate_crud
│   └── services/
│       └── codegen/
│           ├── mod.rs              # Codegen service module
│           ├── go_parser.rs        # Go file parser
│           ├── ts_generator.rs     # TypeScript/React code generator
│           └── types.rs            # Parsed type definitions
├── templates/                      # Tera templates (与 src/ 平级，非代码资源)
│   ├── api.ts.tera                 # API file template
│   ├── route.tsx.tera              # Route component template
│   ├── form.tsx.tera               # Form component template
│   ├── basic_search.tsx.tera       # Basic search template
│   └── helpers.ts.tera             # Helpers template

smp-web/src/ (生成目标 - 仅供参考)
├── apis/sys/{name}.ts
└── pages/_layout/sys/{name}/
    ├── components/
    │   ├── basic-search.tsx
    │   └── form.tsx
    ├── helpers/
    │   └── index.ts
    └── route.tsx
```

---

## Task Decomposition

### Task 1: Add Rust Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add tera dependency to Cargo.toml**

项目已有 `regex`、`serde`、`serde_json`，只需新增 `tera`：

```toml
[dependencies]
# ... existing dependencies ...
tera = "1.20"
```

- [ ] **Step 2: Verify dependencies compile**

Run: `cd src-tauri && cargo check`
Expected: No dependency errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(crud-generator): add tera template engine dependency"
```

---

### Task 2: Create Type Definitions

**Files:**
- Create: `src-tauri/src/services/codegen/types.rs`

- [ ] **Step 1: Define struct for parsed model field**

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModelField {
    pub name: String,           // field_name
    pub json_name: String,      // from json tag
    pub go_type: String,        // string, *string, bool, int, etc.
    pub ts_type: String,        // TypeScript type
    pub label: String,          // Chinese label from label tag
    pub required: bool,         // from validate:"required"
    pub max_length: Option<usize>, // from validate:"max=N"
    pub is_optional: bool,       // pointer type (*string)
    pub is_json: bool,          // map[string]any type
}
```

- [ ] **Step 2: Define struct for parsed entity**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ParsedEntity {
    pub name: String,           // App
    pub snake_name: String,     // app
    pub table_name: String,      // sys_app
    pub module_name: String,     // sys (从 resource 路径自动推导)
    pub fields: Vec<ModelField>,
    pub search_fields: Vec<ModelField>,
    pub param_fields: Vec<ModelField>,
    pub rpc_path: String,        // "smp/sys/app"
}
```

> **变更说明：**
> - 新增 `module_name` 字段，从 resource 文件路径自动推导（如 `internal/sys/resource/app.go` → `sys`），避免硬编码模块名
> - 所有结构体添加 `Serialize` 派生，支持直接通过 `tera::Context::from_serialize()` 传入模板

- [ ] **Step 3: Define embedded struct field table**

```rust
/// Pre-defined fields from embedded structs
pub const BASE_MODEL_FIELDS: &[(&str, &str, &str)] = &[
    ("id", "id", "string"),
    ("created_at", "createdAt", "string"),
    ("updated_at", "updatedAt", "string"),
    ("deleted_at", "deletedAt", "MaybeNull<string>"),
    ("deleted_by", "deletedBy", "string"),
];

pub const FULL_AUDITED_FIELDS: &[(&str, &str, &str)] = &[
    ("created_by", "createdBy", "string"),
    ("updated_by", "updatedBy", "string"),
    ("created_by_name", "createdByName", "MaybeNull<string>"),
    ("updated_by_name", "updatedByName", "MaybeNull<string>"),
    ("deleted_by_name", "deletedByName", "MaybeNull<string>"),
];
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/codegen/types.rs
git commit -m "feat(crud-generator): add type definitions for parsed Go entities"
```

---

### Task 3: Create Go Parser Module

**Files:**
- Create: `src-tauri/src/services/codegen/go_parser.rs`

- [ ] **Step 1: Implement Go type to TypeScript type mapping**

```rust
fn go_type_to_ts_type(go_type: &str) -> String {
    match go_type {
        "string" => "string".to_string(),
        "bool" => "boolean".to_string(),
        "int" | "int32" | "int64" | "uint" | "uint32" | "uint64" => "number".to_string(),
        "*string" => "MaybeNull<string>".to_string(),
        "*bool" => "MaybeNull<boolean>".to_string(),
        "*int" | "*int32" | "*int64" => "MaybeNull<number>".to_string(),
        "map[string]any" => "Record<string, any>".to_string(),
        "[]string" => "MaybeNull<string>[]".to_string(),
        _ => "any".to_string(),
    }
}
```

- [ ] **Step 2: Implement field extraction from struct lines**

```rust
fn parse_field_line(line: &str) -> Option<(String, String, String, String, bool, Option<usize>)>
```

Returns: `(field_name, json_name, go_type, label, required, max_length)`

Uses regex patterns:
- `json:"(\w+)"` for JSON name
- `label:"([^"]+)"` for label
- `validate:"([^"]+)"` for validation rules (解析 `required`, `max=N` 等)
- Field name and type from `^\s+(\w+)\s+([\w*\[\]{}]+)` pattern

- [ ] **Step 3: Implement embedded struct expansion**

```rust
fn expand_embedded_fields(embedded_name: &str) -> Vec<(&'static str, &'static str, &'static str)>
```

Matches `orm.BaseModel` → returns `BASE_MODEL_FIELDS`
Matches `orm.FullAuditedModel` → returns `BASE_MODEL_FIELDS` + `FULL_AUDITED_FIELDS`

- [ ] **Step 4: Implement model file parser**

```rust
pub fn parse_model_file(content: &str, struct_name: &str) -> Result<Vec<ModelField>, String>
```

1. Find struct definition (regex `type {struct_name} struct \{`)
2. Parse each field line until closing `}`
3. If embedded struct, call `expand_embedded_fields`
4. Otherwise call `parse_field_line`
5. Convert Go types via `go_type_to_ts_type`
6. 返回错误信息如果 struct 未找到

- [ ] **Step 5: Implement payload file parser**

```rust
pub fn parse_payload_file(content: &str, entity_name: &str) -> Result<(Vec<ModelField>, Vec<ModelField>), String>
```

解析 `{EntityName}Search` 和 `{EntityName}Params` 两个 struct，返回 `(search_fields, param_fields)`。
如果 Search 或 Params struct 不存在，返回空 Vec 而非报错（某些实体可能只有其中之一）。

- [ ] **Step 6: Implement RPC path extraction from resource file**

```rust
pub fn extract_rpc_path(content: &str) -> Option<String>
```

Go resource 文件中路径声明的典型模式：

```go
// 模式1: Resource 初始化
var appResource = resource.NewResource[model.App, payload.AppSearch, payload.AppParams](
    resource.Config{
        Path: "smp/sys/app",
    },
)

// 模式2: 方法调用
func (r *AppResource) Path() string {
    return "smp/sys/app"
}
```

使用正则 `Path[:\s]*"([^"]+)"` 匹配，覆盖两种常见写法。

- [ ] **Step 7: Implement module name extraction from path**

```rust
/// 从 resource 文件路径推导模块名
/// "internal/sys/resource/app.go" → module="sys", entity="app"
/// "internal/biz/resource/order.go" → module="biz", entity="order"
fn extract_module_and_entity(resource_path: &str) -> Result<(String, String), String>
```

使用正则 `internal/(\w+)/resource/(\w+)\.go$` 匹配，自动推导模块名，避免硬编码 `sys`。

- [ ] **Step 8: Implement main parse function**

```rust
pub fn parse_go_resource(resource_path: &str) -> Result<ParsedEntity, String>
```

1. 从路径推导 `module_name` 和 `entity_name`（Step 7）
2. Read resource file, extract rpc_path
3. Derive model path: `internal/{module}/model/{entity}.go`
4. Derive payload path: `internal/{module}/payload/{entity}.go`
5. Parse model file → get fields（文件不存在时返回明确错误）
6. Parse payload file → get search_fields and param_fields
7. Return `ParsedEntity`

- [ ] **Step 9: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_type_to_ts_type() {
        assert_eq!(go_type_to_ts_type("string"), "string");
        assert_eq!(go_type_to_ts_type("*string"), "MaybeNull<string>");
        assert_eq!(go_type_to_ts_type("int64"), "number");
        assert_eq!(go_type_to_ts_type("map[string]any"), "Record<string, any>");
        assert_eq!(go_type_to_ts_type("CustomType"), "any");
    }

    #[test]
    fn test_parse_field_line() {
        let line = r#"    Name string `json:"name" label:"名称" validate:"required,max=100"`"#;
        let result = parse_field_line(line);
        assert!(result.is_some());
        let (name, json_name, go_type, label, required, max_len) = result.unwrap();
        assert_eq!(json_name, "name");
        assert_eq!(label, "名称");
        assert!(required);
        assert_eq!(max_len, Some(100));
    }

    #[test]
    fn test_extract_module_and_entity() {
        let (module, entity) = extract_module_and_entity(
            "/path/to/project/internal/sys/resource/app.go"
        ).unwrap();
        assert_eq!(module, "sys");
        assert_eq!(entity, "app");
    }

    #[test]
    fn test_parse_model_file() {
        let content = r#"
type App struct {
    orm.BaseModel
    Name string `json:"name" label:"名称" validate:"required"`
    Code string `json:"code" label:"编码"`
}
"#;
        let fields = parse_model_file(content, "App").unwrap();
        // Should have base model fields + 2 custom fields
        assert!(fields.len() >= 7);
    }
}
```

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/services/codegen/go_parser.rs src-tauri/src/services/codegen/mod.rs
git commit -m "feat(crud-generator): implement Go file parser with unit tests"
```

---

### Task 4: Create Tera Templates

**Files:**
- Create: `src-tauri/templates/api.ts.tera`
- Create: `src-tauri/templates/route.tsx.tera`
- Create: `src-tauri/templates/form.tsx.tera`
- Create: `src-tauri/templates/basic_search.tsx.tera`
- Create: `src-tauri/templates/helpers.ts.tera`

> **模板路径说明：** 模板放在 `src-tauri/templates/`（与 `src/` 平级），不混入 Rust 代码目录。生成器通过 `include_str!` 在编译时嵌入模板内容，打包后不依赖文件系统路径。

- [ ] **Step 1: Create API template** (`api.ts.tera`)

Template generates:
- `export interface {{ entity_name }} extends FullAuditedEntity { ... fields ... }`
- `export interface {{ entity_name }}Search { ... search fields ... }`
- `export type {{ entity_name }}Params = Omit<{{ entity_name }}, audit_fields>`
- 6 API functions using `apiClient.createQueryFn` / `createMutationFn`
- `API_PATH` constant

Variables: `entity_name`, `snake_name`, `rpc_path`, `fields`, `search_fields`, `param_fields`

- [ ] **Step 2: Create Route template** (`route.tsx.tera`)

Template generates:
- `tableColumns` array with render functions
- `RouteComponent` with `CrudPage`
- Columns render: bool → Tag, string → ellipsis, map → JSON

Variables: `entity_name`, `fields`, `snake_name`, `default_form_values`

- [ ] **Step 3: Create Form template** (`form.tsx.tera`)

Template generates:
- `Grid` layout form
- Each field with `AppField` component
- Validation rules from field metadata
- Input type selection: `string` → Input, `number` → InputNumber, `boolean` → Switch, long text → TextArea

Variables: `entity_name`, `fields`, `param_fields`

- [ ] **Step 4: Create BasicSearch template** (`basic_search.tsx.tera`)

Template generates:
- Simple form with keyword Input
- Only if `search_fields` contains `keyword`

Variables: `entity_name`, `search_fields`

- [ ] **Step 5: Create Helpers template** (`helpers.ts.tera`)

Template generates:
- `createCrudKit<{{ entity_name }}, {{ entity_name }}Search, CrudBasicSceneFormValues<{{ entity_name }}Params, {{ entity_name }}Params>>()`
- Named exports

Variables: `entity_name`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/templates/
git commit -m "feat(crud-generator): add Tera templates for TypeScript/React code generation"
```

---

### Task 5: Create TypeScript Generator

**Files:**
- Create: `src-tauri/src/services/codegen/ts_generator.rs`

- [ ] **Step 1: Implement template loading via include_str!**

```rust
/// 编译时嵌入模板，避免运行时文件系统依赖
const API_TEMPLATE: &str = include_str!("../../../templates/api.ts.tera");
const ROUTE_TEMPLATE: &str = include_str!("../../../templates/route.tsx.tera");
const FORM_TEMPLATE: &str = include_str!("../../../templates/form.tsx.tera");
const BASIC_SEARCH_TEMPLATE: &str = include_str!("../../../templates/basic_search.tsx.tera");
const HELPERS_TEMPLATE: &str = include_str!("../../../templates/helpers.ts.tera");

pub fn create_tera_engine() -> Result<tera::Tera, String> {
    let mut tera = tera::Tera::default();
    tera.add_raw_templates(vec![
        ("api.ts", API_TEMPLATE),
        ("route.tsx", ROUTE_TEMPLATE),
        ("form.tsx", FORM_TEMPLATE),
        ("basic_search.tsx", BASIC_SEARCH_TEMPLATE),
        ("helpers.ts", HELPERS_TEMPLATE),
    ]).map_err(|e| format!("模板加载失败: {e}"))?;
    Ok(tera)
}
```

- [ ] **Step 2: Implement unified render function**

```rust
fn render_template(
    tera: &tera::Tera,
    template_name: &str,
    entity: &ParsedEntity,
) -> Result<String, String> {
    let context = tera::Context::from_serialize(entity)
        .map_err(|e| format!("序列化实体失败: {e}"))?;
    tera.render(template_name, &context)
        .map_err(|e| format!("渲染模板 {template_name} 失败: {e}"))
}
```

- [ ] **Step 3: Implement main generation function**

```rust
pub fn generate_crud_files(
    entity: &ParsedEntity,
    frontend_base_path: &str,
    overwrite: bool,
) -> Result<Vec<String>, String>
```

1. Derive paths (使用 `entity.module_name` 而非硬编码 `sys`)：
   - `apis/{module}/{snake_name}.ts`
   - `pages/_layout/{module}/{snake_name}/components/basic-search.tsx`
   - `pages/_layout/{module}/{snake_name}/components/form.tsx`
   - `pages/_layout/{module}/{snake_name}/helpers/index.ts`
   - `pages/_layout/{module}/{snake_name}/route.tsx`
2. **文件覆盖检查**：如果目标文件已存在且 `overwrite=false`，返回错误并列出已存在的文件路径
3. Generate each file content via `render_template`
4. Create directories (`std::fs::create_dir_all`)
5. Write files
6. Return list of generated file paths

- [ ] **Step 4: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tera_engine() {
        let tera = create_tera_engine().unwrap();
        assert!(tera.get_template_names().count() == 5);
    }

    #[test]
    fn test_render_api_template() {
        let tera = create_tera_engine().unwrap();
        let entity = ParsedEntity {
            name: "App".to_string(),
            snake_name: "app".to_string(),
            table_name: "sys_app".to_string(),
            module_name: "sys".to_string(),
            fields: vec![],
            search_fields: vec![],
            param_fields: vec![],
            rpc_path: "smp/sys/app".to_string(),
        };
        let result = render_template(&tera, "api.ts", &entity);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/codegen/ts_generator.rs
git commit -m "feat(crud-generator): implement TypeScript code generator with overwrite protection"
```

---

### Task 6: Create Tauri Command & Register Module

**Files:**
- Create: `src-tauri/src/commands/codegen.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create codegen service module entry**

`src-tauri/src/services/codegen/mod.rs`:

```rust
pub mod go_parser;
pub mod ts_generator;
pub mod types;
```

Register in `src-tauri/src/services/mod.rs`:

```rust
pub mod codegen;
pub mod project_files;
```

- [ ] **Step 2: Implement Tauri command**

`src-tauri/src/commands/codegen.rs`:

```rust
use crate::services::codegen::{go_parser, ts_generator};

#[tauri::command]
pub async fn generate_crud(
    resource_path: String,
    frontend_base_path: String,
    overwrite: Option<bool>,
) -> Result<Vec<String>, String> {
    // 1. Validate paths exist
    if !std::path::Path::new(&resource_path).exists() {
        return Err(format!("Resource 文件不存在: {resource_path}"));
    }
    if !std::path::Path::new(&frontend_base_path).is_dir() {
        return Err(format!("前端目录不存在: {frontend_base_path}"));
    }

    // 2. Parse Go resource file
    let entity = go_parser::parse_go_resource(&resource_path)?;

    // 3. Generate files
    let generated = ts_generator::generate_crud_files(
        &entity,
        &frontend_base_path,
        overwrite.unwrap_or(false),
    )?;

    Ok(generated)
}
```

- [ ] **Step 3: Register command in commands/mod.rs**

```rust
pub mod app_setting;
pub mod codegen;
pub mod model_config;
pub mod project;
pub mod project_files;
pub mod project_glossary_term;
pub mod prompt_templates;
pub mod test_connection;
```

- [ ] **Step 4: Register command in lib.rs**

Add `commands::codegen::generate_crud` to `invoke_handler` in existing `generate_handler!` macro.

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/codegen.rs src-tauri/src/commands/mod.rs \
       src-tauri/src/services/codegen/ src-tauri/src/services/mod.rs \
       src-tauri/src/lib.rs
git commit -m "feat(crud-generator): add Tauri command and wire up codegen service"
```

---

### Task 7: Create Vue Frontend Page

**Files:**
- Create: `src/views/codegen/index.vue`
- Modify: `src/router/index.ts` (添加路由)

> **UI 说明：** 本页面使用项目已有的 shadcn-vue 组件库（Card、Button、Input、Label 等），风格与其他页面保持一致。

- [ ] **Step 1: Create page component**

`src/views/codegen/index.vue`:

```vue
<template>
  <div class="p-6 space-y-6">
    <Card>
      <CardHeader>
        <CardTitle>CRUD Code Generator</CardTitle>
        <CardDescription>
          从 Go 后端 Resource 文件生成前端 TypeScript API 和 React CRUD 页面
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <!-- Resource 文件路径 -->
        <div class="space-y-2">
          <Label>Go Resource 文件路径</Label>
          <div class="flex gap-2">
            <Input v-model="resourcePath" placeholder="internal/sys/resource/app.go" class="flex-1" />
            <Button variant="outline" @click="selectResourceFile">选择文件</Button>
          </div>
        </div>

        <!-- 前端输出目录 -->
        <div class="space-y-2">
          <Label>前端项目 src 目录</Label>
          <div class="flex gap-2">
            <Input v-model="frontendBasePath" placeholder="/path/to/smp-web/src" class="flex-1" />
            <Button variant="outline" @click="selectFrontendDir">选择目录</Button>
          </div>
        </div>

        <!-- 覆盖选项 -->
        <div class="flex items-center gap-2">
          <input type="checkbox" id="overwrite" v-model="overwrite" />
          <Label for="overwrite">覆盖已有文件</Label>
        </div>
      </CardContent>
      <CardFooter>
        <Button @click="handleGenerate" :disabled="loading || !resourcePath || !frontendBasePath">
          {{ loading ? '生成中...' : '生成代码' }}
        </Button>
      </CardFooter>
    </Card>

    <!-- 结果展示 -->
    <Card v-if="generatedFiles.length > 0">
      <CardHeader>
        <CardTitle>生成完成</CardTitle>
        <CardDescription>共生成 {{ generatedFiles.length }} 个文件</CardDescription>
      </CardHeader>
      <CardContent>
        <ul class="space-y-1 text-sm font-mono">
          <li v-for="file in generatedFiles" :key="file">{{ file }}</li>
        </ul>
      </CardContent>
    </Card>

    <!-- 错误提示 -->
    <Card v-if="error" class="border-destructive">
      <CardContent class="pt-6 text-destructive text-sm">{{ error }}</CardContent>
    </Card>
  </div>
</template>
```

- [ ] **Step 2: Implement script logic**

```typescript
<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const resourcePath = ref('');
const frontendBasePath = ref('');
const overwrite = ref(false);
const loading = ref(false);
const generatedFiles = ref<string[]>([]);
const error = ref('');

const selectResourceFile = async () => {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Go Files', extensions: ['go'] }],
  });
  if (selected) resourcePath.value = selected as string;
};

const selectFrontendDir = async () => {
  const selected = await open({ directory: true, multiple: false });
  if (selected) frontendBasePath.value = selected as string;
};

const handleGenerate = async () => {
  loading.value = true;
  error.value = '';
  generatedFiles.value = [];
  try {
    const result = await invoke<string[]>('generate_crud', {
      resourcePath: resourcePath.value,
      frontendBasePath: frontendBasePath.value,
      overwrite: overwrite.value,
    });
    generatedFiles.value = result;
  } catch (e: any) {
    error.value = typeof e === 'string' ? e : e.message || '生成失败';
  } finally {
    loading.value = false;
  }
};
</script>
```

- [ ] **Step 3: Add route**

`src/router/index.ts` — 在 `children` 数组中添加：

```typescript
{
  path: '/codegen',
  component: () => import('@/views/codegen/index.vue'),
},
```

- [ ] **Step 4: Commit**

```bash
git add src/views/codegen/index.vue src/router/index.ts
git commit -m "feat(crud-generator): add Vue frontend page for CRUD code generation"
```

---

### Task 8: Unit Tests (Rust)

**Files:**
- Create: `src-tauri/tests/codegen_test.rs` (集成测试，可选)

- [ ] **Step 1: Run all unit tests**

```bash
cd src-tauri && cargo test
```

Verify all tests in `go_parser` and `ts_generator` pass.

- [ ] **Step 2: Fix any failing tests**

- [ ] **Step 3: Commit fixes if any**

---

### Task 9: End-to-End Verification

**Files:**
- Test against: Go backend resource file (e.g. `app.go`)

- [ ] **Step 1: Build the Tauri app**

Run: `cd src-tauri && cargo build`

- [ ] **Step 2: Launch app and test**

Input:
- Resource path: a valid Go resource file path
- Frontend path: target frontend `src/` directory

- [ ] **Step 3: Verify generated files**

Check:
- `apis/{module}/{entity}.ts` — has correct interfaces and API functions
- `pages/_layout/{module}/{entity}/route.tsx` — has correct table columns
- `pages/_layout/{module}/{entity}/components/form.tsx` — has correct form fields
- `pages/_layout/{module}/{entity}/components/basic-search.tsx` — exists
- `pages/_layout/{module}/{entity}/helpers/index.ts` — exists

- [ ] **Step 4: Verify generated code compiles in target project**

If smp-web is available, run its TypeScript compiler to verify generated files have no type errors.

---

## Verification Checklist

1. **Rust compilation:** `cd src-tauri && cargo build` succeeds
2. **Unit tests pass:** `cd src-tauri && cargo test` all green
3. **Tauri app runs:** App launches without errors
4. **Route accessible:** Navigate to `/codegen` page
5. **Generation works:** Input resource path + frontend dir, click generate
6. **File overwrite protection:** Attempting to generate into existing files without "overwrite" checked shows error
7. **Error handling:** Invalid paths show clear error messages
8. **Files correct:** Generated TypeScript/React files compile without type errors in target project
9. **Import works:** Generated `{entity}.ts` imports correctly from `~apis`

---

## Error Handling Strategy

| Scenario | Behavior |
|----------|----------|
| Resource file 不存在 | 返回错误: "Resource 文件不存在: {path}" |
| Resource 路径不匹配 `internal/{module}/resource/{entity}.go` 模式 | 返回错误: "无法从路径推导模块名，期望格式: internal/{module}/resource/{entity}.go" |
| Model file 不存在 | 返回错误: "Model 文件不存在: {derived_path}" |
| Payload file 不存在 | 警告但不报错（search_fields/param_fields 为空） |
| Struct 定义未在文件中找到 | 返回错误: "未找到 struct {StructName} 定义" |
| 目标文件已存在且 overwrite=false | 返回错误并列出已存在的文件 |
| 目标目录不存在 | 自动创建（`create_dir_all`） |
| 模板渲染失败 | 返回错误: "渲染模板 {name} 失败: {detail}" |
| 字段解析失败（非标准格式） | 跳过该字段，继续解析其他字段 |

---

## Notes

- **Embedded struct handling:** 使用预定义字段表处理 `orm.BaseModel` 和 `orm.FullAuditedModel`。新增嵌入 struct 时需更新 `types.rs` 中的常量。
- **Type mapping:** 支持基础 Go 类型。复杂类型（自定义 struct、enum）回退为 `any`。
- **Template customization:** 模板遵循 `@vef-framework-react/components` 约定。组件 API 变化时需同步更新模板。
- **Module generality:** 通过路径自动推导模块名，支持 `sys`、`biz` 等不同模块，无需硬编码。
- **生成目标说明：** 本工具生成的是 React TSX 代码（用于 smp-web），而非 Vue 组件。本工具自身的 UI 是 Vue 3。
