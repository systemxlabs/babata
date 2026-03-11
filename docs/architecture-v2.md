# Architecture v2

## 1. 目标

Babata v2 的目标不是构建一个“内置大量业务逻辑的 agent 框架”，而是构建一个“让大模型主导执行逻辑的最小运行时”。

系统只提供四类基础能力：

1. 把任何输入 prompt 统一建模为 task。
2. 为 task 提供可并行、可中断、可恢复的执行容器。
3. 为 task 提供最小但可靠的状态持久化机制。
4. 为 task 提供工具、记忆、人工输入、子任务这些通用能力，但不在运行时内固化具体策略。

除基础机制外，任务拆分、工具选择、步骤规划、重试策略、上下文组织、何时结束，原则上都交给大模型决定。

这里进一步约束为：

- 控制状态由 runtime 负责，但只保留最薄的一层，例如 task 创建、tokio 任务启动、状态落库。
- 语义状态尽可能由大模型负责，例如当前进度、完成了什么、下一步做什么、哪些假设仍成立。
- task 的可见生命周期只保留少量稳定状态：`running`、`done`、`canceled`、`paused`；状态由数据库维护，而不是通过目录位置编码。

## 2. 核心设计原则

### 2.1 Prompt Is Task

每一个 prompt 都是一个独立 task。无论它来自用户输入、定时任务、webhook、上游 task 还是人工恢复操作，进入系统后都必须先变成 task。

### 2.2 Runtime Is Minimal

运行时只负责：

- 调度
- 持久化
- 并发执行
- 工具调用编排
- 权限边界

运行时不负责：

- 内置 workflow DSL
- 固定 planner
- 固定 executor graph
- 预定义多 agent 协作拓扑
- 业务级状态机
- 系统级防重
- 系统级恢复决策

### 2.3 LLM Owns The Logic

系统不预设“先规划再执行”或“先检索再回答”这类流程。每个 task 会在自己的 loop 中持续与模型 API 多轮交互，采用 ReAct 模式推进，直到任务完成。

进一步地，task 的工作进度也可以主要由模型自己维护。运行中的 task 可以周期性产出一份 `progress` 文档，作为该 task 当前语义状态的 checkpoint。

### 2.4 Progress Is The Recovery Surface

系统不要求维护复杂的系统级真相源。对长 task 来说，`progress.md` 就是主要恢复面。

### 2.5 Resume After Restart Is A First-Class Feature

进程重启、模型超时、工具失败都不是异常边缘场景，而是日常路径。task 在执行循环中的任何时刻被中断后，如何继续执行，主要由模型基于 `task.md` 和 `progress.md` 自主决定。

## 3. 系统抽象

v2 只保留五个核心抽象。

### 3.1 Task

Task 是系统中的基本执行单元。

在目录模型下，task 不再要求有独立的结构化快照文件。它的核心由三部分组成：

- `task_id`
- `task.md`
- `progress.md`

其中：

- `task_id` 使用 uuid，作为 task 的稳定主键，同时也是其目录名。
- `task.md` 描述任务目标、输入和完成标准。
- `progress.md` 描述当前进度、已完成事项、未完成事项和下一步建议。

其他运行时控制信息，例如状态、父子关系、调度元数据，应优先进入数据库；即使有附属文件，也不应替代 `task.md` 和 `progress.md` 成为任务的核心表达。

### 3.2 Tool

Tool 只是一种受控副作用接口。运行时不解释工具背后的业务含义，只保证：

- 有稳定 schema
- 有权限边界
- 有超时
- 有审计日志
- 有幂等键
- 有结果持久化

### 3.3 Artifact

任何大体积或结构化产物都不直接塞进 task 状态，而是写为 artifact。

典型 artifact：

- 文件
- 网页快照
- 表格
- 图片
- 检索结果集
- 中间代码产物
- 长上下文摘要

task 目录中的其他文件可以引用 artifact 的路径、名称和用途。

`progress` 文档本质上也是一种特殊 artifact，只是它承担的是 task 的语义 checkpoint。

### 3.4 Memory

Memory 不再是“神秘长期记忆系统”，而是普通能力模块：

- 短期 memory：task 内上下文与执行摘要
- 长期 memory：跨 task 可检索知识
- 工作记忆：当前树状任务共享的上下文片段

是否读 memory、写 memory、写什么，全由模型决定；运行时只负责提供可用接口。

## 4. 总体架构

### 4.1 逻辑分层

v2 建议拆成五层：

1. Ingress Layer
   把用户消息、定时器、API 请求、系统回调统一转成 `task_created`。
2. Task Store
   存 task 元数据、状态索引和 task 目录内容。
3. Tokio Task Runtime
   每个 task 对应一个 tokio 异步任务，负责读取 task 目录、调用模型、执行动作、更新 progress。
