# 修复任务管理页面计划

## 目标
更新主分支，创建新分支，修复任务管理页面，创建 PR

## 执行步骤

### 步骤 1: 更新主分支
- **操作**: 切换到 main 分支，拉取最新代码
- **验收标准**: 本地 main 分支已是最新

### 步骤 2: 创建新分支
- **操作**: 基于 main 创建 feature/fix-task-manager 分支
- **验收标准**: 新分支已创建并切换

### 步骤 3: 修改任务管理页面
- **操作**: 修改 `web/src/pages/Tasks/Tasks.tsx`:
  1. 修改默认 filter，status 从 'all' 改为 'running'
  2. 移除 agent 相关代码（useEffect 中的 getAgentsList 调用）
  3. 移除 filter 中的 agent 和 search 字段
- **操作**: 修改 `web/src/pages/Tasks/components/TaskListHeader/TaskListHeader.tsx`:
  1. 移除 agent 下拉选择框
  2. 移除搜索框
- **验收标准**: 代码修改完成，页面默认显示运行中任务，无 agent 和搜索过滤

### 步骤 4: 验证修改
- **操作**: 运行前端构建，确保无错误
- **验收标准**: 构建成功

### 步骤 5: 提交并推送
- **操作**: 提交修改，推送到远程
- **验收标准**: 代码已推送到远程分支

### 步骤 6: 创建 PR
- **操作**: 使用 gh CLI 创建 PR
- **验收标准**: PR 创建成功，CI 通过

### 步骤 7: 通知用户
- **操作**: 微信通知用户完成
- **验收标准**: 用户收到通知
