# 后端 CRUD 代码生成设计方案

## 一、需求概述

为现有的代码生成系统添加后端 Go 语言 CRUD 代码生成功能。

### 核心需求

1. **SQL 解析**: 支持用户输入/粘贴 SQL DDL 语句（含 `COMMENT` 注释），自动解析生成 Model
2. **文件树选择**: 界面显示目录树，让用户选择存放路径
3. **代码生成**: 根据解析结果和配置选项生成完整的 Go 后端 CRUD 代码
4. **模块注册**: 生成完成后提示用户手动将新资源注册到 `module.go`（不自动修改）

### 技术约束

- **Schema 文件**: 由 vef-cli 工具自动生成，我们不处理
- **默认 CRUD**: FindPage + Create + Update + Delete
- **审计类型**: 默认 `orm.FullAuditedModel`
- **module.go**: 不自动修改，生成完成后展示需要手动添加的代码片段供用户复制

---

## 二、后端代码结构

### 2.1 现有代码模式

参考项目: `smp-server/internal/sys`

#### Model 层 (`model/*.go`)

```go
package model

import (
    "github.com/coldsmirk/vef-framework-go/orm"
)

// Department 部门模型.
type Department struct {
    orm.BaseModel `bun:"table:sys_department,alias:md"`
    orm.FullAuditedModel

    ParentID      *string `json:"parentId" label:"上级部门主键"`
    OrganizationID string `json:"organizationId" validate:"required,alphanum,max=32" label:"机构主键"`
    Name         string  `json:"name" validate:"required,max=64" label:"部门名称"`
    // ... 更多字段
}
```

#### Payload 层 (`payload/*.go`)

```go
package payload

import (
    "github.com/coldsmirk/vef-framework-go/api"
)

// DepartmentSearch 部门搜索参数.
type DepartmentSearch struct {
    api.P
    Keyword        string  `json:"keyword" search:"contains,column=name"`
    OrganizationID *string `json:"organizationId"`
}

// DepartmentParams 部门新增/修改参数.
type DepartmentParams struct {
    api.P
    ID                      string  `json:"id"`
    ParentID                *string `json:"parentId" label:"上级部门主键"`
    OrganizationID          string  `json:"organizationId" validate:"required,alphanum,max=32" label:"机构主键"`
    Name                    string  `json:"name" validate:"required,max=64" label:"部门名称"`
    // ... 更多字段
}
```

#### Resource 层 (`resource/*.go`)

```go
package resource

import (
    "github.com/gofiber/fiber/v3"
    "github.com/coldsmirk/vef-framework-go/api"
    "github.com/coldsmirk/vef-framework-go/crud"
    "github.com/coldsmirk/vef-framework-go/sortx"
    "smp-server/internal/sys/model"
    "smp-server/internal/sys/payload"
    "smp-server/internal/sys/schema"
)

type AppResource struct {
    api.Resource
    crud.FindPage[model.App, payload.AppSearch]
    crud.Create[model.App, payload.AppParams]
    crud.Update[model.App, payload.AppParams]
    crud.Delete[model.App]
    crud.DeleteMany[model.App]
}

func NewAppResource() api.Resource {
    return &AppResource{
        Resource: api.NewRPCResource("smp/sys/app"),
        FindPage: crud.NewFindPage[model.App, payload.AppSearch]().
            WithDefaultSort(&sortx.OrderSpec{
                Column: schema.App.SortOrder(),
            }).
            WithAuditUserNames(model.UserModel),
        Create: crud.NewCreate[model.App, payload.AppParams]().
            EnableAudit(),
        Update: crud.NewUpdate[model.App, payload.AppParams]().
            EnableAudit(),
        Delete: crud.NewDelete[model.App]().
            EnableAudit(),
        DeleteMany: crud.NewDeleteMany[model.App]().
            EnableAudit(),
    }
}
```

#### Module 层 (`module.go`)

