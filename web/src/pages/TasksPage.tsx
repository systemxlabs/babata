import { Link, useSearchParams } from 'react-router-dom';

import { apiGet, apiPost } from '../api/client';
import { type TaskActionResponse, type TaskListResponse } from '../api/types';
import { Panel } from '../components/Panel';
import { StatusBadge } from '../components/StatusBadge';
import { usePolling } from '../hooks/usePolling';

export function TasksPage() {
  const [searchParams, setSearchParams] = useSearchParams();

  const tasks = usePolling(
    () => apiGet<TaskListResponse>(`/tasks${buildQuerySuffix(searchParams)}`),
    { intervalMs: 5000 },
  );

  const status = searchParams.get('status') ?? '';
  const query = searchParams.get('query') ?? '';
  const agent = searchParams.get('agent') ?? '';
  const neverEnds = searchParams.get('never_ends') === 'true';
  const rootOnly = searchParams.get('root_only') === 'true';

  async function runAction(taskId: string, action: 'pause' | 'resume' | 'cancel') {
    if (action === 'cancel' && !window.confirm('Cancel this task?')) {
      return;
    }

    await apiPost<TaskActionResponse>(`/tasks/${taskId}/${action}`);
    await tasks.refresh();
  }

  function updateFilter(name: string, value: string | boolean) {
    const next = new URLSearchParams(searchParams);
    const normalized = typeof value === 'boolean' ? (value ? 'true' : '') : value;

    if (normalized) {
      next.set(name, normalized);
    } else {
      next.delete(name);
    }

    setSearchParams(next);
  }

  return (
    <>
      <Panel
        className="panel--hero"
        eyebrow="Tasks"
        meta={<span className="panel__tag">Explorer</span>}
        title="Task explorer"
      >
        <p className="panel__lede">
          Filter by control state, keep root-focused context in the URL, and fire quick actions
          without leaving the list.
        </p>

        <div className="filters-grid">
          <label className="form-field">
            <span className="form-field__label">Status</span>
            <select
              className="form-field__input"
              onChange={(event) => updateFilter('status', event.target.value)}
              value={status}
            >
              <option value="">All statuses</option>
              <option value="running">Running</option>
              <option value="paused">Paused</option>
              <option value="done">Done</option>
              <option value="canceled">Canceled</option>
            </select>
          </label>

          <label className="form-field">
            <span className="form-field__label">Agent</span>
            <input
              className="form-field__input"
              onChange={(event) => updateFilter('agent', event.target.value)}
              placeholder="codex"
              value={agent}
            />
          </label>

          <label className="form-field">
            <span className="form-field__label">Query</span>
            <input
              className="form-field__input"
              onChange={(event) => updateFilter('query', event.target.value)}
              placeholder="forge, runtime, relaunch..."
              value={query}
            />
          </label>

          <label className="form-toggle">
            <input
              checked={rootOnly}
              onChange={(event) => updateFilter('root_only', event.target.checked)}
              type="checkbox"
            />
            <span>Root tasks only</span>
          </label>

          <label className="form-toggle">
            <input
              checked={neverEnds}
              onChange={(event) => updateFilter('never_ends', event.target.checked)}
              type="checkbox"
            />
            <span>Never-ends only</span>
          </label>
        </div>
      </Panel>

      <Panel eyebrow="Results" title="Task rows">
        {tasks.error ? (
          <p className="empty-state">Failed to load tasks: {tasks.error.message}</p>
        ) : null}

        {!tasks.data && !tasks.error ? <p className="empty-state">Loading tasks...</p> : null}

        {tasks.data?.tasks.length === 0 ? (
          <p className="empty-state">No tasks matched the current filters.</p>
        ) : null}

        {tasks.data?.tasks.length ? (
          <ul className="task-list">
            {tasks.data.tasks.map((task) => (
              <li key={task.task_id} className="task-list__item">
                <div className="task-list__summary">
                  <div>
                    <p className="task-list__title">{task.description}</p>
                    <p className="task-list__meta">
                      {task.agent ?? 'default agent'} · {task.task_id.slice(0, 8)} ·{' '}
                      {new Date(task.created_at).toLocaleString()}
                    </p>
                  </div>
                  <StatusBadge status={task.status} />
                </div>

                <div className="task-actions">
                  <Link className="task-list__link" to={`/tasks/${task.task_id}`}>
                    Open task
                  </Link>
                  {task.actions.pause ? (
                    <button
                      className="task-actions__button"
                      onClick={() => void runAction(task.task_id, 'pause')}
                      type="button"
                    >
                      Pause
                    </button>
                  ) : null}
                  {task.actions.resume ? (
                    <button
                      className="task-actions__button"
                      onClick={() => void runAction(task.task_id, 'resume')}
                      type="button"
                    >
                      Resume
                    </button>
                  ) : null}
                  {task.actions.cancel ? (
                    <button
                      className="task-actions__button task-actions__button--danger"
                      onClick={() => void runAction(task.task_id, 'cancel')}
                      type="button"
                    >
                      Cancel
                    </button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        ) : null}
      </Panel>
    </>
  );
}

function buildQuerySuffix(searchParams: URLSearchParams) {
  const next = new URLSearchParams();

  for (const [key, value] of searchParams.entries()) {
    if (value) {
      next.set(key, value);
    }
  }

  const query = next.toString();
  return query ? `?${query}` : '';
}
