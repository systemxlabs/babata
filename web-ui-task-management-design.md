# Babata Web UI - 任务管理界面设计方案

## 📋 需求概述

任务管理界面是 Babata 系统的核心功能模块，用于展示和管理所有任务，包括根任务列表浏览、任务树结构展示和任务详情查看。

---

## 🎨 页面布局设计

### 整体布局

```
┌─────────────────────────────────────────────────────────────────────────┐
│  🔷 Babata              [搜索]     [🔔]    [👤]                         │
├──────────┬──────────────────────────────────────────────────────────────┤
│          │  任务管理                                                                │
│  📊 概览  │  ┌────────────────────────────────────────────────────────────┐    │
│          │  │  [状态: 全部 ▼] [Agent: 全部 ▼]           [🔍 搜索...]      │    │
│  📋 任务  │  └────────────────────────────────────────────────────────────┘    │
│          │                                                                │
│  🤖 智能体│  ┌────────────────────────────────────────────────────────────┐    │
│          │  │  📁 分析系统日志文件                        🤖 planner      │    │
│  📡 频道  │  │  🟡 running  ⏱️ 2026-04-10 19:06  📎 3个子任务 [▼] [🗑️] │    │
│          │  ├────────────────────────────────────────────────────────────┤    │
│  🛠️ 技能 │  │    ├─ ○ 子任务1                          🟢 completed      │    │
│          │  │    ├─ ○ 子任务2                          🟡 running        │    │
│          │  │    └─ ○ 子任务3                          ⚪ pending        │    │
│          │  ├────────────────────────────────────────────────────────────┤    │
│  ⚙️ 设置 │  │  📁 代码审查任务                            🤖 code-reviewer│    │
│          │  │  🟢 completed  ⏱️ 2026-04-10 18:30  📎 0个子任务 [▼] [🗑️]│    │
│          │  └────────────────────────────────────────────────────────────┘    │
│          │                                                                │
│          │  ┌───────────────────────────────────────────────────────────┐ │
│          │  │  ◀  1  2  3  ...  10  ▶   共 95 条记录   每页 20 条      │ │
│          │  └───────────────────────────────────────────────────────────┘ │
│          │                                                                │
└──────────┴──────────────────────────────────────────────────────────────┘
```

---

## 📐 组件划分

### 1. 页面级组件

```
src/pages/Tasks/
├── Tasks.tsx                    # 任务管理页面主组件
├── Tasks.css                    # 页面样式
├── components/
│   ├── TaskListHeader/          # 列表头部（筛选、搜索）
│   │   ├── TaskListHeader.tsx
│   │   └── index.ts
│   ├── TaskTreeItem/            # 任务树节点项
│   │   ├── TaskTreeItem.tsx
│   │   ├── TaskTreeItem.css
│   │   └── index.ts
│   ├── TaskStatusBadge/         # 任务状态标签
│   │   ├── TaskStatusBadge.tsx
│   │   └── index.ts
│   ├── TaskPagination/          # 分页组件
│   │   ├── TaskPagination.tsx
│   │   └── index.ts
│   └── EmptyState/              # 空状态展示
│       ├── EmptyState.tsx
│       └── index.ts
└── hooks/
    ├── useTasks.ts              # 任务数据管理 Hook
    └── useTaskTree.ts           # 任务树展开/折叠状态管理
```

### 2. 弹窗组件

```
src/components/TaskDetailModal/
├── TaskDetailModal.tsx          # 任务详情弹窗主组件
├── TaskDetailModal.css
├── components/
│   ├── TaskInfoPanel/           # 任务基本信息面板
│   │   ├── TaskInfoPanel.tsx
│   │   └── index.ts
│   ├── TaskDirectoryTab/        # 任务目录标签页
│   │   ├── TaskDirectoryTab.tsx
│   │   ├── FileTree.tsx         # 文件树组件
│   │   ├── FileViewer.tsx       # 文件内容查看器
│   │   └── index.ts
│   ├── TaskLogsTab/             # 任务日志标签页
│   │   ├── TaskLogsTab.tsx
│   │   └── index.ts
│   └── TaskActionBar/           # 操作按钮栏
│       ├── TaskActionBar.tsx
│       └── index.ts
└── index.ts
```