```go
package sys

import (
    "github.com/coldsmirk/vef-framework-go"
    "smp-server/internal/sys/resource"
    "smp-server/internal/sys/service"
)

var Module = vef.Module(
    "app:sys",
    vef.ProvideAPIResource(resource.NewAppResource),
    vef.ProvideAPIResource(resource.NewUserResource),
    // 更多资源...
)
```

### 2.2 审计模型类型

| 类型 | 包含字段 | 使用场景 |
|------|----------|----------|
| `BaseModel` | - | 纯数据表 |
| `CreationTrackedModel` | created_at, created_by | 仅记录创建 |
| `FullTrackedModel` | created_at, created_by, updated_at, updated_by | 追踪所有修改 |
| `CreationAuditedModel` | id, created_at, created_by, created_by_name | 仅创建审计 |
| `FullAuditedModel` | id, created_at/by, updated_at/by + name | 完整审计 (默认) |

---

## 三、系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端界面                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  SQL 输入器   │  │  文件树选择   │  │  选项配置    │         │
│  │  (TextArea)  │  │  (TreeView)  │  │  (Checkboxes)│         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Tauri Commands (Rust)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  sql_parser  │  │ file_browser │  │ go_generator │         │
│  │  解析 SQL    │  │  浏览/选择   │  │  生成代码    │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      生成的 Go 代码                              │
│  internal/{module}/model/{entity}.go                            │
│  internal/{module}/payload/{entity}.go                          │
│  internal/{module}/resource/{entity}.go                          │
│  internal/{module}/module.go  (更新注册)                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## 四、功能模块详细设计

### 4.1 SQL 解析器 (`sql_parser.rs`)

#### 输入示例

```sql
CREATE TABLE sys_user (
    id VARCHAR(32) PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    email VARCHAR(128),
    phone VARCHAR(20),
    age INT,
    balance DECIMAL(10,2),
    is_active BOOLEAN DEFAULT true,
    meta JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

#### 输出结构

```rust
pub struct ParsedField {
    pub name: String,           // 字段名 (snake_case)
    pub go_name: String,       // Go 命名 (PascalCase)
    pub json_name: String,     // JSON 命名 (camelCase)
    pub go_type: String,       // Go 类型
    pub db_type: String,       // 数据库类型
    pub nullable: bool,        // 是否可空
    pub is_primary_key: bool,  // 是否主键
    pub default_value: Option<String>, // 默认值
    pub comment: Option<String>,      // 注释
}

pub struct ParsedTable {
    pub table_name: String,    // 表名
    pub entity_name: String,   // 实体名 (PascalCase)
    pub fields: Vec<ParsedField>,
}
```

#### SQL 注释解析

支持列注释（`COMMENT '...'`），注释文本将用作：
- Go 字段上方的行注释
- struct tag 中的 `label` 值

```sql
CREATE TABLE sys_user (
    id          VARCHAR(32)  PRIMARY KEY COMMENT '用户主键',
    name        VARCHAR(64)  NOT NULL    COMMENT '用户名称',
    email       VARCHAR(128)             COMMENT '邮箱地址',
    is_active   TINYINT(1)  DEFAULT 1   COMMENT '是否启用',
    sort_order  INT                      COMMENT '排序号'
);
```

生成 Model 字段：

```go
// 用户名称
Name string `json:"name" validate:"required,max=64" label:"用户名称"`
// 邮箱地址
Email *string `json:"email" label:"邮箱地址"`
// 是否启用
IsActive bool `json:"isActive" label:"是否启用"`
```

#### SQL 类型 → Go 类型映射

| SQL 类型 | Go 类型 | 可空时 | 备注 |
|----------|---------|--------|------|
| VARCHAR, CHAR, TEXT, MEDIUMTEXT, LONGTEXT | `string` | `*string` | |
| INT, SMALLINT | `int` | `*int` | |
| BIGINT | `int64` | `*int64` | 避免 32 位平台溢出 |
| TINYINT(1) | `bool` | `*bool` | MySQL 惯用布尔值 |
| TINYINT (其他) | `int` | `*int` | |
| DECIMAL, FLOAT, DOUBLE | `float64` | `*float64` | |
| DATETIME, TIMESTAMP, DATE, TIME | `time.Time` | `*time.Time` | |
| BOOL, BOOLEAN | `bool` | `*bool` | |
| JSON, JSONB | `map[string]any` | - | 不用指针，默认 nil map 可接受 |
| BLOB | `[]byte` | `[]byte` | 不用指针 |

**字段过滤规则**: 以下字段由审计模型自动提供，解析时直接跳过，不生成到 Model/Payload 中：
- `id`, `created_at`, `created_by`, `created_by_name`
- `updated_at`, `updated_by`, `updated_by_name`
- `deleted_at`, `deleted_by`, `tenant_id`

### 4.2 文件浏览器 (`file_browser.rs`)

#### 接口定义

```rust
#[derive(Debug, Clone, Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[tauri::command]
pub fn read_directory(path: String) -> Result<Vec<FileNode>, String> {
    // 读取指定目录，返回树形结构
    // 深度限制: 可配置，默认 5 层，按需懒加载子目录
    // 排除: .git, node_modules, target 等
}

