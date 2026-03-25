import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';

import { apiGet, apiPost } from '../api/client';
import {
  type TaskActionResponse,
  type TaskArtifactsResponse,
  type TaskContentResponse,
  type TaskLogsResponse,
  type TaskSummary,
  type TaskTreeResponse,
} from '../api/types';
import { Panel } from '../components/Panel';
import { StatusBadge } from '../components/StatusBadge';
import { usePolling } from '../hooks/usePolling';

export function TaskDetailPage() {
  const { taskId } = useParams<{ taskId: string }>();
  const [runtimeMessage, setRuntimeMessage] = useState<string | null>(null);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [logs, setLogs] = useState<TaskLogsResponse | null>(null);
  const [artifacts, setArtifacts] = useState<TaskArtifactsResponse | null>(null);
  const [outputsLoading, setOutputsLoading] = useState(true);

  const summary = usePolling(
    () => apiGet<TaskSummary>(`/tasks/${taskId}`),
    { intervalMs: 5000 },
  );
  const content = usePolling(
    () => apiGet<TaskContentResponse>(`/tasks/${taskId}/content`),
    { intervalMs: 5000 },
  );
  const tree = usePolling(
    () => apiGet<TaskTreeResponse>(`/tasks/${taskId}/tree`),
    { intervalMs: 5000 },
  );

  useEffect(() => {
    let cancelled = false;

    async function loadOutputs() {
      if (!taskId) {
        return;
      }

      setOutputsLoading(true);
      try {
        const [nextLogs, nextArtifacts] = await Promise.all([
          apiGet<TaskLogsResponse>(`/tasks/${taskId}/logs`),
          apiGet<TaskArtifactsResponse>(`/tasks/${taskId}/artifacts`),
        ]);

        if (!cancelled) {
          setLogs(nextLogs);
          setArtifacts(nextArtifacts);
        }
      } finally {
        if (!cancelled) {
          setOutputsLoading(false);
        }
      }
    }

    void loadOutputs();

    return () => {
      cancelled = true;
    };
  }, [taskId]);

  if (!taskId) {
    return (
      <Panel eyebrow="Task detail" title="Missing task">
        <p className="empty-state">Task ID is missing from the route.</p>
      </Panel>
    );
  }

  async function runAction(action: 'pause' | 'resume' | 'cancel' | 'relaunch') {
    if (!summary.data) {
      return;
    }

    setRuntimeError(null);
    setRuntimeMessage(null);

    if (action === 'cancel' && !window.confirm('Cancel this task?')) {
      return;
    }

    let body: { reason?: string } | undefined;
    if (action === 'relaunch') {
      const reason = window.prompt('Why are you relaunching this task?') ?? '';
      if (!reason.trim()) {
        setRuntimeError('Relaunch reason is required.');
        return;
      }

      body = { reason: reason.trim() };
    }

    try {
      const response = await apiPost<TaskActionResponse, { reason?: string }>(
        `/tasks/${summary.data.task_id}/${action}`,
        body,
      );
      setRuntimeMessage(`${response.action} request accepted.`);
      await Promise.all([summary.refresh(), content.refresh(), tree.refresh()]);
    } catch (error) {
      setRuntimeError(error instanceof Error ? error.message : 'Action failed');
    }
  }

  if (summary.error || content.error || tree.error) {
    return (
      <Panel eyebrow="Task detail" title="Failed to load task detail">
        <p className="empty-state">
          {summary.error?.message ?? content.error?.message ?? tree.error?.message}
        </p>
      </Panel>
    );
  }

  if (!summary.data || !content.data || !tree.data) {
    return (
      <Panel eyebrow="Task detail" title="Loading task detail">
        <p className="empty-state">Loading summary, semantic state, and tree data...</p>
      </Panel>
    );
  }

  return (
    <>
      <Panel
        className="panel--hero"
        eyebrow="Task detail"
        meta={<StatusBadge status={summary.data.status} />}
        title={summary.data.description}
      >
        <p className="panel__lede">
          Task ID {summary.data.task_id} · root {summary.data.root_task_id} · agent{' '}
          {summary.data.agent ?? 'default'}
        </p>
      </Panel>

      <div className="page-grid page-grid--two-up">
        <Panel eyebrow="Summary" title="Control state">
          <dl className="fact-list">
            <div className="fact-list__row">
              <dt>Status</dt>
              <dd>{summary.data.status}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Never ends</dt>
              <dd>{summary.data.never_ends ? 'true' : 'false'}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Created</dt>
              <dd>{new Date(summary.data.created_at).toLocaleString()}</dd>
            </div>
          </dl>
        </Panel>

        <Panel eyebrow="Runtime" title="Actions">
          <div className="task-actions task-actions--stack">
            {summary.data.actions.pause ? (
              <button className="task-actions__button" onClick={() => void runAction('pause')} type="button">
                Pause task
              </button>
            ) : null}
            {summary.data.actions.resume ? (
              <button
                className="task-actions__button"
                onClick={() => void runAction('resume')}
                type="button"
              >
                Resume task
              </button>
            ) : null}
            {summary.data.actions.cancel ? (
              <button
                className="task-actions__button task-actions__button--danger"
                onClick={() => void runAction('cancel')}
                type="button"
              >
                Cancel task
              </button>
            ) : null}
            {summary.data.actions.relaunch ? (
              <button
                className="task-actions__button"
                onClick={() => void runAction('relaunch')}
                type="button"
              >
                Relaunch task
              </button>
            ) : null}
          </div>

          {runtimeMessage ? <p className="runtime-note">{runtimeMessage}</p> : null}
          {runtimeError ? <p className="form-error">{runtimeError}</p> : null}
        </Panel>
      </div>

      <div className="page-grid page-grid--two-up">
        <Panel eyebrow="Semantic state" title="Progress">
          <pre className="markdown-frame">{content.data.progress_markdown}</pre>
        </Panel>

        <Panel eyebrow="Semantic state" title="Task definition">
          <pre className="markdown-frame">{content.data.task_markdown}</pre>
        </Panel>
      </div>

      <div className="page-grid page-grid--two-up">
        <Panel eyebrow="Tree" title="Task relatives">
          <dl className="fact-list">
            <div className="fact-list__row">
              <dt>Parent</dt>
              <dd>{tree.data.parent?.description ?? 'None'}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Current</dt>
              <dd>{tree.data.current.description}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Children</dt>
              <dd>{tree.data.children.length}</dd>
            </div>
          </dl>
        </Panel>

        <Panel eyebrow="Outputs" title="Logs and artifacts">
          {outputsLoading ? <p className="empty-state">Loading outputs...</p> : null}

          {!outputsLoading && logs?.supported === false ? (
            <p className="empty-state">{logs.reason}</p>
          ) : null}

          {!outputsLoading && logs?.entries?.length ? (
            <ul className="signal-list">
              {logs.entries.map((entry) => (
                <li key={entry.path}>
                  <strong>{entry.path}</strong>
                </li>
              ))}
            </ul>
          ) : null}

          {!outputsLoading && artifacts?.artifacts.length ? (
            <ul className="signal-list">
              {artifacts.artifacts.map((artifact) => (
                <li key={artifact.path}>{artifact.path}</li>
              ))}
            </ul>
          ) : null}

          {!outputsLoading &&
          logs?.supported !== true &&
          !artifacts?.artifacts.length &&
          !logs?.entries?.length ? (
            <p className="empty-state">No outputs are currently available.</p>
          ) : null}
        </Panel>
      </div>
    </>
  );
}
