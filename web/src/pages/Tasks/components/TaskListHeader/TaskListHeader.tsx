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
  const handleStatusChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    onFilterChange({ status: e.target.value as TaskStatus | 'all' });
  }, [onFilterChange]);

  return (
    <div className="task-list-header">
      <div className="filter-row">
        <div className="filter-group">
          <label htmlFor="status-filter">状态:</label>
          <select
            id="status-filter"
            value={filter.status || 'all'}
            onChange={handleStatusChange}
            disabled={loading}
            className="filter-select"
          >
            {STATUS_OPTIONS.map(status => (
              <option key={status} value={status}>
                {STATUS_LABELS[status]}
              </option>
            ))}
          </select>
        </div>


      </div>
    </div>
  );
}
