## 任务管理 API 后端实现

### 🔌 API 接口

| 接口 | 方法 | 描述 | 参数 |
|------|------|------|------|
| `/api/tasks` | GET | 分页获取根任务列表 | `page`, `page_size`, `status`, `agent`, `search` |
| `/api/tasks` | POST | 创建新任务 | `description`, `prompt`, `agent`, etc. |
| `/api/tasks/:id` | GET | 获取任务详情 | `task_id` (路径参数) |
| `/api/tasks/:id` | DELETE | 删除任务（及其所有子任务）| `task_id` (路径参数) |
| `/api/tasks/:id/tree` | GET | 获取完整任务树 | `task_id` (路径参数) |
| `/api/tasks/:id/control` | POST | 控制任务状态 | `task_id`, `action` (pause/resume/cancel) |
| `/api/tasks/:id/files` | GET | 获取任务目录文件列表 | `task_id`, `path` (可选) |
| `/api/tasks/:id/files/{*path}` | GET | 获取任务文件内容 | `task_id`, `path` |
| `/api/tasks/:id/logs` | GET | 获取任务日志 | `task_id` |
| `/api/tasks/count` | GET | 获取任务数量统计 | `status` (可选) |

### 📁 改动文件

```
src/
├── http/
│   ├── get_task_tree.rs        # 新增：获取完整任务树接口
│   ├── list_root_tasks.rs      # 新增：分页获取根任务列表
│   └── mod.rs                  # 修改：注册新路由，删除 list_tasks
├── task/
│   ├── manager.rs              # 修改：添加任务操作方法
│   └── store.rs                # 修改：添加任务查询方法
└── cli/task.rs                 # 修改：CLI 使用新的 list_root_tasks API
```

### 📝 主要功能

1. **根任务列表查询** (`GET /api/tasks`)
   - 只返回根任务（parent_task_id IS NULL）
   - 支持分页（page, page_size）
   - 支持按状态筛选（running/completed/failed/paused/canceled）
   - 支持按 Agent 名称筛选
   - 支持按任务描述关键词搜索
   - 返回子任务数量

2. **完整任务树查询** (`GET /api/tasks/:id/tree`)
   - 递归获取指定任务及其所有后代子任务
   - 返回树形结构：包含嵌套的 children 数组

3. **任务删除** (`DELETE /api/tasks/:id`)
   - 级联删除任务及其所有子任务
   - 先取消正在运行的任务

4. **任务控制** (`POST /api/tasks/:id/control`)
   - 暂停（pause）
   - 恢复（resume）
   - 取消（cancel）

5. **任务文件管理**
   - 浏览任务目录文件树
   - 读取任务文件内容
   - 支持文本文件预览

6. **任务日志**
   - 获取任务执行日志
   - 支持增量读取

### 🗑️ 删除的代码

- ❌ `src/http/list_tasks.rs` - 已删除未使用的模块
- ❌ `GET /api/tasks/roots` - 已合并到 `GET /api/tasks`
- ❌ `GET /api/tasks/:id/children` - 已改为 `GET /api/tasks/:id/tree`

### 🔗 相关 PR

前端页面实现：#98
