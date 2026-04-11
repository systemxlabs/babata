import { useState, useCallback } from 'react';
import type { Task } from '../types';
import { STATUS_COLORS, STATUS_LABELS } from '../types';
import './TaskTreeItem.css';

interface TaskTreeItemProps {
  task: Task;
  level: number;
  childTasks: Task[];
  allTasks: Task[];
  selectedTaskId: string | null;
  onSelect: (taskId: string) => void;
  onDelete: (taskId: string) => void;
}

// Format relative time (e.g., "5 minutes ago")
function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (diffInSeconds < 60) {
    return '刚刚';
  }

  const diffInMinutes = Math.floor(diffInSeconds / 60);
  if (diffInMinutes < 60) {
    return `${diffInMinutes}分钟前`;
  }

  const diffInHours = Math.floor(diffInMinutes / 60);
  if (diffInHours < 24) {
    return `${diffInHours}小时前`;
  }

  const diffInDays = Math.floor(diffInHours / 24);
  if (diffInDays < 30) {
    return `${diffInDays}天前`;
  }

  const diffInMonths = Math.floor(diffInDays / 30);
  if (diffInMonths < 12) {
    return `${diffInMonths}个月前`;
  }

  const diffInYears = Math.floor(diffInMonths / 12);
  return `${diffInYears}年前`;
}

export function TaskTreeItem({
  task,
  level,
  childTasks,
  allTasks,
  selectedTaskId,
  onSelect,
  onDelete,
}: TaskTreeItemProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const hasChildren = childTasks.length > 0;
  const isSelected = selectedTaskId === task.task_id;

  const handleToggle = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setIsExpanded((prev) => !prev);
  }, []);

  const handleSelect = useCallback(() => {
    onSelect(task.task_id);
  }, [onSelect, task.task_id]);

  const handleDeleteClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setShowDeleteConfirm(true);
  }, []);

  const handleConfirmDelete = useCallback(() => {
    onDelete(task.task_id);
    setShowDeleteConfirm(false);
  }, [onDelete, task.task_id]);

  const handleCancelDelete = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setShowDeleteConfirm(false);
  }, []);

  // Get child tasks recursively for this task
  const getChildTasksForTask = useCallback((parentId: string): Task[] => {
    return allTasks.filter((t) => t.parent_task_id === parentId);
  }, [allTasks]);

  const statusColor = STATUS_COLORS[task.status];
  const statusLabel = STATUS_LABELS[task.status];

  return (
    <div className="task-tree-item">
      <div
        className={`task-tree-item__content ${isSelected ? 'task-tree-item__content--selected' : ''}`}
        style={{ paddingLeft: `${level * 24}px` }}
        onClick={handleSelect}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === 'Enter' && handleSelect()}
      >
        {/* Expand/Collapse Icon */}
        <span className="task-tree-item__expand-icon">
          {hasChildren ? (
            <button
              className="task-tree-item__toggle-btn"
              onClick={handleToggle}
              title={isExpanded ? '折叠' : '展开'}
            >
              {isExpanded ? '▼' : '▶'}
            </button>
          ) : (
            <span className="task-tree-item__spacer" />
          )}
        </span>

        {/* Task Info */}
        <div className="task-tree-item__info">
          <span className="task-tree-item__description" title={task.description}>
            {task.description.length > 50
              ? `${task.description.slice(0, 50)}...`
              : task.description}
          </span>
        </div>

        {/* Agent Tag */}
        <span className="task-tree-item__agent">{task.agent}</span>

        {/* Status Tag */}
        <span
          className="task-tree-item__status"
          style={{
            backgroundColor: `${statusColor}20`,
            color: statusColor,
            borderColor: `${statusColor}40`,
          }}
        >
          {statusLabel}
        </span>

        {/* Created Time */}
        <span className="task-tree-item__time">
          {formatRelativeTime(task.created_at)}
        </span>

        {/* Child Count */}
        {hasChildren && (
          <span className="task-tree-item__child-count">
            {childTasks.length}个子任务
          </span>
        )}

        {/* Never Ends Indicator */}
        {task.never_ends && (
          <span className="task-tree-item__never-ends" title="常驻任务">
            ♾️
          </span>
        )}

        {/* Delete Button */}
        <div className="task-tree-item__actions">
          {showDeleteConfirm ? (
            <div className="task-tree-item__confirm">
              <span className="task-tree-item__confirm-text">确认删除?</span>
              <button
                className="task-tree-item__confirm-btn task-tree-item__confirm-btn--yes"
                onClick={handleConfirmDelete}
              >
                是
              </button>
              <button
                className="task-tree-item__confirm-btn task-tree-item__confirm-btn--no"
                onClick={handleCancelDelete}
              >
                否
              </button>
            </div>
          ) : (
            <button
              className="task-tree-item__delete-btn"
              onClick={handleDeleteClick}
              title="删除任务"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Render child tasks if expanded */}
      {isExpanded && hasChildren && (
        <div className="task-tree-item__children">
          {childTasks.map((childTask) => (
            <TaskTreeItem
              key={childTask.task_id}
              task={childTask}
              level={level + 1}
              childTasks={getChildTasksForTask(childTask.task_id)}
              allTasks={allTasks}
              selectedTaskId={selectedTaskId}
              onSelect={onSelect}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
}
