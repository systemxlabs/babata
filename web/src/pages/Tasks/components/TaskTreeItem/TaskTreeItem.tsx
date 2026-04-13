import type { Task, RootTask } from '../../../../types';
import { TaskStatusBadge } from '../TaskStatusBadge/TaskStatusBadge';
import './TaskTreeItem.css';

type TreeTask = (RootTask | Task) & {
  children?: TreeTask[];
};

interface TaskTreeItemProps {
  task: TreeTask;
  level: number;
  isExpanded: boolean;
  children?: TreeTask[];
  isLoading?: boolean;
  onToggle: () => void;
  onClick: (taskId: string) => void;
  onDelete: (task: TreeTask, e: React.MouseEvent) => void;
  onControlTask?: (taskId: string, action: 'pause' | 'resume' | 'cancel') => void;
  formatTime: (timestamp: string | number) => string;
}

export function TaskTreeItem({
  task,
  level,
  isExpanded,
  children,
  isLoading,
  onToggle,
  onClick,
  onDelete,
  onControlTask,
  formatTime,
}: TaskTreeItemProps) {
  const isRootTask = !task.parent_task_id;
  const hasChildren = isRootTask && (task as RootTask).subtask_count > 0;
  const actualChildren = children || [];

  // 截断描述文本
  const truncateDescription = (desc: string, maxLength: number = 60) => {
    if (desc.length <= maxLength) return desc;
    return desc.substring(0, maxLength) + '...';
  };

  return (
    <>
      <div
        className={`task-tree-item ${isRootTask ? 'root-task' : 'sub-task'} ${isExpanded ? 'expanded' : ''}`}
        style={{ 
          paddingLeft: isRootTask ? '16px' : `${16 + level * 24}px`,
        }}
        onClick={() => onClick(task.task_id)}
      >
        <div className="task-item-content">
          {/* 展开/折叠按钮 */}
          <div className="task-expander">
            {hasChildren ? (
              <button
                className={`expand-btn ${isExpanded ? 'expanded' : ''}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onToggle();
                }}
                disabled={isLoading}
              >
                {isLoading ? (
                  <svg className="loading-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                  </svg>
                ) : (
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d={isExpanded ? "m18 15-6-6-6 6" : "m9 18 6-6-6-6"} />
                  </svg>
                )}
              </button>
            ) : isRootTask ? (
              <span className="expand-placeholder"></span>
            ) : null}
          </div>

          {/* 任务图标 */}
          <div className="task-icon">
            {isRootTask ? (
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
              </svg>
            ) : (
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10" />
              </svg>
            )}
          </div>

          {/* 任务描述 */}
          <div className="task-info">
            <div className="task-description" title={task.description}>
              {truncateDescription(task.description)}
            </div>
            <div className="task-meta">
              <span className="task-agent">{task.agent}</span>
              <span className="task-time">{formatTime(task.created_at)}</span>
              {isRootTask && (task as RootTask).subtask_count > 0 && (
                <span className="task-children-count">
                  {(task as RootTask).subtask_count} 个子任务
                </span>
              )}
            </div>
          </div>

          {/* 状态标签 */}
          <div className="task-status">
            <TaskStatusBadge status={task.status} showLabel size="md" />
          </div>

          {/* 操作按钮 */}
          <div className="task-actions" onClick={(e) => e.stopPropagation()}>
            {/* 暂停/恢复/取消控制按钮 */}
            {(task.status === 'running' || task.status === 'paused') && onControlTask && (
              <>
                {task.status === 'running' && (
                  <button
                    className="action-btn pause-btn"
                    onClick={() => onControlTask(task.task_id, 'pause')}
                    title="暂停任务"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <rect x="6" y="4" width="4" height="16" rx="1" />
                      <rect x="14" y="4" width="4" height="16" rx="1" />
                    </svg>
                  </button>
                )}
                {task.status === 'paused' && (
                  <button
                    className="action-btn resume-btn"
                    onClick={() => onControlTask(task.task_id, 'resume')}
                    title="恢复任务"
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <polygon points="5 3 19 12 5 21 5 3" />
                    </svg>
                  </button>
                )}
                <button
                  className="action-btn cancel-btn"
                  onClick={() => onControlTask(task.task_id, 'cancel')}
                  title="取消任务"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="12" cy="12" r="10" />
                    <path d="m15 9-6 6" />
                    <path d="m9 9 6 6" />
                  </svg>
                </button>
              </>
            )}
            <button
              className="action-btn delete-btn"
              onClick={(e) => onDelete(task, e)}
              title="删除任务"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M3 6h18" />
                <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      {/* 子任务列表 */}
      {isExpanded && actualChildren.length > 0 && (
        <div className="subtask-list">
          {actualChildren.map(child => (
            <TaskTreeItem
              key={child.task_id}
              task={child}
              level={level + 1}
              isExpanded={Boolean(child.children?.length)}
              children={child.children}
              onToggle={() => {}}
              onClick={onClick}
              onDelete={onDelete}
              onControlTask={onControlTask}
              formatTime={formatTime}
            />
          ))}
        </div>
      )}
    </>
  );
}
