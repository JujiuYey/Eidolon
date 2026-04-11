# 代码分析 Agent 技术方案

> 本文档定义 Sco Code App 中代码分析 Agent 的技术实现方案。

## 1. 产品定位

### 核心价值

**像程序员一样思考的代码理解 Agent**

这个 Agent 不是简单的代码问答，而是能够：
- 理解代码的层级结构（页面 → 组件 → 事件 → 函数 → API）
- 追踪数据在代码中的流动路径
- 自动生成代码文档

### 用户场景

- 接手老项目时，快速理解代码结构
- 追踪数据流，知道改动会影响哪些地方
- 自动生成文档，减少手动文档工作

---

## 2. 核心能力

### 2.1 代码层级理解

```
┌─────────────────────────────────────────────────────────────────────┐
│                    程序员的代码理解流程                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1️⃣  UI 层                                                           │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                       │
│  │ 页面A    │───▶│ 组件1    │───▶│ 组件2    │                       │
│  └─────────┘    └────┬────┘    └─────────┘                       │
│                       │                                              │
│  2️⃣  事件层          │                                              │
│  ┌─────────┐    ┌────▼────┐                                      │
│  │ @click   │───▶│ handleX │                                       │
│  └─────────┘    └────┬────┘                                      │
│                       │                                              │
│  3️⃣  函数层          │                                              │
│  ┌─────────┐    ┌────▼────┐                                      │
│  │ async    │───▶│ fetchX() │                                     │
│  └─────────┘    └────┬────┘                                      │
│                       │                                              │
│  4️⃣  数据层          │                                              │
│  ┌─────────┐    ┌────▼────┐                                      │
│  │ GET     │───▶│ /api/x   │                                      │
│  └─────────┘    └─────────┘                                       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 支持的问题类型

| 问题类型 | 示例 |
|----------|------|
| 结构理解 | "这个页面有哪些组件？" |
| 事件追踪 | "点击按钮后发生了什么？" |
| 数据流 | "数据从哪里来，经过哪些处理？" |
| 依赖分析 | "修改这个函数会影响哪些地方？" |
| 文档生成 | "给我讲讲这个模块的业务逻辑" |

---

## 3. Neo4j 图谱模型

### 3.1 节点类型

| 节点类型 | 说明 | 示例 |
|----------|------|------|
| `View` | 页面/视图 | agent/index.vue |
| `Component` | 组件 | ChatPanel.vue |
| `Handler` | 事件处理函数 | handleSend() |
| `Function` | 普通函数 | buildMockReply() |
| `API` | API 调用 | fetchUser() |
| `DataType` | 数据类型 | User |
| `State` | 响应式状态 | files |
| `Prop` | 组件属性 | messages |

### 3.2 边类型

| 边类型 | 说明 | 方向 |
|--------|------|------|
| `CONTAINS` | 包含关系 | View → Component |
| `HAS_EVENT` | 有事件 | Component → Event |
| `TRIGGERS` | 触发处理函数 | Event → Handler |
| `CALLS` | 调用函数/API | Handler → Handler/Function/API |
| `RETURNS` | 返回数据类型 | Function/API → DataType |
| `USES_STATE` | 使用状态 | Handler → State |
| `SETS_STATE` | 设置状态 | Handler → State |
| `HAS_PROP` | 有属性 | Component → Prop |
| `IMPORTS` | 导入模块 | File → File |

### 3.3 示例图谱

以 agent/index.vue 为例：

```cypher
// View 节点
(:View {name: "agent/index.vue", path: "/views/agent/index.vue"})

// Component 节点
(:Component {name: "ChatPanel", path: "/views/agent/components/ChatPanel.vue"})

// Handler 节点
(:Handler {name: "handleSend", params: ["content: string"]})
(:Handler {name: "buildMockReply", params: ["question: string"]})