#[tauri::command]
pub fn read_directory_children(path: String) -> Result<Vec<FileNode>, String> {
    // 懒加载：仅读取一层子目录，用于展开节点时按需加载
}

#[tauri::command]
pub fn create_directory(path: String) -> Result<(), String> {
    // 创建目录（含多级）
}

#[tauri::command]
pub fn validate_go_package(path: String) -> Result<bool, String> {
    // 验证路径是否适合作为 Go 包路径
    // 规则: 小写字母、数字、下划线
}
```

#### 目录排除规则

| 排除项 | 原因 |
|--------|------|
| `.git` | 版本控制目录 |
| `node_modules` | 依赖目录 |
| `target` | Rust 编译输出 |
| `dist`, `build` | 前端编译输出 |
| `vendor` | Go vendor 目录 |

### 4.3 Go 代码生成器 (`go_generator.rs`)

#### 生成配置

```rust
pub struct GoCodeGenConfig {
    pub entity_name: String,         // 实体名称 (PascalCase)
    pub module_path: String,         // 模块路径 (如 "sys", "hr/org")
    pub table_name: String,          // 数据库表名
    pub table_alias: String,         // 表别名 (如 "u", "md")
    pub rpc_path: String,            // RPC 路径 (如 "smp/sys/user")
    pub go_module_prefix: String,    // Go module 前缀 (如 "smp-server/internal")
    pub fields: Vec<ParsedField>,    // 解析的字段
    pub enable_find_page: bool,      // 启用分页查询
    pub enable_create: bool,         // 启用创建
    pub enable_update: bool,         // 启用更新
    pub enable_delete: bool,         // 启用删除
    pub enable_delete_many: bool,    // 启用批量删除
    pub enable_sort: bool,           // 启用默认排序 (依赖 schema 文件存在)
    pub enable_audit_user_names: bool, // 启用 WithAuditUserNames (仅 sys 模块)
    pub audit_type: AuditType,       // 审计类型
    pub output_dir: String,          // 输出目录
}

pub enum AuditType {
    FullAudited,        // orm.FullAuditedModel (默认)
    FullTracked,        // orm.FullTrackedModel
    CreationAudited,    // orm.CreationAuditedModel
    CreationTracked,    // orm.CreationTrackedModel
    None,               // 仅 BaseModel
}
```

#### 模板文件

| 模板文件 | 输出位置 |
|----------|----------|
| `go_model.tera` | `{module}/model/{entity}.go` |
| `go_payload.tera` | `{module}/payload/{entity}.go` |
| `go_resource.tera` | `{module}/resource/{entity}.go` |

#### Model 模板 (`go_model.tera`)

```go
package model

import (
    "github.com/coldsmirk/vef-framework-go/orm"
)