---

## 🔧 数据结构定义

### 任务基础类型

```typescript
// src/types/task.ts

/** 任务状态 */
export type TaskStatus = 'running' | 'completed' | 'failed' | 'canceled' | 'paused';

/** 任务类型 */
export type TaskType = 'roottask' | 'subtask';

/** 基础任务信息 */
export interface Task {
  task_id: string;
  description: string;
  agent: string;
  status: TaskStatus;
  parent_task_id: string | null;
  root_task_id: string;
  created_at: number;           // Unix timestamp (seconds)
  never_ends: boolean;
}

/** 根任务（包含子任务统计） */
export interface RootTask extends Task {
  subtask_count: number;
  children?: Task[];            // 展开时填充的子任务
}

/** 任务筛选条件 */
export interface TaskFilter {
  status?: TaskStatus | 'all';
  agent?: string;
  search?: string;
  page: number;
  pageSize: number;
}

/** 任务列表响应 */
export interface TaskListResponse {
  tasks: RootTask[];
  total: number;
  page: number;
  pageSize: number;
}

/** 文件信息 */
export interface TaskFile {
  name: string;
  path: string;
  isDirectory: boolean;
  size?: number;
  modifiedAt?: number;
}

/** 任务详情 */
export interface TaskDetail extends Task {
  children: Task[];
  files: TaskFile[];
  logs: string;
  final_response?: string;
}

/** 任务控制操作 */
export type TaskControlAction = 'pause' | 'resume' | 'cancel';
```

### 状态颜色映射

```typescript
// src/constants/taskStatus.ts

export const TASK_STATUS_CONFIG = {
  running: {
    color: '#F59E0B',      // 琥珀色
    bgColor: 'bg-amber-500/10',
    textColor: 'text-amber-500',
    borderColor: 'border-amber-500/30',
    label: '运行中',
    icon: 'Loader2',       // Lucide 图标名
    animate: true,
  },
  completed: {
    color: '#10B981',      // 翠绿色
    bgColor: 'bg-emerald-500/10',
    textColor: 'text-emerald-500',
    borderColor: 'border-emerald-500/30',
    label: '已完成',
    icon: 'CheckCircle2',
    animate: false,
  },
  failed: {
    color: '#EF4444',      // 红色
    bgColor: 'bg-red-500/10',
    textColor: 'text-red-500',
    borderColor: 'border-red-500/30',
    label: '失败',
    icon: 'XCircle',
    animate: false,
  },
  canceled: {
    color: '#6B7280',      // 灰色
    bgColor: 'bg-gray-500/10',
    textColor: 'text-gray-500',
    borderColor: 'border-gray-500/30',
    label: '已取消',
    icon: 'Ban',
    animate: false,
  },
  paused: {
    color: '#F97316',      // 橙色
    bgColor: 'bg-orange-500/10',
    textColor: 'text-orange-500',
    borderColor: 'border-orange-500/30',
    label: '已暂停',
    icon: 'PauseCircle',
    animate: false,
  },
} as const;
```

---

## 🔌 API 接口定义

### 任务列表相关

```typescript
// GET /api/tasks/roots
// 获取根任务列表（支持分页和筛选）
interface GetRootTasksRequest {
  status?: 'running' | 'completed' | 'failed' | 'canceled' | 'paused' | 'all';
  agent?: string;
  search?: string;           // 按描述搜索
  page?: number;             // 默认 1
  pageSize?: number;         // 默认 20
}

interface GetRootTasksResponse {
  tasks: RootTask[];
  total: number;
  page: number;
  pageSize: number;
}

// GET /api/tasks/:id/children
// 获取任务的子任务列表
interface GetTaskChildrenResponse {
  children: Task[];
}

// GET /api/tasks/count
// 获取任务统计数量
interface GetTaskCountRequest {
  status?: TaskStatus;
  isRoot?: boolean;          // 是否只统计根任务
}

interface GetTaskCountResponse {
  count: number;
}
```

### 任务操作相关