4. Capability Layer
   提供 tools、memory、artifact、human input、subtask API。
5. Observability & Control Plane
   提供追踪、取消、观察、人工介入。

### 4.2 `.babata` 目录布局

v2 采用“统一 task 目录 + 数据库存状态”的最小持久化层，核心布局如下：

```text
.babata/
  task.db
  tasks/
    <task_id>/
```

其中：

- `<task_id>` 是 uuid。
- `.babata/tasks/<task_id>/` 永远是该 task 的工作目录，无论其当前状态是 `running`、`done`、`canceled` 还是 `paused`。
- `.babata/task.db` 保存 task 元数据、状态、索引和关系。
- task 状态必须从数据库读取，不能从目录位置推断。

每个 task 目录建议至少包含：

- `task.md`
  由人类或模型写入，描述任务目标、输入和完成标准
- `progress.md`
  由大模型维护的当前进度文档
- `artifacts/`
  该 task 产生的文件、摘要、代码、截图等产物

### 4.3 核心判断

Babata v2 不应该围绕“单个 agent loop”设计，而应该围绕“task 目录 + tokio 异步任务”设计。

原因：

- 单 loop 天然限制并行度。
- 单 loop 把运行中状态藏在内存里，不利于重启续跑。
- 每个 task 独立映射为 tokio 任务，更符合“prompt 即 task”的系统模型。

## 5. Task 生命周期

v2 建议把 task 生命周期收缩到四个用户可见状态：

- `running`
- `done`
- `canceled`
- `paused`

状态含义：

- `running`: task 仍在活动集合中，可继续运行、暂停后继续、或由 runtime 在重启后重新启动。
- `done`: task 已完成，不再参与调度；其目录仍保留在 `.babata/tasks/<task_id>/` 下。
- `canceled`: task 被人工或系统明确取消，不再继续运行；其目录仍保留，供审计和历史查询。
- `paused`: task 被临时暂停，不进入调度；恢复时再切回 `running`。

重要约束：

- `running` 不是“正在占用 CPU”，而是“当前仍是活跃任务”。
- task 内部的等待、重试、人工补充、子任务等待，不再提升为 task 主状态，而是写进 `progress` 文档或调度元数据。
- `done` 后 task 不删除，也不移动目录，只更新数据库状态。
- `canceled` 和 `paused` 都不删除目录；它们只是不同的控制状态。
- runtime 只调度 `running` task；`done`、`canceled`、`paused` 都不应被自动拉起。
- 续跑主要依赖 `task.md`、`progress.md` 和产物目录，而不是进程内缓存。

## 6. 并行模型

### 6.1 默认并行

每个 task 默认彼此独立，可并行执行。系统不需要“主 agent 串行处理消息”；相反，所有新 prompt 进入后立即创建为活动 task，并发运行在各自的 tokio 异步任务上。

### 6.2 树状并行

模型可在执行中创建子任务：

- 研究任务拆为多个检索子任务
- 编码任务拆为多个文件级子任务
- 操作任务拆为多个外部系统调用子任务

父 task 不直接依赖复杂调度状态，而是把“等待哪些子任务”写进 `progress`，并在适当时机继续运行。

### 6.3 并行不是共享会话

v2 不把“一个用户连续发来的多条消息”自动视作同一串行会话。默认策略应当是：

- 每个 prompt 独立成 task
- 是否关联到已有 task，由模型或接入层显式决定
- 共享信息通过 memory 或 artifact 完成，而不是靠长寿命内存会话

这样才能获得真正的可扩展并行性。

## 7. 可恢复执行

### 7.1 恢复机制

task 能在重启后继续执行，依赖两个机制：

1. 目录持久化
   `task.md`、`progress.md`、`artifacts/` 在磁盘上持续存在。
2. 模型自恢复
   模型基于当前目录内容决定是否继续、从哪里继续、是否需要改写计划。

但 v2 不要求每次恢复都把所有历史细节重新塞给模型。推荐把恢复分成两层：

1. 运行时恢复
   runtime 重启后查询数据库中所有 `running` task，并为每个 task 恢复一个 tokio 任务；`paused` 和 `canceled` task 不自动恢复。
2. 语义恢复
   模型基于 `task.md`、最新 `progress.md` 和产物，恢复“现在做到哪了、下一步该干什么”。

### 7.2 执行循环中的提交点

恢复粒度不应只看整个 task 是否完成，而应看 task 在执行循环里推进到了哪里。也就是说：

- 模型返回了下一步计划，但 progress 尚未更新，重启后可以从上一个 checkpoint 恢复。
- 工具已经执行但结果尚未写入 artifact，则恢复后由模型自行判断如何继续。
- 子任务已创建，则恢复时通过目录和子任务文件判断当前状态。