// {{ entity_name }} {{ entity_label }}.
type {{ entity_name }} struct {
    orm.BaseModel `bun:"table:{{ table_name }},alias:{{ table_alias }}"`
    orm.{{ audit_model_type }}

{%- for field in fields %}
{%- if field.comment %}
    // {{ field.comment }}
{%- endif %}
    {{ field.go_name }} {{ field.go_type }} `json:"{{ field.json_name }}"{% if field.label %} label:"{{ field.label }}"{% endif %}{% if field.validate %} validate:"{{ field.validate }}"{% endif %}`
{%- endfor %}
}
```

#### Payload 模板 (`go_payload.tera`)

```go
package payload

import (
    "github.com/coldsmirk/vef-framework-go/api"
)

// {{ entity_name }}Search {{ entity_label }}搜索参数.
type {{ entity_name }}Search struct {
    api.P
{%- if search_fields %}
{%- for field in search_fields %}
    {{ field.go_name }} {{ field.go_type }} `json:"{{ field.json_name }}"{% if field.search_annot %} search:"{{ field.search_annot }}"{% endif %}`
{%- endfor %}
{%- else %}
    Keyword string `json:"keyword" search:"contains,column=name"`
{%- endif %}
}

// {{ entity_name }}Params {{ entity_label }}新增/修改参数.
type {{ entity_name }}Params struct {
    api.P

    ID string `json:"id"`
{%- for field in param_fields %}
    {{ field.go_name }} {{ field.go_type }} `json:"{{ field.json_name }}"{% if field.label %} label:"{{ field.label }}"{% endif %}{% if field.validate %} validate:"{{ field.validate }}"{% endif %}`
{%- endfor %}
}
```

#### Resource 模板 (`go_resource.tera`)

```go
package resource

import (
    "github.com/gofiber/fiber/v3"

    "github.com/coldsmirk/vef-framework-go/api"
    "github.com/coldsmirk/vef-framework-go/crud"
{%- if enable_sort %}
    "github.com/coldsmirk/vef-framework-go/sortx"
{%- endif %}

    "{{ module_import_path }}/model"
    "{{ module_import_path }}/payload"
{%- if enable_sort %}
    "{{ module_import_path }}/schema"
{%- endif %}
)

type {{ entity_name }}Resource struct {
    api.Resource
{%- if enable_find_page %}
    crud.FindPage[model.{{ entity_name }}, payload.{{ entity_name }}Search]
{%- endif %}
{%- if enable_create %}
    crud.Create[model.{{ entity_name }}, payload.{{ entity_name }}Params]
{%- endif %}
{%- if enable_update %}
    crud.Update[model.{{ entity_name }}, payload.{{ entity_name }}Params]
{%- endif %}
{%- if enable_delete %}
    crud.Delete[model.{{ entity_name }}]
{%- endif %}
{%- if enable_delete_many %}
    crud.DeleteMany[model.{{ entity_name }}]
{%- endif %}
}