```typescript
// DELETE /api/tasks/:id
// 删除任务（及其所有子任务）
interface DeleteTaskResponse {
  success: boolean;
  deletedCount: number;      // 删除的任务总数（包含子任务）
}

// POST /api/tasks/:id/control
// 控制任务状态（暂停/恢复/取消）
interface ControlTaskRequest {
  action: 'pause' | 'resume' | 'cancel';
}

interface ControlTaskResponse {
  success: boolean;
  task: Task;
}
```

### 任务详情相关

```typescript
// GET /api/tasks/:id
// 获取任务详情
interface GetTaskDetailResponse extends TaskDetail {}

// GET /api/tasks/:id/files
// 获取任务目录文件列表
interface GetTaskFilesResponse {
  files: TaskFile[];
}

// GET /api/tasks/:id/files/content
// 获取任务文件内容
interface GetTaskFileContentRequest {
  path: string;              // 文件路径
}

interface GetTaskFileContentResponse {
  content: string;
  path: string;
  size: number;
}

// GET /api/tasks/:id/logs
// 获取任务日志
interface GetTaskLogsResponse {
  logs: string;
  lastModified: number;
}
```

### Agent 筛选相关

```typescript
// GET /api/agents
// 获取所有 Agent 列表（用于筛选下拉框）
interface GetAgentsResponse {
  agents: {
    name: string;
    description: string;
  }[];
}
```

---

## 🎯 交互流程设计

### 1. 根任务列表展示流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   页面加载   │────▶│  获取根任务  │────▶│  渲染列表   │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  获取Agent  │
                    │   列表      │
                    └─────────────┘
```

**详细流程：**
1. 页面加载时，默认获取第 1 页根任务（pageSize=20）
2. 同时获取 Agent 列表用于筛选下拉框
3. 渲染任务列表，每个任务项显示：
   - 展开/折叠按钮（根据子任务数显示）
   - 任务描述（前 50 字符，超出省略）
   - Agent 名称
   - 状态标签（带颜色）
   - 创建时间（格式化：YYYY-MM-DD HH:mm）
   - 子任务数量
   - 操作按钮组（展开/删除）

### 2. 任务树展开/折叠流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  点击展开   │────▶│  获取子任务  │────▶│  渲染子树   │
│   按钮      │     │   (API)     │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                      │
       ▼                                      ▼
┌─────────────┐                       ┌─────────────┐
│ 已缓存数据  │──────────────────────▶│   直接渲染   │
│   存在？    │                       │             │
└─────────────┘                       └─────────────┘
```

**详细流程：**
1. 用户点击展开按钮（▶）
2. 检查是否已缓存子任务数据
   - 已缓存：直接渲染子任务树
   - 未缓存：调用 `GET /api/tasks/:id/children` 获取子任务
3. 展开子任务树，按钮变为折叠状态（▼）
4. 子任务以缩进形式展示在父任务下方
5. 点击折叠按钮（▼）收起子任务树

### 3. 任务详情弹窗流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  点击任务   │────▶│   打开弹窗   │────▶│  获取详情   │
│  行/描述    │     │  (显示loading)│    │   (API)    │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │  并行获取：  │
                                        │  - 任务信息  │
                                        │  - 文件列表  │
                                        │  - 日志内容  │
                                        └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │  渲染弹窗    │
                                        │ 内容(默认显示│
                                        │  任务目录页) │
                                        └─────────────┘
```

**详细流程：**
1. 用户点击任务行或任务描述
2. 打开任务详情弹窗，显示加载状态
3. 并行发起三个请求：
   - `GET /api/tasks/:id` - 获取任务基本信息
   - `GET /api/tasks/:id/files` - 获取任务目录文件
   - `GET /api/tasks/:id/logs` - 获取任务日志
4. 数据返回后渲染弹窗内容
5. 默认显示"任务目录"标签页

### 4. 文件查看流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  点击文件   │────▶│  获取文件   │────▶│  渲染文件   │
│  树节点     │     │   内容      │     │   内容      │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                     │
       ▼                                     ▼
┌─────────────┐                       ┌─────────────┐
│  是目录？   │──────────────────────▶│  展开/折叠   │
│             │                       │  子目录      │
└─────────────┘                       └─────────────┘
       │ 否
       ▼
┌─────────────┐
│  选中文件   │
│  高亮显示   │
└─────────────┘
```

