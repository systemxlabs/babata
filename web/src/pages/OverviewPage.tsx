import { Link } from 'react-router-dom';

import { type OverviewResponse } from '../api/types';
import { apiGet } from '../api/client';
import { Panel } from '../components/Panel';
import { StatusBadge } from '../components/StatusBadge';
import { usePolling } from '../hooks/usePolling';

export function OverviewPage() {
  const overview = usePolling(() => apiGet<OverviewResponse>('/overview'), {
    intervalMs: 5000,
  });

  if (overview.error) {
    return (
      <Panel eyebrow="Overview" title="Runtime sightline" titleLevel={1}>
        <p className="panel__lede">Failed to load overview data: {overview.error.message}</p>
      </Panel>
    );
  }

  if (!overview.data) {
    return (
      <Panel eyebrow="Overview" title="Runtime sightline" titleLevel={1}>
        <p className="panel__lede">Loading runtime counters...</p>
      </Panel>
    );
  }

  const { recent_tasks: recentTasks, status_counts: statusCounts } = overview.data;
  const activeTasks = recentTasks.filter((task) => task.status === 'running' || task.status === 'paused');

  return (
    <div className="page-stack">
      <Panel
        className="panel--hero"
        eyebrow="Overview"
        meta={<span className="panel__tag">Live polling</span>}
        title="Runtime sightline"
        titleLevel={1}
      >
        <p className="panel__lede">
          The control plane now shows the current pulse of the local runtime, with active
          attention first and recent task movement underneath.
        </p>
        <div className="metric-grid">
          <MetricTile label="Running" value={statusCounts.running} />
          <MetricTile label="Paused" value={statusCounts.paused} />
          <MetricTile label="Done" value={statusCounts.done} />
          <MetricTile label="Canceled" value={statusCounts.canceled} />
          <MetricTile label="Total" value={statusCounts.total} />
        </div>
      </Panel>

      <div className="page-grid page-grid--two-up">
        <Panel eyebrow="Attention" title="Active tasks">
          {activeTasks.length === 0 ? (
            <p className="empty-state">No running or paused tasks need attention right now.</p>
          ) : (
            <TaskList tasks={activeTasks} />
          )}
        </Panel>

        <Panel eyebrow="Recent" title="Recent tasks">
          {recentTasks.length === 0 ? (
            <p className="empty-state">No tasks have been created yet.</p>
          ) : (
            <TaskList tasks={recentTasks} />
          )}
        </Panel>
      </div>
    </div>
  );
}

function MetricTile({ label, value }: { label: string; value: number }) {
  return (
    <div className="metric-tile">
      <span className="metric-tile__label">{label}</span>
      <strong className="metric-tile__value">{value}</strong>
    </div>
  );
}

function TaskList({ tasks }: { tasks: OverviewResponse['recent_tasks'] }) {
  return (
    <ul className="task-list">
      {tasks.map((task) => (
        <li key={task.task_id} className="task-list__item">
          <div className="task-list__summary">
            <div>
              <p className="task-list__title">{task.description}</p>
              <p className="task-list__meta">
                {task.agent ?? 'default agent'} · {new Date(task.created_at).toLocaleString()}
              </p>
            </div>
            <StatusBadge status={task.status} />
          </div>
          <Link className="task-list__link" to={`/tasks/${task.task_id}`}>
            Open task
          </Link>
        </li>
      ))}
    </ul>
  );
}