func New{{ entity_name }}Resource() api.Resource {
    return &{{ entity_name }}Resource{
        Resource: api.NewRPCResource("{{ rpc_path }}"),
{%- if enable_find_page %}
        FindPage: crud.NewFindPage[model.{{ entity_name }}, payload.{{ entity_name }}Search]()
{%- if enable_sort %}
            WithDefaultSort(&sortx.OrderSpec{
                Column: schema.{{ entity_name }}.SortOrder(),
            }).
{%- endif %}
{%- if enable_audit_user_names %}
            WithAuditUserNames(model.UserModel),
{%- else %}
            ,
{%- endif %}
{%- endif %}
{%- if enable_create %}
        Create: crud.NewCreate[model.{{ entity_name }}, payload.{{ entity_name }}Params]().
            EnableAudit(),
{%- endif %}
{%- if enable_update %}
        Update: crud.NewUpdate[model.{{ entity_name }}, payload.{{ entity_name }}Params]().
            EnableAudit(),
{%- endif %}
{%- if enable_delete %}
        Delete: crud.NewDelete[model.{{ entity_name }}]().
            EnableAudit(),
{%- endif %}
{%- if enable_delete_many %}
        DeleteMany: crud.NewDeleteMany[model.{{ entity_name }}]().
            EnableAudit(),
{%- endif %}
    }
}
```

#### 生成完成后的提示片段

生成成功后，界面展示以下代码片段，供用户手动复制到 `module.go`：

```go
// 请将以下代码添加到 module.go 的 vef.Module(...) 中：
vef.ProvideAPIResource(resource.New{{ entity_name }}Resource),
```

---

## 五、前端界面设计

### 5.1 页面布局

```
┌─────────────────────────────────────────────────────────────────┐
│  后端 CRUD 代码生成                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────┬─────────────────────────┐ │
│  │  1. SQL 输入                    │  2. 文件树选择           │ │
│  │  ┌───────────────────────────┐ │  ┌───────────────────┐  │ │
│  │  │ CREATE TABLE sys_user (   │ │  │ 📁 smp-server/    │  │ │
│  │  │   id VARCHAR(32),         │ │  │   📁 internal/    │  │ │
│  │  │   name VARCHAR(64) NOT NULL│ │  │     📁 sys/  ◀──  │ │ │
│  │  │ );                        │ │  │       📁 model/   │  │ │
│  │  └───────────────────────────┘ │  │       📁 payload/ │  │ │
│  │  [解析 SQL]  [清空]            │  │       📁 resource/│  │ │
│  │  ┌───────────────────────────┐ │  │       📁 schema/  │  │ │
│  │  │ ✓ 已解析 3 个字段          │ │  └───────────────────┘  │ │
│  │  │ - id: string              │ │  当前路径: sys          │ │
│  │  │ - name: string            │ │                         │ │
│  │  │ - email: *string          │ │  [+ 新建目录]           │ │
│  │  └───────────────────────────┘ │                         │ │
│  └─────────────────────────────────┴─────────────────────────┘ │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  3. 生成配置                                                ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │ 实体名称:  [User    ]  模块路径: [smp-server/internal/] │││
│  │  │ 表名:      [sys_user]  别名:      [u            ]       │││
│  │  │ RPC 路径:  [smp/sys/user                               ]││
│  │  │                                                         │││
│  │  │ 功能选项:                                               │││
│  │  │ ☑ FindPage (分页查询) ☑ Create (创建)                  │││
│  │  │ ☑ Update (更新)     ☑ Delete (删除)                    │││
│  │  │ ☐ DeleteMany (批量删除)                                 │││
│  │  │                                                         │││
│  │  │ 审计类型: ○ FullAudited ● CreationAudited ○ None       │││
│  │  └─────────────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  4. 预览 & 生成                                            ││
│  │  [预览代码]  [生成代码]                                     ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │ 📄 model/user.go     [已存在 - 将覆盖]                  │││
│  │  │ 📄 payload/user.go   [新文件]                           │││
│  │  │ 📄 resource/user.go  [新文件]                           │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │                                                             ││
│  │  ⚠ 请手动将以下代码添加到 module.go:                       ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │ vef.ProvideAPIResource(resource.NewUserResource),       │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │  [复制]                                                     ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 组件结构

```
src/views/codegen/
├── index.vue                    # 主页面
├── components/
│   ├── SqlInput.vue             # SQL 输入组件
│   ├── SqlParserResult.vue      # 解析结果展示
│   ├── FileTreeSelector.vue     # 文件树选择器
│   ├── CodeGenConfig.vue        # 配置表单
│   ├── CodePreview.vue          # 代码预览
│   └── GeneratedFiles.vue       # 生成结果列表
└── composables/
    └── useGoCodegen.ts          # 代码生成逻辑
```

### 5.3 服务接口

```typescript
// src/services/codegen.ts
export const parseSql = (sql: string): Promise<ParsedTable>;
export const readDirectory = (path: string): Promise<FileNode[]>;
export const createDirectory = (path: string): Promise<void>;
export const generateGoCode = (config: GoCodeGenConfig): Promise<GeneratedResult>;
export const previewGoCode = (config: GoCodeGenConfig): Promise<CodePreview>;
```

---

## 六、实现计划

### 阶段 1: Rust 后端 (优先级: 高)

按以下顺序实现，各步骤可独立验证：