### 5. 任务删除流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  点击删除   │────▶│  确认弹窗   │────▶│  调用删除   │
│   按钮      │     │  (二次确认) │     │   API       │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │  删除成功？  │
                                        └─────────────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          │ 是                 │                    │ 否
                          ▼                    │                    ▼
                   ┌─────────────┐             │             ┌─────────────┐
                   │  刷新列表   │             │             │  显示错误   │
                   │  显示成功   │             │             │   提示      │
                   │   消息      │             │             │             │
                   └─────────────┘             │             └─────────────┘
                          │                    │
                          └────────────────────┘
```

### 6. 状态筛选流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  选择筛选   │────▶│  更新筛选   │────▶│  重置分页   │
│   条件      │     │   状态      │     │  到第1页    │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────┐
                                        │  重新获取   │
                                        │  任务列表   │
                                        └─────────────┘
```

---

## 🧩 组件详细设计

### 1. TaskListHeader 组件

```typescript
interface TaskListHeaderProps {
  filters: TaskFilter;
  agents: string[];
  onFilterChange: (filters: Partial<TaskFilter>) => void;
  onSearch: (search: string) => void;
}
```

**UI 结构：**
```
┌────────────────────────────────────────────────────────────┐
│  状态: [全部 ▼]  Agent: [全部 ▼]              [🔍 搜索...] │
└────────────────────────────────────────────────────────────┘
```

**筛选选项：**
- 状态筛选：全部 / 运行中 / 已完成 / 失败 / 已暂停 / 已取消
- Agent 筛选：全部 / planner / frontend-developer / code-reviewer / ...
- 搜索框：支持按任务描述关键词搜索（实时或回车触发）

### 2. TaskTreeItem 组件

```typescript
interface TaskTreeItemProps {
  task: Task;
  level: number;              // 层级深度（用于缩进）
  isExpanded: boolean;
  children?: Task[];
  onToggle: (taskId: string) => void;
  onDelete: (taskId: string) => void;
  onClick: (task: Task) => void;
}
```

**UI 结构（根任务）：**
```
┌────────────────────────────────────────────────────────────────┐
│ [▼] 📁 任务描述...               🤖 agent   🟡 running  ⏱️ 时间 │
│                                   [👁️查看] [🗑️删除]            │
├────────────────────────────────────────────────────────────────┤
│    ├─ ○ 子任务1                          🟢 completed         │
│    ├─ ○ 子任务2                          🟡 running           │
│    └─ ○ 子任务3                          ⚪ pending           │
└────────────────────────────────────────────────────────────────┘
```

**样式规范：**
- 根任务：带背景卡片样式，hover 效果
- 子任务：无背景，左侧缩进 24px * level
- 展开按钮：有子任务时显示，无子任务时显示占位符
- 状态图标：使用 Lucide 图标，运行中状态带旋转动画

### 3. TaskStatusBadge 组件

```typescript
interface TaskStatusBadgeProps {
  status: TaskStatus;
  showLabel?: boolean;        // 是否显示文字标签
  size?: 'sm' | 'md' | 'lg';
}
```

**样式：**
- 小尺寸：仅图标（用于子任务列表）
- 中尺寸：图标 + 文字（用于根任务列表）
- 大尺寸：图标 + 文字 + 背景色块（用于详情弹窗）

### 4. TaskDetailModal 组件

```typescript
interface TaskDetailModalProps {
  taskId: string | null;
  isOpen: boolean;
  onClose: () => void;
}
```