// 关系
(:View)-[:CONTAINS]->(:Component {name: "ChatPanel"})
(:Component)-[:HAS_EVENT]->(:Event {name: "@submit"})
(:Event)-[:TRIGGERS]->(:Handler {name: "handleSend"})
(:Handler)-[:CALLS]->(:Handler {name: "buildMockReply"})
(:Handler)-[:SETS_STATE]->(:State {name: "messages"})
```

---

## 4. 技术架构

### 4.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         用户界面 (Vue)                                │
│                                                                      │
│   Agent 对话界面 ←── 用户问题 ←── 回答展示                            │
│         │                                                           │
│         │ Tauri Command                                              │
│         ▼                                                           │
├─────────────────────────────────────────────────────────────────────┤
│                         Rust 后端                                    │
│                                                                      │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│   │ Agent Runner │───▶│ 代码分析器    │───▶│  图谱服务     │       │
│   │   (rig)      │    │  (parser)    │    │  (Neo4j)     │       │
│   └──────────────┘    └──────────────┘    └──────────────┘       │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│                         Neo4j                                        │
│                                                                      │
│   节点: View, Component, Handler, Function, API...                   │
│   边: CONTAINS, CALLS, TRIGGERS...                                 │
│   属性: embedding 向量                                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 Rust 依赖

```toml
[dependencies]
# AI 能力
rig-core = { version = "0.33.0", features = ["derive"] }

# Neo4j
neo4rs = "0.9"

# 代码解析
tree-sitter = "0.24"          # AST 解析
tree-sitter-typescript = "0.24" # TypeScript 支持
tree-sitter-vue = "0.4"        # Vue 支持

# 其他
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
```

### 4.3 模块设计

```
src-tauri/src/
├── commands/
│   └── agent.rs              # Tauri 命令入口
├── services/
│   ├── agent_runner.rs       # Agent 运行器 (rig)
│   ├── code_parser.rs        # 代码解析器
│   ├── graph_service.rs      # Neo4j 图谱服务
│   └── embedding.rs           # 向量化服务
└── models/
    └── code_graph.rs         # 图谱数据结构
```

---

## 5. 代码解析策略

### 5.1 多语言支持

| 语言 | 解析方案 | 优先级 |
|------|----------|--------|
| TypeScript/TSX | Tree-sitter | P0 |
| Vue | Tree-sitter + 正则 | P1 |
| Rust | Tree-sitter | P2 |
| Go | Tree-sitter | P3 |

### 5.2 解析内容

```rust
// 每个文件解析出：
struct ParsedFile {
    path: String,
    file_type: FileType,      // .vue, .ts, .tsx, .go...
    
    // 层级结构
    components: Vec<Component>,
    functions: Vec<Function>,
    handlers: Vec<Handler>,
    
    // 关系
    imports: Vec<Import>,
    exports: Vec<Export>,
}

// 组件解析
struct Component {
    name: String,
    props: Vec<Prop>,
    events: Vec<Event>,      // @click, @submit...
    emits: Vec<Emit>,
    state: Vec<State>,       // ref(), reactive()
    lifecycle: Vec<String>,  // onMounted, onUnmounted...
}

// 函数解析
struct Function {
    name: String,
    params: Vec<Param>,
    return_type: Option<String>,
    calls: Vec<String>,      // 调用了哪些函数
    async: bool,
}
```

---

## 6. Agent 工作流程

### 6.1 初始化流程

```rust
async fn initialize_project(project_path: &str) -> Result<()> {
    // 1. 扫描项目文件
    let files = scan_source_files(project_path)?;
    
    // 2. 解析每个文件
    for file in files {
        let parsed = parse_file(&file)?;
        
        // 3. 提取节点和边
        let nodes = extract_nodes(&parsed);
        let edges = extract_edges(&parsed);
        
        // 4. 生成 embedding
        for node in &nodes {
            node.embedding = generate_embedding(&node.to_text());
        }
        
        // 5. 存入 Neo4j
        save_to_graph(&nodes, &edges).await?;
    }
    
    // 6. 创建索引
    create_vector_index().await?;
    
    Ok(())
}
```

### 6.2 问答流程

```rust
async fn answer_question(question: &str) -> Result<String> {
    // 1. 理解问题类型
    let question_type = classify_question(question);
    
    // 2. 构建 prompt
    let prompt = match question_type {
        QuestionType::Structure => build_structure_prompt(question),
        QuestionType::DataFlow => build_dataflow_prompt(question),
        QuestionType::Document => build_document_prompt(question),
        QuestionType::General => build_general_prompt(question),
    };
    
    // 3. 查询相关上下文
    let context = search_graph(&question).await?;
    
    // 4. 调用模型
    let answer = call_model(&prompt, &context).await?;
    
    // 5. 返回回答
    Ok(answer)
}
```

---

## 7. 文档生成能力

### 7.1 支持的文档类型

| 文档类型 | 内容 |
|----------|------|
| 模块概览 | 模块功能、入口点、依赖关系 |
| 数据流图 | 数据从输入到输出的完整路径 |
| API 文档 | 接口说明、参数、返回值 |
| 组件文档 | 组件属性、事件、插槽 |
| 调用图 | 函数/组件之间的调用关系 |

### 7.2 文档格式

```markdown
# 模块: agent

