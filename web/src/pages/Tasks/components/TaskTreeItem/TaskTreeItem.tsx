import type { Task } from '../../../../types';
import { TaskStatusBadge } from '../TaskStatusBadge/TaskStatusBadge';
import './TaskTreeItem.css';

type TreeTask = Task & {
  children?: TreeTask[];
};

interface TaskTreeItemProps {
  task: TreeTask;
  onClick: (taskId: string) => void;
  onControlTask?: (taskId: string, action: 'pause' | 'resume' | 'cancel') => void;
  formatTime: (timestamp: string | number) => string;
}

export function TaskTreeItem({
  task,
  onClick,
  onControlTask,
  formatTime,
}: TaskTreeItemProps) {
  const isRootTask = !task.parent_task_id;
  const actualChildren = task.children || [];
  const hasVisibleChildren = actualChildren.length > 0;

  // 截断描述文本
  const truncateDescription = (desc: string, maxLength: number = 60) => {
    if (desc.length <= maxLength) return desc;
    return desc.substring(0, maxLength) + '...';
  };

  return (
    <div className="task-tree-branch">
      <div className="task-tree-node-row">
        <div className={`task-tree-card-wrap ${hasVisibleChildren ? 'has-children' : ''}`}>
          <div
            className={`task-tree-item ${isRootTask ? 'root-task' : 'sub-task'}`}
            onClick={() => onClick(task.task_id)}
          >
            <div className="task-card-header">
              <div className="task-card-title-group">
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
                <div className="task-card-heading">
                  <div className="task-card-kind">{isRootTask ? 'Root Task' : 'Subtask'}</div>
                  <div className="task-description" title={task.description}>
                    {truncateDescription(task.description, isRootTask ? 72 : 56)}
                  </div>
                </div>
              </div>

              <div className="task-status">
                <TaskStatusBadge status={task.status} showLabel size="md" />
              </div>
            </div>

            <div className="task-info">
              <div className="task-meta">
                <span className="task-agent">{task.agent}</span>
                <span className="task-time">{formatTime(task.created_at)}</span>
              </div>
              {actualChildren.length > 0 && (
                <div className="task-children-count">
                  {actualChildren.length} 个子任务
                </div>
              )}
            </div>

            <div className="task-actions" onClick={(e) => e.stopPropagation()}>
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
            </div>
          </div>
        </div>

        {hasVisibleChildren && (
          <div className="task-tree-children">
            {actualChildren.map((child) => (
              <div key={child.task_id} className="task-tree-child">
                <TaskTreeItem
                  task={child}
                  onClick={onClick}
                  onControlTask={onControlTask}
                  formatTime={formatTime}
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
