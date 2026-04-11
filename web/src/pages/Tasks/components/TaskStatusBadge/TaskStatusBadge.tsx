import type { TaskStatus } from '../../../../types';
import { STATUS_LABELS, STATUS_BG_COLORS, STATUS_TEXT_COLORS } from '../../../../types';
import './TaskStatusBadge.css';

interface TaskStatusBadgeProps {
  status: TaskStatus;
  showLabel?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

export function TaskStatusBadge({ status, showLabel = true, size = 'md' }: TaskStatusBadgeProps) {
  const bgColorClass = STATUS_BG_COLORS[status];
  const textColorClass = STATUS_TEXT_COLORS[status];
  const label = STATUS_LABELS[status];

  // 根据状态获取图标
  const getStatusIcon = () => {
    switch (status) {
      case 'running':
        return (
          <svg className="status-icon spinning" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
          </svg>
        );
      case 'completed':
        return (
          <svg className="status-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <path d="m9 11 3 3L22 4" />
          </svg>
        );
      case 'failed':
        return (
          <svg className="status-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <path d="m15 9-6 6" />
            <path d="m9 9 6 6" />
          </svg>
        );
      case 'paused':
        return (
          <svg className="status-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M10 15V9" />
            <path d="M14 15V9" />
          </svg>
        );
      case 'canceled':
        return (
          <svg className="status-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <path d="m8 12 8 0" />
          </svg>
        );
      default:
        return null;
    }
  };

  return (
    <span className={`task-status-badge ${bgColorClass} ${textColorClass} size-${size}`}>
      {getStatusIcon()}
      {showLabel && <span className="status-label">{label}</span>}
    </span>
  );
}
