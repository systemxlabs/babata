import { useState, useCallback, useMemo } from 'react';
import type { Task, TaskStatus } from '../types';
import { TaskTreeItem } from './TaskTreeItem';
import { TaskFilter } from './TaskFilter';
import { getTasks, deleteTask } from '../api';
import './TaskTreeList.css';

interface TaskTreeListProps {
  initialTasks?: Task[];
}

export function TaskTreeList({ initialTasks = [] }: TaskTreeListProps) {
  const [tasks, setTasks] = useState<Task[]>(initialTasks);
  const [selectedStatus, setSelectedStatus] = useState<TaskStatus | 'all'>('all');
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Get root tasks (tasks without parent)
  const rootTasks = useMemo(() => {
    return tasks.filter((task) => task.parent_task_id === null);
  }, [tasks]);

  // Get child tasks for a parent task
  const getChildTasks = useCallback(
    (parentId: string): Task[] => {
      return tasks.filter((task) => task.parent_task_id === parentId);
    },
    [tasks]
  );

  // Fetch tasks from API
  const fetchTasks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getTasks({
        status: selectedStatus === 'all' ? undefined : selectedStatus,
      });
      setTasks(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取任务列表失败');
    } finally {
      setLoading(false);
    }
  }, [selectedStatus]);

  // Handle status filter change
  const handleStatusChange = useCallback(
    (status: TaskStatus | 'all') => {
      setSelectedStatus(status);
      // Trigger refresh when filter changes
      setTimeout(() => fetchTasks(), 0);
    },
    [fetchTasks]
  );

  // Handle task selection
  const handleSelect = useCallback((taskId: string) => {
    setSelectedTaskId(taskId);
  }, []);

  // Handle task deletion
  const handleDelete = useCallback(
    async (taskId: string) => {
      try {
        await deleteTask(taskId);
        // Remove task and its children from local state
        setTasks((prevTasks) => {
          const taskIdsToRemove = new Set<string>();

          // Helper to collect all child task IDs recursively
          const collectChildIds = (parentId: string) => {
            taskIdsToRemove.add(parentId);
            const children = prevTasks.filter((t) => t.parent_task_id === parentId);
            children.forEach((child) => collectChildIds(child.task_id));
          };

          collectChildIds(taskId);
          return prevTasks.filter((t) => !taskIdsToRemove.has(t.task_id));
        });

        // Clear selection if the deleted task was selected
        setSelectedTaskId((prev) => (prev === taskId ? null : prev));
      } catch (err) {
        setError(err instanceof Error ? err.message : '删除任务失败');
      }
    },
    []
  );

  return (
    <div className="task-tree-list">
      <TaskFilter
        selectedStatus={selectedStatus}
        onStatusChange={handleStatusChange}
        onRefresh={fetchTasks}
        loading={loading}
      />

      {error && (
        <div className="task-tree-list__error">
          <span className="task-tree-list__error-icon">⚠️</span>
          {error}
          <button
            className="task-tree-list__error-close"
            onClick={() => setError(null)}
          >
            ✕
          </button>
        </div>
      )}

      <div className="task-tree-list__content">
        {loading && tasks.length === 0 ? (
          <div className="task-tree-list__loading">
            <div className="task-tree-list__spinner" />
            <span>加载中...</span>
          </div>
        ) : rootTasks.length === 0 ? (
          <div className="task-tree-list__empty">
            <div className="task-tree-list__empty-icon">📋</div>
            <p>暂无任务</p>
            <button
              className="task-tree-list__refresh-link"
              onClick={fetchTasks}
              disabled={loading}
            >
              点击刷新
            </button>
          </div>
        ) : (
          <div className="task-tree-list__items">
            {rootTasks.map((task) => (
              <TaskTreeItem
                key={task.task_id}
                task={task}
                level={0}
                childTasks={getChildTasks(task.task_id)}
                allTasks={tasks}
                selectedTaskId={selectedTaskId}
                onSelect={handleSelect}
                onDelete={handleDelete}
              />
            ))}
          </div>
        )}
      </div>

      {tasks.length > 0 && (
        <div className="task-tree-list__footer">
          <span className="task-tree-list__stats">
            共 {tasks.length} 个任务
            {selectedStatus !== 'all' && ` (${selectedStatus})`}
          </span>
        </div>
      )}
    </div>
  );
}
