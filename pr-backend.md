## 任务管理 API 后端实现

### 🔌 新增 API 接口

| 接口 | 方法 | 描述 | 参数 |
|------|------|------|------|
| `/api/tasks/roots` | GET | 获取根任务列表 | `page`, `page_size`, `status`, `agent`, `search` |
| `/api/tasks/:id/children` | GET | 获取任务的子任务列表 | `task_id` (路径参数) |
| `/api/tasks/:id` | DELETE | 删除任务（及其所有子任务）| `task_id` (路径参数) |
| `/api/tasks/:id/control` | POST | 控制任务状态 | `task_id`, `action` (pause/resume/cancel) |
| `/api/tasks/:id/files` | GET | 获取任务目录文件列表 | `task_id`, `path` (可选) |
| `/api/tasks/:id/files/content` | GET | 获取任务文件内容 | `task_id`, `path` |
| `/api/tasks/:id/logs` | GET | 获取任务日志 | `task_id` |

### 📁 改动文件

```
src/
├── http/
│   ├── get_task_children.rs    # 新增：获取子任务接口
│   ├── list_root_tasks.rs      # 新增：获取根任务列表接口
│   └── mod.rs                  # 修改：注册新路由
├── task/
│   ├── manager.rs              # 修改：添加任务操作方法
│   └── store.rs                # 修改：添加任务查询方法
```

### 📝 主要功能

1. **根任务列表查询**
   - 支持分页（page, page_size）
   - 支持按状态筛选（running/completed/failed/paused/canceled）
   - 支持按 Agent 名称筛选
   - 支持按任务描述关键词搜索

2. **任务树查询**
   - 支持获取指定任务的直接子任务
   - 支持递归获取所有后代任务

3. **任务删除**
   - 级联删除任务及其所有子任务
   - 先取消正在运行的任务

4. **任务控制**
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

### 🔗 相关 PR

前端页面实现：#98