### 7.3 Progress Checkpoint

对于长 task，建议引入模型维护的 `progress` 文档。

`progress` 文档建议包含：

- 当前目标
- 已完成事项
- 当前结论
- 未完成事项
- 正在等待的外部依赖
- 已创建的子任务及其用途
- 关键 artifact 引用
- 关键假设与风险
- 推荐恢复提示词，说明下一步应该如何继续

推荐在以下时机触发 progress 更新：

- 每执行若干轮后
- 完成一批工具调用后
- 创建完一组子任务后
- 进入 `waiting_human` / `waiting_subtasks` 前
- 主动 `yield` 前
- 上下文接近窗口上限时

设计上，这份文档可以由模型自己总结和覆写；runtime 只负责版本化保存，不理解其中业务语义。

### 7.4 恢复时如何使用 Progress

task 重启后，runtime 不必把全量历史重新喂给模型，而可以采用：

1. 读取 `task.md`。
2. 读取最新一版 `progress` 文档。
3. 读取已有 artifact。
4. 让模型基于 `task.md + progress.md + artifacts` 恢复工作状态。
5. 继续执行下一轮 task loop。

这本质上把恢复责任交给模型本身，而 runtime 只负责把 task 目录重新交回给模型。

### 7.5 边界条件

`progress` 文档由模型负责，因此它天然带有不确定性。

原因：

- 模型总结可能遗漏信息。
- 模型可能把临时假设误写成既定事实。
- 崩溃可能发生在 progress 尚未来得及更新时。

因此推荐原则是：

- artifact 是硬结果。
- progress 是语义 checkpoint。
- 当 `progress.md` 和 artifact 不一致时，由模型自己基于目录内容重建最合理的继续路径。

## 8. 上下文重建

v2 不维护“超大常驻 prompt 缓冲区”。上下文在每轮 task loop 动态构建。

建议组成：

1. 固定系统提示
2. 当前 task 的 `task.md`
3. 当前 task 的 `progress.md`
4. 需要引用的 artifact 内容或摘要
5. 相关 memory 检索结果
6. 必要的父任务上下文
7. 工具 schema

这里的关键点不是“把所有历史都塞给模型”，而是“让上下文构建器成为一个最小可替换机制”。

建议将上下文构建分为两段：

- Deterministic Context Builder
  从 `task.md`、`progress.md`、artifact、memory 中提取候选上下文。
- LLM Compression Pass
  在上下文过长时，由模型把候选上下文压缩成下一轮 task loop 所需的工作摘要。

对于长 task，最新 `progress.md` 应成为上下文构建的第一优先级输入。

## 9. ReAct 执行模型

每个 task 的 tokio 异步任务会持续运行一个 loop，直到 task 完成为止。

每一轮 loop 的基本结构是：

1. 读取 `task.md`
2. 读取 `progress.md`
3. 读取已有 artifact
4. 调用模型 API
5. 让模型基于当前上下文进行 ReAct 推进
6. 执行模型决定的动作
7. 更新 `progress.md` 和产物
8. 进入下一轮，直到完成

这里的重点不是定义一套复杂协议，而是明确执行风格：

- 模型在每轮里先思考，再决定动作
- 动作执行结果会回到下一轮上下文中
- task 会一直跑下去，直到模型明确完成任务

runtime 不需要理解复杂角色系统，只需要支持这类持续的 ReAct 循环。

## 10. 最小持久化模型

建议至少有以下存储：

### 10.1 `tasks/`

对应目录 `.babata/tasks/<task_id>/`。

每个目录都是一个 task 的完整工作目录，至少包含 `task.md`、`progress.md` 和 `artifacts/`。

目录本身不表达状态；它只负责保存 task 内容。

### 10.2 `task.db`

对应文件 `.babata/task.db`。

这是 task 元数据的真相源，至少应包含一张 `tasks` 表，用于记录：

- `task_id`
- `status`
- `parent_task_id`
- `root_task_id`
- `created_at`
- `updated_at`
- `completed_at`

其中：

- `status` 只保留 `running`、`done`、`canceled`、`paused`。
- runtime 是否恢复一个 task，取决于数据库里该 task 是否为 `running`。
- task 完成时更新 `status = done` 与 `completed_at`，而不是移动目录。
- task 被取消时更新 `status = canceled`；task 被暂停时更新 `status = paused`；两者都不进入待调度集合。

### 10.3 待调度 Task 获取

运行时通过查询 `tasks.status = 'running'` 获取待调度 task。

最小实现可以直接是数据库查询：

- 查询 `status = 'running'` 的 task
- 按需要结合 `updated_at`、创建时间或调度字段排序

`done`、`canceled`、`paused` 都不是待调度 task。

重点不是引入复杂队列系统，而是保证 runtime 能发现应启动的 task。

