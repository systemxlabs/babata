import type { TaskStatus } from '../api/types';

interface StatusBadgeProps {
  compact?: boolean;
  status: TaskStatus;
}

export function StatusBadge({ compact = false, status }: StatusBadgeProps) {
  const badgeClassName = compact
    ? `status-badge status-badge--compact status-badge--${status}`
    : `status-badge status-badge--${status}`;

  return (
    <span className={badgeClassName}>
      <span className="status-badge__dot" aria-hidden="true" />
      {status}
    </span>
  );
}
