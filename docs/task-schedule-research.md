# 任务排期（时间块）调研

> 2026-10-13。目标：任务管理器升级到第三档——小时级时间块（time blocking），对齐 Sunsama / Motion 的形态，但保持「个人分身」定位。先收集开源方案，再定设计与实现。

## 一、要解决的问题

现在的任务只有**截止日期**（什么时候必须交），没有**排期**（打算哪天哪个时段做）。「帮我排一排」只是一段一次性文字建议，不落到任务上。第三档要补的是：

1. 任务可以放到某天某时段（时间块）
2. 有日程视图（周/日时间网格）可视化
3. 拖拽调整（从清单拖上日历、块之间挪动、拉长缩短）
4. AI 排期从"给建议"升级为"直接排出时间块，过目后应用"

## 二、开源完整应用（产品形态参考，不直接用代码）

| 应用                                                                              | 形态要点                                                     | 对我们的参考价值                         |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------------- |
| [Super Productivity](https://github.com/super-productivity)（Angular，GPL）       | 任务 + timeboxing + 时间跟踪 + 番茄钟                        | 「任务预估时长」字段设计；做完打勾的闭环 |
| [Tududi](https://github.com/chrisvel/tududi)（自托管）                            | 日视图 + 领域/项目/标签的每日规划                            | 「今天」视图的信息密度参考               |
| [Atomic time blocking app](https://github.com/rush86999/atomic-time-blocking-app) | 免费在线 timeblocking 工具                                   | README 极简，代码参考价值低，略          |
| Sunsama / Motion / Akiflow / FlowSavvy（闭源）                                    | 商业标杆：两栏式（左任务右日历）、每日 shutdown、AI 自动排期 | 交互范式的事实标准，抄交互不抄代码       |

商业产品里 Motion/FlowSavvy 的「AI 自动排期」（给约束，AI 把任务铺进日历）正是我们 `task_assistant` 已有 AI 能力的自然延伸。

## 三、日历组件库（直接可用的候选）

| 库                                                                                | Vue 3 支持 | 拖拽/缩放 | License | 备注 |
| --------------------------------------------------------------------------------- | ---------- | --------- | ------- | ---- |
| **结论（2026-10-13 修订）：自研 `sag-time-grid`，EventCalendar 降级为算法参考。** |

初版结论曾推荐 EventCalendar（交互免费、MIT、CSS 变量主题），用户看过默认样式后不认可观感——「能换肤」不等于「能换成 Sunsama 质感」，深改第三方组件层叠结构代价高且升级易碎。结合本应用自攒 `sag-*` 组件的惯例（SagBarChart/SagDonutChart/SagAiPanel），改为自研：

- 渲染层（CSS Grid + 绝对定位块）工作量小，视觉完全长在 shadcn token 体系上；
- 交互层（拖动/缩放/外部拖入/15 分钟吸附/撞块并排）是工作量大头，Pointer Events 手写约几百行，可控；
- EventCalendar（MIT）源码可作 overlap 布局与 snap 计算的参考实现；
- 我们 v1 只需周/日视图 + 单日内拖拽，用不到通用库一半功能。

组件库对比保留如下，供未来回头看（例如需要多资源视图或月视图矩阵时）：

| 库                                                 | Vue 3 支持                         | 拖拽/缩放                                               | License    | 备注                                                                                                         |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------ |
| **vkurko/calendar (EventCalendar)**                | 框架无关 core，自己包一层 Vue 组件 | ✅ 全免费（Interaction 插件）                           | MIT        | 35kb 零依赖，CSS 变量主题（`--ec-*`），`dateFromPoint(x,y)` 可实现外部拖入；**默认观感偏旧，深改样式代价高** |
| [Schedule-X](https://schedule-x.dev)               | ✅ 官方 `@schedule-x/vue`          | ⚠️ **拖拽/缩放/拖建都是 premium 付费插件**（文档标 ⭐） | core MIT   | 最现代的 API 设计（v4、Temporal），还有 MCP server；但时间块的核心交互全在付费墙里                           |
| [FullCalendar v6](https://fullcalendar.io/license) | ✅ 官方 `@fullcalendar/vue3`       | ✅ 免费含 timeGrid + 交互                               | 标准版 MIT | 老牌、文档全、生态大；premium 仅资源视图（$480/年）。包体更大，Vue 包装层略糙                                |
| [vue-cal](https://github.com/antoniandre/vue-cal)  | 原生 Vue 2/3                       | ⚠️ 拖拽仅限单日内、限制多                               | MIT        | 无依赖，但交互完成度不如上面三家                                                                             |
| qalendar / Svar / Mobiscroll                       | —                                  | —                                                       | —          | qalendar 维护弱；Svar/Mobiscroll 商业授权，排除                                                              |

Schedule-X 若未来愿意付费可回来看；它的 MCP server 思路（让外部 AI 助手操作日历）与我们已有的对外 MCP 服务方向契合，v1 不需要。

## 四、落地草图（初步，正式设计等拍板后出）

**数据模型**（migration 0010，tasks 表加列）：

- `start_at TEXT` / `end_at TEXT`：ISO 本地时间（如 `2026-10-14T14:00`），空串 = 未排期。任务有时间即时间块，v1 一个任务最多一个块（拖离日历=清空，任务保留——Sunsama 同款语义）。
- `estimate_minutes INTEGER`：预估时长，AI 排期与拖入日历时的默认块长。

**UI**（/tasks 页加视图切换：清单 / 日程）：

- 日程视图两栏：左侧未排期任务清单（可拖出），右侧自研 `sag-time-grid` 周视图（15 分钟一格吸附，now 指示线，默认滚到 8:00），交互蓝本 = Sunsama 两栏式。
- 清单拖到网格 → `dateFromPoint` 算落点 → 写 start/end；网格框选 → 快速新建带时间的任务；点块 → 编辑；拖动/缩放块 → 更新时间。
- 主题：Vue 包装组件内把 `--ec-*` 映射到 shadcn token，深色模式挂 `ec-dark` 跟 `useTheme`。

**AI 排期 v2**：「帮我排一排」输出结构化块建议（复用现有候选模式：预览 → 一键应用），约束 = 工作时段 + 未排期任务 + 优先级 + 截止 + 预估时长。LLM 走已配置的 `task_assistant` 默认模型。

**后端**：tasks 表加列 + DTO/mapper 跟进；`upsert_task` 带新字段即可，无新命令；AI 命令返回严格 JSON 由 Rust 解析（对齐 api_generate 模式）。

**明确不做（v1）**：外部日历订阅（ICS/Google/Outlook 同步）、多资源视图、任务拆多个块、甘特图。外部日历将来可用 background events 呈现。

## 五、风险与注意

- 自研网格的难点集中在交互层：拖动/缩放用 Pointer Events 手写（吸附 15 分钟、最小块长、拖拽中撞块并排预览），时间计算全部抽纯函数进 `__tests__`（对齐 chart 工具的待遇）；overlap 布局与 snap 算法参考 EventCalendar（MIT）源码思路。
- 外部拖入（清单→网格）是唯一没有现成范式的交互，需重点测试（拖影、落点高亮、松手建块）。
- 时区一律本地、存无时区 ISO，避免 Tauri 端/前端换算分叉（due_date 已是纯文本日期，同风格）。