## 概述
代码分析 Agent 的主页面，提供项目文件浏览和对话功能。

## 文件结构
```
agent/
├── index.vue          # 主页面入口
└── components/
    ├── AgentLayout.vue     # 布局组件
    ├── ChatPanel.vue       # 对话面板
    ├── MessageList.vue     # 消息列表
    ├── ProjectFileTree.vue # 文件树
    └── ToolTimeline.vue    # 工具时间线
```

## 数据流
```
projectPath
    ↓
scanProjectFiles()
    ↓
files (状态)
    ↓
ProjectFileTree → selectedFilePath → selectedFileContent → ChatPanel
```

## 核心组件

### ChatPanel
**路径**: `/views/agent/components/ChatPanel.vue`

**属性**:
- `messages: AgentMessage[]` - 消息列表
- `busy: boolean` - 是否正在处理

**事件**:
- `@submit` - 发送消息
```

---

## 8. 实现优先级

### Phase 1: MVP (2-3 周)

| 任务 | 说明 |
|------|------|
| Neo4j 集成 | 连接、写入、查询 |
| 基础代码解析 | 只解析 TypeScript 文件 |
| 基础关系提取 | IMPORTS, CALLS |
| 简单问答 | 结构理解类问题 |
| 文档生成 | Markdown 格式输出 |

### Phase 2: 增强 (2-3 周)

| 任务 | 说明 |
|------|------|
| Vue 支持 | 组件、事件、响应式状态 |
| 向量搜索 | 语义相似度查询 |
| 多语言支持 | Rust, Go |
| 交互式图谱 | 可视化展示 |

### Phase 3: 完善 (持续)

| 任务 | 说明 |
|------|------|
| 实时更新 | 文件变化时更新图谱 |
| 对话记忆 | 记住之前的上下文 |
| 自定义文档模板 | 符合团队规范的文档 |

---

## 9. 注意事项

### 9.1 性能考虑

- 代码解析是 CPU 密集型操作，应放在后台任务
- 大项目解析可能需要较长时间，考虑增量更新
- Neo4j 查询需要建立索引

### 9.2 准确性考虑

- 静态代码分析的局限性（动态调用无法追踪）
- 需要在分析结果中标注置信度
- 保留代码原文作为参考

### 9.3 Neo4j 部署

- 开发阶段：Neo4j Aura (云端)
- 生产阶段：考虑 Neo4j Desktop 或自托管

---

## 10. 后续扩展

### 10.1 MCP 集成

未来可以接入 MCP 生态：
- 文件系统 MCP - 读写文件
- Git MCP - 分析提交历史
- 终端 MCP - 执行命令

### 10.2 多 Agent 协作

不同 Agent 负责不同语言/领域：
- 前端 Agent - 专注 Vue/React
- 后端 Agent - 专注 Go/Rust
- 全栈 Agent - 理解整体架构

---

## 附录：Neo4j 快速入门

### 启动 Neo4j Aura

1. 访问 https://neo4j.com/cloud/aura/
2. 创建免费实例
3. 获取连接 URI 和凭证

### 基础 Cypher

```cypher
// 创建节点
CREATE (v:View {name: "index.vue", path: "/views/index.vue"})

// 创建关系
MATCH (a:View {name: "index.vue"}), (b:Component {name: "ChatPanel"})
CREATE (a)-[:CONTAINS]->(b)

// 查询
MATCH (v:View)-[:CONTAINS*1..3]->(end)
WHERE v.name = "index.vue"
RETURN end

// 向量搜索
MATCH (n)
WHERE n.embedding IS NOT NULL
WITH n, gds.alpha.similarity.cosine(n.embedding, [0.1, 0.2, ...]) AS sim
RETURN n ORDER BY sim DESC LIMIT 5
```

---

*文档创建时间: 2026-04-07*
