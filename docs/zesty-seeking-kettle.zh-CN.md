# CRUD 代码生成器实施方案

> **给智能代理的说明：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 来实现本计划中的任务。步骤使用复选框（`- [ ]`）语法进行跟踪。

**目标：** 在 Tauri 桌面应用中构建一个 CRUD 代码生成器，读取 Go 后端资源文件，解析其模型和载荷，生成前端 TypeScript API 文件和 Vue CRUD 页面组件。

**架构：** Rust 后端解析 Go 源文件（基于正则表达式，无需完整解析器），使用 Tera 模板生成 TypeScript/Vue 代码，通过 Tauri 命令暴露给 Vue 前端调用。

**技术栈：** Tauri 2.x、Rust（regex、tera crates）、Vue 3、TypeScript

---

## 文件结构

```
src-tauri/
├── src/
│   ├── lib.rs                      # 注册 Tauri 命令
│   ├── main.rs                     # 入口点
│   ├── commands/
│   │   └── generate.rs             # Tauri 命令：generate_crud
│   ├── parser/
│   │   ├── mod.rs                  # 解析器模块
│   │   ├── go_parser.rs            # Go 文件解析器
│   │   └── types.rs                # 解析后的类型定义
│   ├── generator/
│   │   ├── mod.rs                  # 生成器模块
│   │   └── ts_generator.rs         # TypeScript 代码生成器
│   └── templates/
│       ├── api.ts.tpl              # API 文件模板
│       ├── route.tsx.tpl           # 路由组件模板
│       ├── form.tsx.tpl            # 表单组件模板
│       ├── basic_search.tsx.tpl   # 基础搜索模板
│       └── helpers.ts.tpl          # 辅助函数模板

smp-web/src/ (生成输出 - 仅作参考)
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

## 任务分解

### 任务 1：添加 Rust 依赖

**文件：**
- 修改：`src-tauri/Cargo.toml`

- [ ] **步骤 1：在 Cargo.toml 中添加依赖**

添加 `regex`、`tera` 和 `serde` 包：

```toml
[dependencies]
regex = "1.10"
tera = "1.20"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

- [ ] **步骤 2：验证依赖编译**

运行：`cd src-tauri && cargo check`
预期结果：无依赖错误

- [ ] **步骤 3：提交**

```bash
git add src-tauri/Cargo.toml
git commit -m "feat(crud-generator): add regex, tera, serde dependencies"
```

---

### 任务 2：创建类型定义

**文件：**
- 创建：`src-tauri/src/parser/types.rs`

- [ ] **步骤 1：定义解析后的模型字段结构体**

```rust
#[derive(Debug, Clone)]
pub struct ModelField {
    pub name: String,           // field_name
    pub json_name: String,      // 来自 json 标签
    pub go_type: String,        // string, *string, bool, int 等
    pub ts_type: String,        // TypeScript 类型
    pub label: String,          // 来自 label 标签的中文标签
    pub required: bool,         // 来自 validate:"required"
    pub max_length: Option<usize>, // 来自 validate:"max=N"
    pub is_optional: bool,       // 指针类型 (*string)
    pub is_json: bool,          // map[string]any 类型
}
```

- [ ] **步骤 2：定义解析后的实体结构体**

```rust
#[derive(Debug, Clone)]
pub struct ParsedEntity {
    pub name: String,           // App
    pub snake_name: String,     // app
    pub table_name: String,      // sys_app
    pub fields: Vec<ModelField>,
    pub search_fields: Vec<ModelField>,
    pub param_fields: Vec<ModelField>,
    pub rpc_path: String,        // "smp/sys/app"
}
```

- [ ] **步骤 3：定义嵌入式结构体字段表**

