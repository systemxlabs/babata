interface ToolbarProps {
  autoRefresh: boolean;
  isRefreshing: boolean;
  lastRefreshedAt: number | null;
  onAutoRefreshChange: (value: boolean) => void;
  onRefresh: () => void;
}

export function Toolbar({
  autoRefresh,
  isRefreshing,
  lastRefreshedAt,
  onAutoRefreshChange,
  onRefresh,
}: ToolbarProps) {
  const refreshLabel = isRefreshing ? 'Refreshing...' : 'Refresh';
  const lastSeenLabel = lastRefreshedAt
    ? new Intl.DateTimeFormat(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      }).format(lastRefreshedAt)
    : 'Waiting for first refresh';

  return (
    <div className="toolbar">
      <div className="toolbar__meta">
        <span className="toolbar__kicker">Polling every 5s</span>
        <span className="toolbar__timestamp">Last sync {lastSeenLabel}</span>
      </div>
      <div className="toolbar__actions">
        <button className="toolbar__button" type="button" onClick={onRefresh}>
          {refreshLabel}
        </button>
        <label className="toolbar__toggle">
          <input
            checked={autoRefresh}
            onChange={(event) => onAutoRefreshChange(event.target.checked)}
            type="checkbox"
          />
          <span>Auto refresh</span>
        </label>
      </div>
    </div>
  );
}
