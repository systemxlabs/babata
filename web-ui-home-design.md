# Babata Web UI 首页设计方案

## 📋 需求分析

### 用户明确要求的功能
1. **系统状态概览** - 显示运行中的根任务数、总任务数、Agent数量、Skill数量
2. **正在运行的根任务列表** - 展示部分活跃任务
3. **创建新任务的输入框** - 快速创建任务入口

---

## 🎨 页面布局设计

```
┌─────────────────────────────────────────────────────────────┐
│  🧠 Babata System Dashboard                    [🔍搜索]    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │
│  │ 🏃 运行  │ │ 📋 总任务 │ │ 🤖 Agents│ │ 🛠️ Skills│          │
│  │   12    │ │   156   │ │    5    │ │    8    │          │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │
├─────────────────────────────────────────────────────────────┤
│  🚀 快速创建任务                                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ [🤖 planner ▼] [输入任务描述...          ] [创建]  │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  ▶️ 正在运行的根任务 (12)              [刷新] [查看全部]   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 📄 分析系统日志文件                    🤖 planner   │   │
│  │ ⏱️ 5分钟前  🔄 running  📎 3个子任务  [暂停] [取消]  │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ 📄 生成API文档                         🤖 frontend   │   │
│  │ ⏱️ 12分钟前 🔄 running  📎 0个子任务  [暂停] [取消]  │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │ 📄 数据库迁移检查                      🤖 general    │   │
│  │ ⏱️ 28分钟前 🔄 running  📎 2个子任务  [暂停] [取消]  │   │
│  └─────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  ✅ 最近完成的任务 (显示最近3-5个)                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 📄 更新依赖包                          🤖 planner   │   │
│  │ ✅ 1小时前完成                                   [👁️] │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔌 后端 API 对接

### 已有的 API 端点

| 功能 | 端点 | 方法 | 说明 |
|------|------|------|------|
| 统计任务 | `/api/tasks/count?status=running` | GET | 获取运行中任务数 |
| 统计总任务 | `/api/tasks/count` | GET | 获取总任务数 |
| 任务列表 | `/api/tasks?status=running&limit=10` | GET | 获取运行中任务 |
| Agent列表 | `/api/agents` | GET | 获取所有Agent |
| Skill列表 | `/api/skills` | GET | 获取所有Skill |
| 创建任务 | `/api/tasks` | POST | 创建新任务 |
| 控制任务 | `/api/tasks/{id}/control` | POST | 暂停/恢复/取消 |

### 任务数据结构
```typescript
interface Task {
  task_id: string;
  description: string;
  agent: string;
  status: 'running' | 'completed' | 'failed' | 'canceled' | 'paused';
  parent_task_id?: string;
  root_task_id: string;
  created_at: number;  // timestamp
  never_ends: boolean;
}
```

---

## 💡 建议的额外功能

### 高优先级建议
1. **实时状态自动刷新**
   - 轮询间隔：10-30秒
   - 显示最后更新时间
   - 手动刷新按钮

2. **任务详情抽屉**
   - 点击任务展开右侧抽屉
   - 显示完整任务信息
   - 快速查看任务日志

3. **快速操作按钮**
   - 暂停/恢复任务
   - 取消任务
   - 查看详情

### 中优先级建议
4. **最近完成的任务区域**
   - 显示最近 3-5 个完成的任务
   - 可快速查看结果

5. **系统健康指示器**
   - 显示系统状态
   - 如果有失败任务标红提示

6. **任务搜索/过滤**
   - 按关键词搜索
   - 按Agent过滤
   - 按时间范围过滤

### 可选功能
7. **深色/浅色主题切换**
8. **任务执行时间图表**
9. **系统通知中心**

---

## 🛠️ 技术实现建议

### 前端技术栈
- **框架**: React + TypeScript (已有)
- **构建工具**: Vite (已有)
- **样式**: CSS Modules / Tailwind CSS
- **状态管理**: React Hooks (useState, useEffect)
- **数据获取**: 原生 fetch / axios

### 组件结构
```
src/
├── components/
│   ├── Dashboard/
│   │   ├── Dashboard.tsx       # 首页主组件
│   │   ├── Dashboard.css
│   │   └── index.ts
│   ├── StatsCard/
│   │   ├── StatsCard.tsx       # 统计卡片
│   │   ├── StatsCard.css
│   │   └── index.ts
│   ├── TaskList/
│   │   ├── TaskList.tsx        # 任务列表
│   │   ├── TaskItem.tsx        # 单个任务项
│   │   └── index.ts
│   ├── CreateTask/
│   │   ├── CreateTask.tsx      # 创建任务表单
│   │   └── index.ts
│   └── TaskDrawer/
│       ├── TaskDrawer.tsx      # 任务详情抽屉
│       └── index.ts
├── hooks/
│   ├── useTasks.ts             # 任务数据Hook
│   ├── useAgents.ts            # Agent数据Hook
│   └── useSkills.ts            # Skill数据Hook
├── api/
│   └── client.ts               # API客户端
└── types/
    └── index.ts                # TypeScript类型定义
```

---

## ✅ 确认清单

实施前需要用户确认：

- [ ] 只显示**根任务**（parent_task_id 为 null），还是显示所有运行中任务？
- [ ] 是否需要**自动刷新**功能？刷新间隔多少秒？
- [ ] 页面风格偏好：**深色主题** / **浅色主题** / **跟随系统**？
- [ ] 是否接受上述建议的额外功能？
- [ ] 优先实现哪些功能？

---

## 📝 API 调用示例

### 获取统计数据
```typescript
// 运行中根任务数
const runningRootCount = await fetch('/api/tasks/count?status=running');

// 总任务数
const totalCount = await fetch('/api/tasks/count');

// Agent 列表
const agents = await fetch('/api/agents');

// Skill 列表
const skills = await fetch('/api/skills');
```

### 获取运行中的根任务
```typescript
// 过滤出根任务（parent_task_id 为 null）
const tasks = await fetch('/api/tasks?status=running&limit=20');
const rootTasks = tasks.filter(t => !t.parent_task_id);
```

### 创建新任务
```typescript
await fetch('/api/tasks', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    agent: 'planner',
    prompt: '用户输入的任务描述',
    task_type: 'roottask'
  })
});
```