```rust
// 来自嵌入式结构体的预定义字段
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

- [ ] **步骤 4：提交**

```bash
git add src-tauri/src/parser/types.rs
git commit -m "feat(crud-generator): add type definitions for parsed Go entities"
```

---

### 任务 3：创建 Go 解析器模块

**文件：**
- 创建：`src-tauri/src/parser/go_parser.rs`

- [ ] **步骤 1：实现 Go 类型到 TypeScript 类型的映射**

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

- [ ] **步骤 2：实现从结构体行中提取字段**

```rust
fn parse_field_line(line: &str) -> Option<(String, String, String, String, bool, Option<usize>)>
```

返回：`(field_name, json_name, go_type, label, required, max_length)`

使用正则表达式模式：
- `json:"(\w+)"` 获取 JSON 名称
- `label:"([^"]+)"` 获取标签
- `validate:"([^"]+)"` 获取验证规则
- 从 `FieldName Type` 中获取字段名和类型

- [ ] **步骤 3：实现嵌入式结构体展开**

```rust
fn expand_embedded_fields(embedded_name: &str) -> Vec<(&'static str, &'static str, &'static str)>
```

匹配 `orm.BaseModel` → 返回 `BASE_MODEL_FIELDS`
匹配 `orm.FullAuditedModel` → 返回 `FULL_AUDITED_FIELDS`

- [ ] **步骤 4：实现模型文件解析器**

```rust
pub fn parse_model_file(content: &str, struct_name: &str) -> Vec<ModelField>
```

1. 查找结构体定义（正则表达式 `type Xxx struct {`）
2. 逐行解析字段直到闭合 `}`
3. 如果是嵌入式结构体，调用 `expand_embedded_fields`
4. 否则调用 `parse_field_line`
5. 通过 `go_type_to_ts_type` 转换 Go 类型

- [ ] **步骤 5：实现载荷文件解析器**

```rust
pub fn parse_payload_file(content: &str, struct_name: &str) -> Vec<ModelField>
```

与模型解析器相同的逻辑，但针对 `XxxSearch` 和 `XxxParams` 结构体。

- [ ] **步骤 6：实现从资源文件中提取 RPC 路径**

```rust
pub fn extract_rpc_path(content: &str) -> Option<String>
```

使用正则表达式在资源文件中查找 `Resource.Path("...")` 或类似模式。

- [ ] **步骤 7：实现主解析函数**

```rust
pub fn parse_go_resource(resource_path: &str) -> Result<ParsedEntity, String>
```

1. 读取资源文件，提取 rpc_path 和结构体名称
2. 推导模型路径：`internal/sys/model/{name}.go`
3. 推导载荷路径：`internal/sys/payload/{name}.go`
4. 解析模型文件 → 获取 fields
5. 解析载荷文件 → 获取 search_fields 和 param_fields
6. 返回 `ParsedEntity`

- [ ] **步骤 8：提交**

```bash
git add src-tauri/src/parser/go_parser.rs src-tauri/src/parser/mod.rs
git commit -m "feat(crud-generator): implement Go file parser"
```

---

### 任务 4：创建 Tera 模板

**文件：**
- 创建：`src-tauri/src/templates/api.ts.tpl`
- 创建：`src-tauri/src/templates/route.tsx.tpl`
- 创建：`src-tauri/src/templates/form.tsx.tpl`
- 创建：`src-tauri/src/templates/basic_search.tsx.tpl`
- 创建：`src-tauri/src/templates/helpers.ts.tpl`

- [ ] **步骤 1：创建 API 模板**（`api.ts.tpl`）

模板生成：
- `export interface Xxx extends FullAuditedEntity { ... fields ... }`
- `export interface XxxSearch { ... search fields ... }`
- `export type XxxParams = Omit<Xxx, audit_fields>`
- 6 个使用 `apiClient.createQueryFn` / `createMutationFn` 的 API 函数
- `API_PATH` 常量

变量：`entity_name`、`snake_name`、`rpc_path`、`fields`、`search_fields`、`param_fields`

- [ ] **步骤 2：创建路由模板**（`route.tsx.tpl`）

模板生成：
- 带有渲染函数的 `tableColumns` 数组
- 使用 `CrudPage` 的 `RouteComponent`
- 列渲染：bool 渲染为 Tag，string 显示省略号，meta 渲染为 JSON

变量：`entity_name`、`fields`、`snake_name`、`default_form_values`

- [ ] **步骤 3：创建表单模板**（`form.tsx.tpl`）