**UI 结构：**
```
┌─────────────────────────────────────────────────────────────────┐
│  任务详情                                        [✕] 关闭      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  📄 分析系统日志文件                                    │   │
│  │  ─────────────────────────────────────────────────────  │   │
│  │  ID: f91c7ae8-c905-4c95-9f27-5c6f7c8f9a0b               │   │
│  │  状态: 🟡 running                                       │   │
│  │  Agent: planner                                         │   │
│  │  创建时间: 2026-04-10 19:06:07                          │   │
│  │  子任务数: 3                                            │   │
│  │  Never Ends: false                                      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  [⏸ 暂停] [▶ 恢复] [✕ 取消]                                   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  [任务目录] [任务日志]                                  │   │
│  │  ─────────────────────────────────────────────────────  │   │
│  │                                                         │   │
│  │  任务目录内容 / 任务日志内容                            │   │
│  │                                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**尺寸：**
- 宽度：800px（桌面端），移动端全屏
- 高度：最大 80vh，支持内容区域滚动

### 5. TaskDirectoryTab 组件

```typescript
interface TaskDirectoryTabProps {
  taskId: string;
}
```

**UI 结构：**
```
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────┬───────────────────────────────────┐   │
│  │  📁 task-home/      │  📄 final-response.md             │   │
│  │  ├─ 📄 final-       │  ───────────────────────────────  │   │
│  │  │   response.md   │                                   │   │
│  │  ├─ 📁 logs/        │  [文件内容显示区域]                │   │
│  │  │   └─ 📄         │                                   │   │
│  │  │       task.log  │  # 任务执行结果                   │   │
│  │  └─ 📁 outputs/     │                                   │   │
│  │                     │  任务已成功完成...                 │   │
│  │                     │                                   │   │
│  └─────────────────────┴───────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**交互：**
- 左侧文件树：支持点击展开/折叠目录，点击文件在右侧查看
- 右侧文件查看器：
  - 显示文件名和路径
  - 支持代码高亮（根据文件类型）
  - 支持滚动查看
  - Markdown 文件支持预览模式切换

### 6. TaskLogsTab 组件

```typescript
interface TaskLogsTabProps {
  taskId: string;
  logs: string;
}
```

**UI 结构：**
```
┌─────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  📋 任务执行日志                           [🔄 刷新]    │   │
│  │  ─────────────────────────────────────────────────────  │   │
│  │  ┌─────────────────────────────────────────────────┐   │   │
│  │  │ 2026-04-10 19:06:10 [INFO] 任务开始执行...       │   │   │
│  │  │ 2026-04-10 19:06:11 [INFO] 初始化组件...         │   │   │
│  │  │ 2026-04-10 19:06:12 [INFO] 获取数据完成          │   │   │
│  │  │ 2026-04-10 19:06:15 [WARN] 发现警告信息          │   │   │
│  │  │ 2026-04-10 19:06:20 [INFO] 任务执行完成          │   │   │
│  │  │                                                   │   │   │
│  │  │ [日志内容支持滚动查看，自动滚动到底部]            │   │   │
│  │  └─────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

**功能：**
- 日志自动滚动到底部（可选）
- 支持手动刷新
- 支持复制日志内容
- 不同日志级别显示不同颜色（INFO-白色, WARN-黄色, ERROR-红色）

---

## 🎨 样式规范

### 颜色系统（与现有设计一致）

```css
/* 主色调 */
--color-primary: #2563EB;       /* 蓝色 - 主要操作 */
--color-success: #10B981;       /* 绿色 - 成功状态 */
--color-warning: #F59E0B;       /* 琥珀色 - 运行中 */
--color-error: #EF4444;         /* 红色 - 失败 */
--color-orange: #F97316;        /* 橙色 - 暂停 */

/* 深色模式背景 */
--bg-primary: #0F172A;          /* slate-900 */
--bg-secondary: #1E293B;        /* slate-800 - 卡片背景 */
--bg-tertiary: #334155;         /* slate-700 - hover */

/* 文字颜色 */
--text-primary: #F8FAFC;        /* slate-50 */
--text-secondary: #94A3B8;      /* slate-400 */
--text-muted: #64748B;          /* slate-500 */

/* 边框颜色 */
--border-primary: #334155;      /* slate-700 */
--border-secondary: #1E293B;    /* slate-800 */
```

### 间距系统

```css
/* 列表项间距 */
--task-item-padding: 16px;
--task-item-gap: 12px;
--task-tree-indent: 24px;       /* 每级缩进 */

