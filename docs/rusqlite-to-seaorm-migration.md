# Rusqlite + SQLite 迁移到 SeaORM + PostgreSQL 教程

## 一、核心概念对比

| 层级 | 原方案 (rusqlite) | 新方案 (SeaORM) |
|------|------------------|-----------------|
| 数据库 | SQLite (本地文件) | PostgreSQL (远程) |
| 驱动 | rusqlite + tokio-rusqlite | sqlx (异步 PostgreSQL 驱动) |
| ORM | 手写 SQL | SeaORM Active Record 模式 |
| 迁移 | rusqlite_migration | SeaORM 迁移 |

---

## 二、依赖变更 (Cargo.toml)

### 移除的依赖

```toml
# 删除
rusqlite = { version = "0.32.1", features = ["bundled", "serde_json"] }
tokio-rusqlite = "0.6.0"
rusqlite_migration = "1.3.1"
```

### 新增的依赖

```toml
# 添加
sea-orm = { version = "0.12", features = ["sqlx-postgres", "runtime-tokio-native-tls", "macros", "debug"] }
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono"] }
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15"  # 用于加载 .env 环境变量
```

### 完整的 dependencies 区块

```toml
[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
log = "0.4"
tauri = { version = "2.10.3", features = [] }
tauri-plugin-log = "2"
tauri-plugin-dialog = "2"
rig-core = { version = "0.33.0", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs"] }
walkdir = "2"
globset = "0.4"
regex = "1"
sha2 = "0.10"
hex = "0.4"

# 数据库 - 新方案
sea-orm = { version = "0.12", features = ["sqlx-postgres", "runtime-tokio-native-tls", "macros", "debug"] }
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono"] }
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15"

schemars = "1"
```

---

## 三、数据库连接配置

### 1. 创建 .env 文件

在 `src-tauri/` 目录下创建 `.env` 文件（记得加入 .gitignore）：

```env
DATABASE_URL=postgres://用户名:密码@主机地址:5432/数据库名
```

示例：

```env
DATABASE_URL=postgres://postgres:mysecretpassword@192.168.1.100:5432/sco_code_app
```

### 2. 修改 db/connection.rs

删除原来的 `AppDatabase` 结构体，改用 SeaORM 的连接方式：

```rust
// 新的 connection.rs
use sea_orm::DbConn;
use sqlx::postgres::PgPoolOptions;
use std::env;

pub struct AppDatabase {
    pub conn: DbConn,
}

// 异步初始化连接池
impl AppDatabase {
    pub async fn open() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL 环境变量未设置".to_string())?;

        // 创建 PostgreSQL 连接池
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await
            .map_err(|e| format!("连接 PostgreSQL 失败: {}", e))?;

        // 用 sea_orm 包装连接池
        let conn = sea_orm::Database::from_sqlx_postgres_pool(pool)
            .await
            .map_err(|e| format!("初始化 SeaORM 失败: {}", e))?;

        Ok(Self { conn })
    }
}
```

### 3. 修改 main.rs 中的初始化逻辑

原来：

```rust
let db = AppDatabase::open(&db_path).await;
```

改为：

```rust
let db = AppDatabase::open().await;
```

---

## 四、Model 改为 SeaORM Entity

### 原有 Model 模式（以 Project 为例）

```rust
// 原来的 models/project.rs
#[derive(Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}
```

### 新 Entity 模式

```rust
// 新的 entities/project.rs（建议在 src/entities/ 目录）
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub description: String,
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

### 主要改动

| 原字段 | 新写法 | 说明 |
|--------|--------|------|
| `id: i64` | `id: i64` + `#[sea_orm(primary_key)]` | 主键标记 |
| `created_at: String` | `created_at: DateTimeUtc` | 使用 chrono 的 UTC 时间类型 |
| `pub id: i64` | `id: i64` (私有) | SeaORM 默认字段是私有的 |

---

## 五、Repository 改造

### 原有 Repository（rusqlite）

```rust
// 原来的 db/repositories/project.rs
use crate::db::AppDatabase;
use crate::models::Project;

pub struct ProjectRepository<'a> {
    db: &'a AppDatabase,
}

impl<'a> ProjectRepository<'a> {
    pub async fn list(&self) -> Result<Vec<Project>, String> {
        let mut stmt = self.db.conn
            .prepare("SELECT id, name, description, created_at, updated_at FROM projects")
            .await
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .await
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows)
    }
}
```

### 新 Repository（SeaORM）