### 10.4 `artifacts`

在目录模型下，默认直接放在每个 task 的 `artifacts/` 子目录中。

其中 `progress.md` 也可以视为一种特殊 artifact，只是通常放在 task 目录顶层，便于恢复时直接读取。

### 10.5 `task_relations`

父子任务、依赖、等待关系建议进入数据库，而不是散落在 task 目录中。

最小实现可以先复用 `tasks.parent_task_id` 和 `tasks.root_task_id`；如果后续需要多依赖边，再增加独立关系表。

### 10.6 `task_history_lookup`

这是一个能力而不是必须独立成表，但系统必须支持按 `task_id` 同时读取：

- `.babata/tasks/<task_id>/` 下的 `task.md`、`progress.md` 和 `artifacts/`
- `.babata/task.db` 中该 task 的状态、关系和完成时间

供大模型恢复上下文或复用历史结果。

## 11. Tokio Task Runtime

单个 task runtime 的循环应当非常简单：

1. 为一个 task 启动一个 tokio 异步任务。
2. 读取 task 当前上下文。
3. 调用模型得到下一动作。
4. 执行动作并更新 `progress.md` 与产物目录。
5. 根据执行结果更新数据库状态：继续执行则保持 `running`，完成则标记为 `done`，人工取消则标记为 `canceled`，人工暂停则标记为 `paused`。

runtime 启动时的职责也很简单：

1. 查询数据库中 `status = 'running'` 的 task
2. 为每个活动 task 启动一个 tokio 异步任务
3. 不决定如何恢复，只把 task 目录内容交给模型

这种模型下，runtime 不再需要抢占调度、租约续约或系统级防重。

## 12. 人工介入

“完全 AI 驱动”不等于“系统中不能出现 human-in-the-loop”。正确的理解是：

- 默认逻辑由 AI 驱动
- 人工输入是一个能力，而不是主流程控制器

因此 v2 应支持：

- 模型主动请求补充信息
- 模型请求审批
- 人工对 task 进行暂停、取消、重试、注释、接管

这些都应当通过 task 目录内容进入系统，而不是绕开 task 模型。

这里的“暂停”或“等待审批”不意味着引入新的 task 主状态；它们更多表现为 `progress.md` 中的待处理条件，以及 tokio task 的暂停。

## 13. 失败模型

v2 应明确区分三类失败：

### 13.1 Transient Failure

例如模型超时、网络错误、临时限流。处理方式是单轮执行失败后再进入下一轮，不改变 task 语义。

### 13.2 Capability Failure

例如工具执行失败、权限不足、参数非法。处理方式是把失败结果反馈给模型，让模型决定修正、降级还是结束。

### 13.3 Semantic Failure

例如模型走错方向、长期无进展、重复调用工具。处理方式不是在运行时写死大量规则，而是提供：

- 最大轮数限额
- 最大成本限额
- 循环检测
- 人工接管入口

## 14. 非目标

以下内容不应成为 v2 的核心：

- 把所有能力塞进一个超级 system prompt
- 设计复杂的 YAML/DSL workflow 引擎
- 在运行时里硬编码 planner / researcher / coder / reviewer 等角色
- 依赖长寿命内存对象保存执行现场
- 让队列语义和聊天会话语义耦合

这些都违背“最小机制、逻辑交给模型”的方向。

## 15. 推荐实现顺序

建议按下面顺序实现：

1. `.babata/tasks/ + task.db` 基础模型
2. `tasks` 表与最小状态流转
3. task.md / progress.md 模板
4. tokio task runtime
5. 统一模型动作协议
6. 工具调用
7. 子任务树与等待机制
8. artifact 存储
9. memory 接口
10. 人工输入通道与观测、取消

## 16. 历史任务查询

已完成 task 被标记为 `done` 后，大模型应当能够根据 `task_id` 查询历史任务。

这个能力用于：

- 恢复父子任务链路
- 复用已完成任务的结论或 artifact
- 在新 task 中引用旧 task 结果
- 审计某个历史任务到底做了什么

推荐返回内容：

- `task_id`
- `status`
- `task.md`
- `final_output`
- `latest_progress`
- `artifacts`
- `parent_task_id`
- `root_task_id`
- `completed_at`

在统一目录模型下，这个查询本质上就是读取 `.babata/tasks/<task_id>/` 下的 `task.md`、`progress.md` 和 `artifacts/`，再结合 `.babata/task.db` 中该 task 的状态与时间字段。

## 17. 一句话定义 v2

Babata v2 是一个以 task 为中心、每个 task 对应一个 tokio 异步任务、任务目录统一存放在 `.babata/tasks/` 下、`running` / `done` / `canceled` / `paused` 由数据库维护，并把进度维护与恢复决策尽量交给大模型决定的最小 agent runtime。
