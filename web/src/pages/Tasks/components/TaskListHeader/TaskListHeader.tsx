import { useCallback } from 'react';
import type { TaskFilter, TaskStatus } from '../../../../types';
import { STATUS_LABELS } from '../../../../types';
import './TaskListHeader.css';

interface TaskListHeaderProps {
  filter: TaskFilter;
  onFilterChange: (filter: Partial<TaskFilter>) => void;
  loading?: boolean;
}

const STATUS_OPTIONS: (TaskStatus | 'all')[] = ['all', 'running', 'completed', 'failed', 'paused', 'canceled'];

export function TaskListHeader({ filter, onFilterChange, loading }: TaskListHeaderProps) {
  const handleStatusToggle = useCallback((status: TaskStatus) => {
    const nextStatus = filter.status === status ? 'all' : status;
    onFilterChange({ status: nextStatus });
  }, [filter.status, onFilterChange]);

  const activeStatus = filter.status && filter.status !== 'all' ? filter.status : null;

  return (
    <div className="task-list-header">
      <div className="filter-row">
        <div className="status-tabs" aria-label="任务状态筛选">
          {(STATUS_OPTIONS.filter((status) => status !== 'all') as TaskStatus[]).map((status) => (
            <button
              key={status}
              type="button"
              className={`status-tab ${activeStatus === status ? 'active' : ''}`}
              onClick={() => handleStatusToggle(status)}
              disabled={loading}
              aria-pressed={activeStatus === status}
            >
              {STATUS_LABELS[status]}
            </button>
          ))}
        </div>
        <div className="status-filter-summary">
          {activeStatus ? `当前筛选：${STATUS_LABELS[activeStatus]}` : '当前筛选：全部根任务'}
        </div>
      </div>
    </div>
  );
}