```rust
// 新的 db/repositories/project.rs
use crate::db::AppDatabase;
use crate::entities::{Entity as ProjectEntity, Model as Project};
use sea_orm::EntityTrait;

pub struct ProjectRepository<'a> {
    db: &'a AppDatabase,
}

impl<'a> ProjectRepository<'a> {
    pub async fn list(&self) -> Result<Vec<Project>, String> {
        ProjectEntity::find()
            .all(&self.db.conn)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Project>, String> {
        ProjectEntity::find_by_id(id)
            .one(&self.db.conn)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn create(&self, name: &str, description: &str) -> Result<Project, String> {
        use sea_orm::ActiveModel;

        let active_model = sea_orm::ActiveModel {
            name: sea_orm::Set(name.to_string()),
            description: sea_orm::Set(description.to_string()),
            ..Default::default()
        };

        ProjectEntity::insert(active_model)
            .exec(&self.db.conn)
            .await
            .map_err(|e| e.to_string())?;

        // 查询返回刚插入的数据
        self.list().await?.into_iter().last().ok_or("插入失败".to_string())
    }

    pub async fn update(&self, id: i64, name: &str, description: &str) -> Result<Project, String> {
        use sea_orm::ActiveModel;

        let project: Project = self.find_by_id(id)
            .await?
            .ok_or("记录不存在")?;

        let mut active_model: sea_orm::ActiveModel = project.into();
        active_model.name = sea_orm::Set(name.to_string());
        active_model.description = sea_orm::Set(description.to_string());

        ProjectEntity::update(active_model)
            .exec(&self.db.conn)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete(&self, id: i64) -> Result<(), String> {
        use sea_orm::EntityTrait;

        ProjectEntity::delete_by_id(id)
            .exec(&self.db.conn)
            .await
            .map_err(|e| e.to_string())
    }
}
```

### Repository 方法对比

| 操作 | SeaORM 写法 |
|------|------------|
| 查所有 | `Entity::find().all(conn)` |
| 按 ID 查 | `Entity::find_by_id(id).one(conn)` |
| 条件查询 | `Entity::find().filter(Column::Name.eq("xxx")).one(conn)` |
| 插入 | `Entity::insert(active_model).exec(conn)` |
| 更新 | `Entity::update(active_model).exec(conn)` |
| 删除 | `Entity::delete_by_id(id).exec(conn)` |
| 计数 | `Entity::find().count(conn)` |

---

## 六、迁移脚本

### 创建迁移目录

```bash
mkdir -p src-tauri/src/migrations
```

### 编写迁移文件

```rust
// src/migrations/mod.rs
use sea_orm_migration::prelude::*;

pub struct M20240101CreateProjects;

#[async_trait::async_trait]
impl MigrationTrait for M20240101CreateProjects {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Projects::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Projects::Name).string().not_null())
                    .col(ColumnDef::new(Projects::Description).text().not_null().default("''"))
                    .col(ColumnDef::new(Projects::CreatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Projects::UpdatedAt).timestamp_with_time_zone().not_null().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Projects::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum Projects {
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}
```

### 在 main.rs 中运行迁移

```rust
use sea_orm_migration::MigratorTrait;

#[tokio::main]
async fn main() {
    // 初始化数据库连接
    let db = AppDatabase::open().await.expect("数据库连接失败");

    // 运行迁移
    db.conn.run_migrations().await.expect("迁移失败");

    // 后续逻辑...
}
```

---

## 七、常用操作示例

### 条件查询

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

// 查询名称包含关键字的项目
let projects = ProjectEntity::find()
    .filter(Projects::Name.like("%关键字%"))
    .all(&db.conn)
    .await?;
```

### 分页

```rust
use sea_orm::Iterable;

let paginator = ProjectEntity::find()
    .paginate(&db.conn, 20);  // 每页 20 条

let page = paginator.fetch_page(1).await?;  // 第 2 页
```

### 关联查询（如果未来需要）

```rust
// 在 Entity 中定义关联
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::project_glossary_term::Entity")]
    ProjectGlossaryTerm,
}

impl Related<super::project_glossary_term::Entity> for Entity {
    fn def() -> RelationDef {
        Relation::ProjectGlossaryTerm.def()
    }
}

// 查询时使用
let projects = Entity::find()
    .find_with_related(super::project_glossary_term::Entity)
    .all(&db.conn)
    .await?;
```

---

## 八、ID 类型变更

SQLite 用 `i64` 作为自增 ID，PostgreSQL 也用 `i64` 自增，所以 `Model` 里的 `id: i64` 不需要改。

但如果未来想用 UUID 做主键：

```rust
#[sea_orm(primary_key)]
pub id: Uuid,  // 需要改模型和迁移
```

---

## 九、调试技巧

### 开启 SQL 日志

```rust
let conn = sea_orm::Database::from_sqlx_postgres_pool(pool)
    .await
    .map_err(|e| format!("初始化 SeaORM 失败: {}", e))?
    .enable_logging();  // 打印所有 SQL
```

### 查看生成的 SQL

```rust
use sea_orm::QueryTrait;

let query = Entity::find().filter(Column::Name.eq("test")).build(&db.conn);
println!("SQL: {}", query.sql);
```

---

## 十、迁移检查清单

- [ ] 修改 Cargo.toml，替换依赖
- [ ] 创建 .env 文件，设置 DATABASE_URL
- [ ] 修改 db/connection.rs，重写连接逻辑
- [ ] 创建 entities/ 目录，编写 SeaORM Entity
- [ ] 修改 repositories，使用 SeaORM 查询接口
- [ ] 创建 migrations/ 目录，编写迁移脚本
- [ ] 在 main.rs 中调用 run_migrations()
- [ ] 删除不再需要的旧模型文件（models/ 目录下的 .rs 文件）
- [ ] 删除不再需要的旧迁移文件（db/migrations.rs）
- [ ] 测试各个 CRUD 操作是否正常

---

## 十一、参考资料

- [SeaORM 官方文档](https://www.sea-ql.org/SeaORM/)
- [SeaORM Examples](https://github.com/SeaQL/sea-orm/tree/master/examples)
- [sqlx 文档](https://docs.rs/sqlx/latest/sqlx/)
- [PostgreSQL 连接字符串格式](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING)
