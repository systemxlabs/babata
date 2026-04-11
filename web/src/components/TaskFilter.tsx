import { STATUS_LABELS, type TaskStatus } from '../types';
import './TaskFilter.css';

interface TaskFilterProps {
  selectedStatus: TaskStatus | 'all';
  onStatusChange: (status: TaskStatus | 'all') => void;
  onRefresh: () => void;
  loading?: boolean;
}

export function TaskFilter({
  selectedStatus,
  onStatusChange,
  onRefresh,
  loading = false,
}: TaskFilterProps) {
  const statusOptions: Array<TaskStatus | 'all'> = [
    'all',
    'running',
    'completed',
    'failed',
    'paused',
    'canceled',
  ];

  return (
    <div className="task-filter">
      <div className="task-filter__group">
        <label htmlFor="status-filter" className="task-filter__label">
          状态筛选
        </label>
        <select
          id="status-filter"
          className="task-filter__select"
          value={selectedStatus}
          onChange={(e) => onStatusChange(e.target.value as TaskStatus | 'all')}
          disabled={loading}
        >
          {statusOptions.map((status) => (
            <option key={status} value={status}>
              {STATUS_LABELS[status]}
            </option>
          ))}
        </select>
      </div>
      
      <button
        className="task-filter__refresh-btn"
        onClick={onRefresh}
        disabled={loading}
        title="刷新列表"
      >
        <svg
          className={`task-filter__refresh-icon ${loading ? 'task-filter__refresh-icon--spinning' : ''}`}
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="23 4 23 10 17 10" />
          <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
        </svg>
        刷新
      </button>
    </div>
  );
}