模板生成：
- `Grid` 布局表单
- 每个字段使用 `AppField` 组件
- 基于字段元数据的验证规则
- 根据类型选择 Input vs InputNumber vs Bool vs TextArea

变量：`entity_name`、`fields`、`param_fields`

- [ ] **步骤 4：创建基础搜索模板**（`basic_search.tsx.tpl`）

模板生成：
- 带有关键词 Input 的简单表单
- 仅当 `search_fields` 包含 `keyword` 时生成

变量：`entity_name`、`search_fields`

- [ ] **步骤 5：创建辅助函数模板**（`helpers.ts.tpl`）

模板生成：
- `createCrudKit<App, AppSearch, CrudBasicSceneFormValues<AppParams, AppParams>>()`
- 命名导出

变量：`entity_name`

- [ ] **步骤 6：提交**

```bash
git add src-tauri/src/templates/
git commit -m "feat(crud-generator): add Tera templates for TypeScript/Vue code generation"
```

---

### 任务 5：创建 TypeScript 生成器

**文件：**
- 创建：`src-tauri/src/generator/ts_generator.rs`

- [ ] **步骤 1：实现模板加载**

```rust
pub fn load_templates() -> tera::Tera {
    // 从 templates/ 目录加载所有 .tpl 文件
}
```

- [ ] **步骤 2：实现 API 文件生成**

```rust
pub fn generate_api_ts(entity: &ParsedEntity) -> Result<String, String>
```

1. 使用实体上下文渲染 `api.ts.tpl`
2. 返回渲染后的内容

- [ ] **步骤 3：实现路由文件生成**

```rust
pub fn generate_route_tsx(entity: &ParsedEntity) -> Result<String, String>
```

1. 使用实体上下文渲染 `route.tsx.tpl`
2. 返回渲染后的内容

- [ ] **步骤 4：实现表单文件生成**

```rust
pub fn generate_form_tsx(entity: &ParsedEntity) -> Result<String, String>
```

1. 使用实体上下文渲染 `form.tsx.tpl`
2. 返回渲染后的内容

- [ ] **步骤 5：实现基础搜索文件生成**

```rust
pub fn generate_basic_search_tsx(entity: &ParsedEntity) -> Result<String, String>
```

1. 使用实体上下文渲染 `basic_search.tsx.tpl`
2. 返回渲染后的内容

- [ ] **步骤 6：实现辅助函数文件生成**

```rust
pub fn generate_helpers_ts(entity: &ParsedEntity) -> Result<String, String>
```

1. 使用实体上下文渲染 `helpers.ts.tpl`
2. 返回渲染后的内容

- [ ] **步骤 7：实现目录创建**

```rust
pub fn ensure_dir(path: &Path) -> Result<(), String>
```

如果目录不存在，创建目录及所有父目录。

- [ ] **步骤 8：实现文件写入**

```rust
pub fn write_generated_file(path: &Path, content: &str) -> Result<(), String>
```

将内容写入文件，如果目录不存在则失败。

- [ ] **步骤 9：实现主生成函数**

```rust
pub fn generate_crud_files(
    entity: &ParsedEntity,
    frontend_base_path: &str,
) -> Result<Vec<String>, String>
```

1. 推导路径：
   - `apis/sys/{snake_name}.ts`
   - `pages/_layout/sys/{snake_name}/components/basic-search.tsx`
   - `pages/_layout/sys/{snake_name}/components/form.tsx`
   - `pages/_layout/sys/{snake_name}/helpers/index.ts`
   - `pages/_layout/sys/{snake_name}/route.tsx`
2. 生成每个文件内容
3. 创建目录
4. 写入文件
5. 返回生成的文件路径列表

- [ ] **步骤 10：提交**

```bash
git add src-tauri/src/generator/mod.rs src-tauri/src/generator/ts_generator.rs
git commit -m "feat(crud-generator): implement TypeScript code generator"
```

---

### 任务 6：创建 Tauri 命令

**文件：**
- 创建：`src-tauri/src/commands/generate.rs`

- [ ] **步骤 1：实现 Tauri 命令**

