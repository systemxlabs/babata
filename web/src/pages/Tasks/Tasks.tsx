import { useState, useEffect, useCallback } from 'react';
import type { Task, RootTask, TaskFilter } from '../../types';
import { getRootTasks, getTaskTree, deleteTask, controlTask } from '../../api';
import type { TaskTreeResponse } from '../../api';
import { TaskListHeader } from './components/TaskListHeader';
import { TaskTreeItem } from './components/TaskTreeItem';
import { TaskPagination } from './components/TaskPagination';
import { TaskDetailModal } from '../../components/TaskDetailModal';
import { DeleteConfirmModal } from '../../components/DeleteConfirmModal';
import './Tasks.css';

export function Tasks() {
  const [tasks, setTasks] = useState<RootTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [treeLoading, setTreeLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [selectedRootTaskId, setSelectedRootTaskId] = useState<string | null>(null);
  const [selectedTree, setSelectedTree] = useState<TaskTreeResponse | null>(null);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [taskToDelete, setTaskToDelete] = useState<Task | null>(null);
  
  // 筛选条件
  const [filter, setFilter] = useState<TaskFilter>({
    status: 'running',
    page: 1,
    pageSize: 20,
  });

  const hasStatusFilter = filter.status !== undefined && filter.status !== 'all';

  const sortTaskTreeByCreatedAt = useCallback((tree: TaskTreeResponse): TaskTreeResponse => {
    return {
      ...tree,
      children: [...tree.children]
        .sort((a, b) => a.created_at - b.created_at)
        .map(sortTaskTreeByCreatedAt),
    };
  }, []);

  // 获取根任务列表
  const fetchTasks = useCallback(async () => {
    setLoading(true);
    try {
      const response = await getRootTasks(filter);
      setTasks(response.tasks);
      setTotal(response.total);
    } catch (error) {
      console.error('Failed to fetch tasks:', error);
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => {
    fetchTasks();
  }, [fetchTasks]);

  useEffect(() => {
    if (tasks.length === 0) {
      setSelectedRootTaskId(null);
      setSelectedTree(null);
      return;
    }

    if (!selectedRootTaskId || !tasks.some((task) => task.task_id === selectedRootTaskId)) {
      setSelectedRootTaskId(tasks[0].task_id);
    }
  }, [selectedRootTaskId, tasks]);

  useEffect(() => {
    if (!selectedRootTaskId) {
      setSelectedTree(null);
      return;
    }

    let cancelled = false;

    const fetchTree = async () => {
      setTreeLoading(true);
      try {
        const tree = await getTaskTree(selectedRootTaskId);
        if (!cancelled) {
          setSelectedTree(sortTaskTreeByCreatedAt(tree));
        }
      } catch (error) {
        console.error('Failed to fetch task tree:', error);
        if (!cancelled) {
          setSelectedTree(null);
        }
      } finally {
        if (!cancelled) {
          setTreeLoading(false);
        }
      }
    };

    void fetchTree();

    return () => {
      cancelled = true;
    };
  }, [selectedRootTaskId, sortTaskTreeByCreatedAt]);

  // 处理筛选变化
  const handleFilterChange = useCallback((newFilter: Partial<TaskFilter>) => {
    setFilter(prev => ({
      ...prev,
      ...newFilter,
      page: 1, // 重置到第一页
    }));
  }, []);

  // 处理页码变化
  const handlePageChange = useCallback((page: number) => {
    setFilter(prev => ({ ...prev, page }));
  }, []);

  // 处理任务点击
  const handleTaskClick = useCallback((taskId: string) => {
    setSelectedTaskId(taskId);
  }, []);

  const selectedRootTask = tasks.find((task) => task.task_id === selectedRootTaskId) ?? null;

  const handleRootTaskSelect = useCallback((taskId: string) => {
    setSelectedRootTaskId(taskId);
  }, []);

  // 处理整棵任务树删除
  const handleDeleteTreeClick = useCallback(() => {
    if (!selectedRootTask) return;
    setTaskToDelete(selectedRootTask);
    setShowDeleteModal(true);
  }, [selectedRootTask]);

  // 确认删除
  const handleDeleteConfirm = useCallback(async () => {
    if (!taskToDelete) return;
    await deleteTask(taskToDelete.task_id);
    await fetchTasks();
    setShowDeleteModal(false);
    setTaskToDelete(null);
  }, [taskToDelete, fetchTasks]);

  // 处理任务控制操作（暂停/恢复/取消）
  const handleControlTask = useCallback(async (taskId: string, action: 'pause' | 'resume' | 'cancel') => {
    try {
      await controlTask(taskId, action);
      await fetchTasks();
    } catch (error) {
      console.error(`Failed to ${action} task:`, error);
    }
  }, [fetchTasks]);

  // 格式化时间
  const formatTime = useCallback((timestamp: string | number) => {
    return new Date(timestamp).toLocaleString('zh-CN');
  }, []);

  return (
    <div className="tasks-page">
      <TaskListHeader
        filter={filter}
        onFilterChange={handleFilterChange}
        loading={loading}
      />

      <div className="tasks-content">
        {loading && tasks.length === 0 ? (
          <div className="tasks-loading">加载中...</div>
        ) : tasks.length === 0 ? (
          <div className="tasks-empty">
            <p>暂无任务</p>
            <p className="empty-hint">
              {hasStatusFilter ? '尝试切换其他状态标签，或再次点击当前标签查看全部根任务' : '当前没有任何根任务'}
            </p>
          </div>
        ) : (
          <div className="tasks-workspace">
            <aside className="root-task-panel">
              <div className="panel-title-row">
                <h2>根任务</h2>
                <span>{tasks.length} 个</span>
              </div>
              <div className="root-task-list">
                {tasks.map((task) => (
                  <button
                    key={task.task_id}
                    type="button"
                    className={`root-task-card ${selectedRootTaskId === task.task_id ? 'active' : ''}`}
                    onClick={() => handleRootTaskSelect(task.task_id)}
                  >
                    <div className="root-task-card-header">
                      <span className="root-task-agent">{task.agent}</span>
                      <span className={`root-task-status root-task-status-${task.status}`}>
                        {task.status}
                      </span>
                    </div>
                    <div className="root-task-card-title">{task.description}</div>
                    <div className="root-task-card-meta">
                      <span>{formatTime(task.created_at)}</span>
                      <span>{task.subtask_count} 个子任务</span>
                    </div>
                  </button>
                ))}
              </div>
            </aside>

            <section className="task-tree-panel">
              <div className="panel-title-row">
                <div>
                  <h2>任务树</h2>
                  <p>
                    {selectedRootTask
                      ? `${selectedRootTask.description}`
                      : '选择左侧根任务查看整棵任务树'}
                  </p>
                </div>
                {selectedRootTask && (
                  <button
                    type="button"
                    className="tree-delete-button"
                    onClick={handleDeleteTreeClick}
                    title="删除整棵任务树"
                  >
                    删除任务树
                  </button>
                )}
              </div>

              {treeLoading ? (
                <div className="tasks-loading tree-loading">加载任务树中...</div>
              ) : selectedTree ? (
                <div className="task-tree-stage">
                  <TaskTreeItem
                    task={selectedTree}
                    onClick={handleTaskClick}
                    onControlTask={handleControlTask}
                    formatTime={formatTime}
                  />
                </div>
              ) : (
                <div className="tasks-empty tree-empty">
                  <p>未加载到任务树</p>
                  <p className="empty-hint">请重新选择根任务或刷新列表</p>
                </div>
              )}
            </section>
          </div>
        )}
      </div>

      {total > filter.pageSize && (
        <TaskPagination
          currentPage={filter.page}
          pageSize={filter.pageSize}
          total={total}
          onPageChange={handlePageChange}
        />
      )}

      {/* 任务详情弹窗 */}
      <TaskDetailModal
        taskId={selectedTaskId || ''}
        isOpen={!!selectedTaskId}
        onClose={() => setSelectedTaskId(null)}
      />

      {/* 删除确认弹窗 */}
      <DeleteConfirmModal
        isOpen={showDeleteModal}
        taskId={taskToDelete?.task_id || ''}
        taskDescription={taskToDelete?.description || ''}
        onConfirm={handleDeleteConfirm}
        onCancel={() => {
          setShowDeleteModal(false);
          setTaskToDelete(null);
        }}
      />
    </div>
  );
}