/* 卡片样式 */
--card-border-radius: 12px;
--card-padding: 16px;
--card-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
```

### 字体规范

```css
/* 字体大小 */
--font-xs: 12px;      /* 时间、辅助信息 */
--font-sm: 14px;      /* 次要文字 */
--font-md: 16px;      /* 正文 */
--font-lg: 18px;      /* 标题 */
--font-xl: 24px;      /* 页面标题 */

/* 字重 */
--font-normal: 400;
--font-medium: 500;
--font-semibold: 600;
--font-bold: 700;
```

---

## 📱 响应式设计

### 断点适配

| 断点 | 宽度 | 布局调整 |
|------|------|----------|
| Desktop | ≥1280px | 完整布局，侧边栏展开 |
| Laptop | ≥1024px | 侧边栏可收起为图标模式 |
| Tablet | ≥768px | 侧边栏隐藏，汉堡菜单 |
| Mobile | <768px | 单列布局，弹窗全屏 |

### 移动端适配

**任务列表：**
- 卡片垂直布局，信息分行显示
- 操作按钮组简化（仅保留主要操作）
- 子任务缩进减少为 16px

**任务详情弹窗：**
- 全屏显示，顶部关闭按钮
- 文件浏览器改为上下布局（文件树在上，内容在下）
- 标签页改为滑动切换

---

## ⚡ 性能优化

### 数据加载策略

1. **分页加载**：根任务列表使用分页，避免一次性加载大量数据
2. **懒加载子任务**：子任务按需加载，展开时才获取
3. **缓存机制**：
   - 已展开的任务子树数据缓存
   - 文件内容缓存（LRU 策略）
   - 列表数据 SWR (Stale-While-Revalidate) 策略

### 渲染优化

1. **虚拟列表**：当任务数量超过 100 时启用虚拟滚动
2. **组件懒加载**：任务详情弹窗组件按需加载
3. **防抖搜索**：搜索输入使用 300ms 防抖

### 实时更新

1. **轮询机制**：运行中任务列表每 10 秒自动刷新
2. **增量更新**：仅更新状态变化的任务，避免全量刷新

---

## 🔒 错误处理

### API 错误处理

```typescript
// 统一错误处理策略
const handleApiError = (error: ApiError) => {
  switch (error.code) {
    case 'TASK_NOT_FOUND':
      toast.error('任务不存在或已被删除');
      break;
    case 'DELETE_FAILED':
      toast.error('删除失败，请稍后重试');
      break;
    case 'CONTROL_FAILED':
      toast.error('操作失败，任务状态可能已改变');
      break;
    default:
      toast.error('网络错误，请检查连接');
  }
};
```

### 空状态处理

- **无任务**：显示空状态插画 + "暂无任务" 提示
- **筛选无结果**：显示 "没有找到匹配的任务" + 清除筛选按钮
- **加载失败**：显示错误信息 + 重试按钮

---

## ✅ 实现检查清单

### Phase 1: 基础组件
- [ ] TaskListHeader 组件（筛选、搜索）
- [ ] TaskStatusBadge 组件（状态标签）
- [ ] TaskTreeItem 组件（任务树节点）
- [ ] TaskPagination 组件（分页）

### Phase 2: 任务列表页面
- [ ] 根任务列表获取和展示
- [ ] 任务树展开/折叠功能
- [ ] 状态筛选功能
- [ ] 搜索功能
- [ ] 删除任务功能

### Phase 3: 任务详情弹窗
- [ ] TaskDetailModal 组件框架
- [ ] TaskInfoPanel 任务信息面板
- [ ] TaskDirectoryTab 任务目录标签
- [ ] TaskLogsTab 任务日志标签
- [ ] 文件查看器组件

### Phase 4: 优化和测试
- [ ] 响应式适配
- [ ] 性能优化（虚拟列表、缓存）
- [ ] 错误处理
- [ ] 无障碍支持

---

## 📖 参考资源

- 设计风格：参考 `web-ui-design.md` 和 `web-ui-home-design.md`
- 图标库：Lucide React (https://lucide.dev)
- 颜色参考：Tailwind CSS Slate 色系
- 设计灵感：GitHub Actions 工作流列表、Vercel Dashboard

---

**设计文档版本**: v1.0
**设计日期**: 2026-04-11
**设计师**: Frontend Developer