```rust
#[tauri::command]
pub async fn generate_crud(
    resource_path: String,
    frontend_base_path: String,
) -> Result<Vec<String>, String>
```

1. 解析 Go 资源文件：`parse_go_resource(&resource_path)`
2. 生成文件：`generate_crud_files(&entity, &frontend_base_path)`
3. 返回生成的文件列表

- [ ] **步骤 2：在 lib.rs 中注册命令**

```rust
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(...)
        .invoke_handler(tauri::generate_handler![commands::generate::generate_crud])
        .run(...)
}
```

- [ ] **步骤 3：验证编译**

运行：`cd src-tauri && cargo check`
预期结果：编译无错误

- [ ] **步骤 4：提交**

```bash
git add src-tauri/src/commands/generate.rs src-tauri/src/lib.rs
git commit -m "feat(crud-generator): add Tauri command for CRUD generation"
```

---

### 任务 7：创建 Vue 前端页面

**文件：**
- 创建或修改：桌面应用中 CRUD 生成器的 Vue 页面

- [ ] **步骤 1：创建页面布局**

- 资源路径输入字段
- 前端目录选择器（使用 Tauri dialog）
- 生成按钮

- [ ] **步骤 2：实现目录选择**

```typescript
import { open } from "@tauri-apps/plugin-dialog";

const selectDirectory = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
  });
  return selected;
};
```

- [ ] **步骤 3：实现生成调用**

```typescript
import { invoke } from "@tauri-apps/api-tauri";

const handleGenerate = async () => {
  const result = await invoke<string[]>("generate_crud", {
    resourcePath: resourcePath.value,
    frontendBasePath: frontendBasePath.value,
  });
  // 显示包含生成文件列表的成功消息
};
```

- [ ] **步骤 4：显示结果**

显示生成的文件列表，提供复制到剪贴板或在新编辑器中打开的操作。

- [ ] **步骤 5：提交**

```bash
git add <frontend-file>
git commit -m "feat(crud-generator): add Vue frontend for CRUD generator"
```

---

### 任务 8：集成测试

**文件：**
- 测试目标：`/Users/jujiuyey/Documents/Work/sm/smp-server/internal/sys/resource/app.go`

- [ ] **步骤 1：构建 Tauri 应用**

运行：`cd src-tauri && cargo build --release`

- [ ] **步骤 2：使用 app.go 资源路径测试**

输入：
- 资源路径：`/Users/jujiuyey/Documents/Work/sm/smp-server/internal/sys/resource/app.go`
- 前端路径：`/Users/jujiuyey/Documents/Work/sm/smp-web/src`

- [ ] **步骤 3：验证生成的文件**

检查：
- `src/apis/sys/app.ts` 存在且包含正确的 API 函数
- `src/pages/_layout/sys/app/route.tsx` 包含正确的表格列
- `src/pages/_layout/sys/app/components/form.tsx` 包含正确的表单字段
- `src/pages/_layout/sys/app/components/basic-search.tsx` 存在
- `src/pages/_layout/sys/app/helpers/index.ts` 存在

- [ ] **步骤 4：提交生成的测试文件**

```bash
git add <generated-files>
git commit -m "test(crud-generator): verify generated CRUD files for app entity"
```

---

## 验证

1. **Rust 编译：** `cd src-tauri && cargo build` 成功
2. **Tauri 应用运行：** 应用启动无错误
3. **生成功能正常：** 输入资源路径 + 前端目录，点击生成
4. **文件正确：** 生成的 TypeScript/Vue 文件编译无错误
5. **导入正常：** 生成的 `app.ts` 可以正确从 `~apis` 导入

---

## 注意事项

- **嵌入式结构体处理：** 目前使用 `orm.BaseModel` 和 `orm.FullAuditedModel` 的预定义字段表。如果添加了新的嵌入式结构体，请更新 `types.rs`。
- **类型映射：** 支持基本的 Go 类型。复杂类型（自定义结构体、枚举）会回退到 `any`。
- **模板定制：** 模板遵循 `@vef-framework-react/components` 约定。如果组件 API 发生变化，请调整模板。