| 顺序 | 文件 | 功能 |
|------|------|------|
| 1 | `src/services/codegen/sql_parser.rs` | SQL DDL 解析（含 COMMENT、类型映射、字段过滤） |
| 2 | `src/services/codegen/templates/go_model.tera` | Model 模板 |
| 3 | `src/services/codegen/templates/go_payload.tera` | Payload 模板 |
| 4 | `src/services/codegen/templates/go_resource.tera` | Resource 模板 |
| 5 | `src/services/codegen/go_generator.rs` | 渲染模板、写文件、覆盖保护 |
| 6 | `src/services/codegen/file_browser.rs` | 目录浏览（含懒加载） |
| 7 | `src/commands/codegen.rs` | Tauri 命令注册 |
| 8 | `src/lib.rs` | 模块注册 |

### 阶段 2: 前端界面 (优先级: 中)

| 文件 | 功能 |
|------|------|
| `src/services/codegen.ts` | API 服务（调用 Tauri 命令） |
| `src/views/codegen/index.vue` | 主页面布局 |
| `src/views/codegen/components/SqlInput.vue` | SQL 输入框 + 解析触发 |
| `src/views/codegen/components/SqlParserResult.vue` | 解析结果展示 |
| `src/views/codegen/components/FileTreeSelector.vue` | 文件树选择器（懒加载） |
| `src/views/codegen/components/CodeGenConfig.vue` | 配置表单 |
| `src/views/codegen/components/CodePreview.vue` | 代码预览（Tab 切换） |
| `src/views/codegen/components/GeneratedFiles.vue` | 生成结果 + module.go 提示 |
| `src/views/codegen/composables/useGoCodegen.ts` | 代码生成逻辑 |

### 阶段 3: 测试与优化 (优先级: 低)

- SQL 解析边界测试（无注释、多表、特殊字符）
- TINYINT(1) 与 TINYINT 区分测试
- 代码生成完整性验证（字段过滤、import 去重）
- 文件写入错误处理（权限不足、磁盘满）

---

## 七、验证规则

### 7.1 名称验证

| 字段 | 规则 | 示例 |
|------|------|------|
| 实体名 | PascalCase, 字母开头 | `User`, `OrderItem` |
| JSON 字段名 | camelCase | `userName`, `orderId` |
| Go 包名 | 小写字母、数字、下划线 | `user_service` |
| RPC 路径 | 小写字母、斜杠 | `smp/sys/user` |

### 7.2 路径验证

- 必须在允许的根目录下
- 不允许覆盖关键文件 (如 `go.mod`)
- 包路径必须合法

---

## 八、错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| SQL 解析失败 | 显示具体错误位置和原因（行号 + 列） |
| 目录不存在 | 提示创建或选择其他目录 |
| 文件已存在 | 预览阶段标注"将覆盖"，生成前二次确认 |
| 写入失败 | 显示文件系统错误详情（权限、磁盘空间）|
| 包路径无效 | 实时校验并提示修正 |
| 字段全部被过滤 | 提示"所有字段均为审计字段，请检查 SQL" |

---

## 九、后续扩展

### 9.1 树形结构支持

```rust
// 配置选项
pub enable_find_tree: bool,
pub tree_parent_field: Option<String>,
pub tree_id_field: Option<String>,
```

### 9.2 自定义 Hooks

允许用户添加自定义的 Pre/Post Hooks:

```go
Update: crud.NewUpdate[model.User, payload.UserParams]().
    WithPreUpdate(preUpdateUser).
    WithPostUpdate(postUpdateUser).
    EnableAudit(),
```

### 9.3 关系查询支持

```rust
pub struct RelationConfig {
    pub related_entity: String,
    pub foreign_key: String,
    pub selected_columns: Vec<String>,
}
```

---

## 十、参考资料

- [vef-framework-go](https://github.com/coldsmirk/vef-framework-go) - Go 后端框架
- [Bun ORM](https://bun.uptrace.dev/) - 数据库 ORM
- [Tera Template](https://tera.netlify.app/) - Rust 模板引擎
