## 任务管理页面 UI 实现

### 🎨 功能特性

#### 1. 页面导航
- 使用 React Router 实现多页面导航
- 新增侧边栏导航菜单
- 支持概览、任务、智能体、频道四个页面

#### 2. 任务列表页面 (`/tasks`)
- **根任务列表展示**
  - 分页显示所有根任务
  - 每行显示：任务描述、Agent、状态、创建时间、子任务数
  
- **筛选和搜索**
  - 按状态筛选（全部/运行中/已完成/失败/已暂停/已取消）
  - 按 Agent 筛选
  - 按描述关键词搜索

- **任务树展示**
  - 可展开/折叠的树形结构
  - 显示任务层级关系
  - 懒加载子任务

- **任务操作**
  - 删除任务（带二次确认弹窗）
  - 点击任务查看详情

#### 3. 任务详情弹窗
- **任务信息面板**
  - 显示任务完整信息（ID、描述、Agent、状态、创建时间等）
  - 支持任务控制操作（暂停/恢复/取消）

- **任务目录标签**
  - 树形文件目录展示
  - 支持查看文件内容
  - 代码高亮显示

- **任务日志标签**
  - 实时显示任务执行日志
  - 支持自动刷新

### 📁 项目结构

```
web/src/
├── pages/
│   └── Tasks/
│       ├── Tasks.tsx              # 任务管理主页面
│       ├── Tasks.css
│       └── components/
│           ├── TaskListHeader/    # 筛选和搜索头部
│           ├── TaskPagination/    # 分页组件
│           ├── TaskStatusBadge/   # 状态标签
│           └── TaskTreeItem/      # 任务树节点
├── components/
│   ├── TaskDetailModal/           # 任务详情弹窗
│   │   ├── TaskDetailModal.tsx
│   │   └── components/
│   │       ├── TaskDirectoryTab/  # 任务目录标签
│   │       └── TaskLogsTab/       # 任务日志标签
│   ├── TaskTreeList.tsx           # 任务树列表
│   ├── TaskFiles.tsx              # 文件浏览器
│   ├── TaskLogs.tsx               # 任务日志
│   ├── TaskFilter.tsx             # 任务筛选器
│   ├── DeleteConfirmModal.tsx     # 删除确认弹窗
│   └── index.ts
├── api.ts                         # API 客户端
├── types.ts                       # TypeScript 类型定义
└── App.tsx                        # 应用入口（添加路由）
```

### 🔌 依赖的后端 API

此 PR 依赖后端 API 实现（见 #99）：
- `GET /api/tasks/roots` - 获取根任务列表
- `GET /api/tasks/:id/children` - 获取子任务
- `DELETE /api/tasks/:id` - 删除任务
- `POST /api/tasks/:id/control` - 控制任务
- `GET /api/tasks/:id/files` - 获取文件列表
- `GET /api/tasks/:id/files/content` - 获取文件内容
- `GET /api/tasks/:id/logs` - 获取任务日志

### 📝 技术说明

- React + TypeScript + Vite
- React Router v7 用于路由
- 自定义 hooks 用于数据获取
- CSS 模块化管理
- 深色主题设计

### 🔗 相关 PR

后端 API 实现：#99
