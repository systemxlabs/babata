import { useState, useEffect, useCallback } from 'react';
import type { Task, RootTask, TaskFilter } from '../../types';
import { getRootTasks, getTaskTree, deleteTask, controlTask } from '../../api';
import { TaskListHeader } from './components/TaskListHeader';
import { TaskTreeItem } from './components/TaskTreeItem';
import { TaskPagination } from './components/TaskPagination';
import { TaskDetailModal } from '../../components/TaskDetailModal';
import { DeleteConfirmModal } from '../../components/DeleteConfirmModal';
import './Tasks.css';

// 扩展的任务类型，包含展开状态和子任务
interface TaskWithChildren extends RootTask {
  isExpanded?: boolean;
  children?: Task[];
  isLoadingChildren?: boolean;
}

export function Tasks() {
  const [tasks, setTasks] = useState<TaskWithChildren[]>([]);
  const [loading, setLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [taskToDelete, setTaskToDelete] = useState<Task | null>(null);
  
  // 筛选条件
  const [filter, setFilter] = useState<TaskFilter>({
    status: 'running',
    page: 1,
    pageSize: 20,
  });

  // 获取根任务列表
  const fetchTasks = useCallback(async () => {
    setLoading(true);
    try {
      const response = await getRootTasks(filter);
      setTasks(response.tasks.map(t => ({ ...t, isExpanded: false })));
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

  // 处理任务树展开/折叠
  const handleToggleExpand = useCallback(async (taskId: string) => {
    setTasks(prevTasks => {
      const taskIndex = prevTasks.findIndex(t => t.task_id === taskId);
      if (taskIndex === -1) return prevTasks;

      const task = prevTasks[taskIndex];
      const newIsExpanded = !task.isExpanded;

      // 如果展开且没有子任务，先加载子任务
      if (newIsExpanded && !task.children) {
        // 设置加载状态
        const updatedTasks = [...prevTasks];
        updatedTasks[taskIndex] = { ...task, isLoadingChildren: true };

        // 异步加载子任务
        getTaskTree(taskId)
          .then((tree) => {
            setTasks(currentTasks => {
              const idx = currentTasks.findIndex(t => t.task_id === taskId);
              if (idx === -1) return currentTasks;
              const newTasks = [...currentTasks];
              newTasks[idx] = { 
                ...newTasks[idx], 
                isExpanded: true, 
                children: tree.children, 
                isLoadingChildren: false 
              };
              return newTasks;
            });
          })
          .catch(() => {
            setTasks(currentTasks => {
              const idx = currentTasks.findIndex(t => t.task_id === taskId);
              if (idx === -1) return currentTasks;
              const newTasks = [...currentTasks];
              newTasks[idx] = { ...newTasks[idx], isLoadingChildren: false };
              return newTasks;
            });
          });

        return updatedTasks;
      }

      // 直接切换展开状态
      const updatedTasks = [...prevTasks];
      updatedTasks[taskIndex] = { ...task, isExpanded: newIsExpanded };
      return updatedTasks;
    });
  }, []);

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

  // 处理删除点击
  const handleDeleteClick = useCallback((task: Task, e: React.MouseEvent) => {
    e.stopPropagation();
    setTaskToDelete(task);
    setShowDeleteModal(true);
  }, []);

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
            <p className="empty-hint">尝试调整筛选条件</p>
          </div>
        ) : (
          <div className="tasks-list">
            {tasks.map(task => (
              <TaskTreeItem
                key={task.task_id}
                task={task}
                level={0}
                isExpanded={task.isExpanded || false}
                children={task.children}
                isLoading={task.isLoadingChildren || false}
                onToggle={() => handleToggleExpand(task.task_id)}
                onClick={() => handleTaskClick(task.task_id)}
                onDelete={(e: React.MouseEvent) => handleDeleteClick(task, e)}
                onControlTask={handleControlTask}
                formatTime={formatTime}
              />
            ))}
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
