import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { apiPost } from '../api/client';
import { type CreateTaskResponse } from '../api/types';
import { Panel } from '../components/Panel';

interface CreateTaskPayload {
  prompt: Array<{ type: 'text'; text: string }>;
  agent?: string;
  parent_task_id?: string;
  never_ends: boolean;
}

export function CreatePage() {
  const navigate = useNavigate();
  const [prompt, setPrompt] = useState('');
  const [agent, setAgent] = useState('codex');
  const [parentTaskId, setParentTaskId] = useState('');
  const [neverEnds, setNeverEnds] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!prompt.trim()) {
      setError('Prompt is required');
      return;
    }

    setError(null);
    setIsSubmitting(true);

    const payload: CreateTaskPayload = {
      prompt: [{ type: 'text', text: prompt.trim() }],
      never_ends: neverEnds,
    };

    if (agent.trim()) {
      payload.agent = agent.trim();
    }

    if (parentTaskId.trim()) {
      payload.parent_task_id = parentTaskId.trim();
    }

    try {
      const response = await apiPost<CreateTaskResponse, CreateTaskPayload>('/tasks', payload);
      navigate(`/tasks/${response.task_id}`);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : 'Failed to create task');
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <Panel
      className="panel--hero"
      eyebrow="Create"
      meta={<span className="panel__tag">Explicit launch</span>}
      title="Launch forge"
    >
      <p className="panel__lede">
        Create a task intentionally: prompt first, optional parent linkage, and a clear
        never-ends toggle for long-running control loops.
      </p>

      <form className="task-form" onSubmit={handleSubmit}>
        <label className="form-field form-field--full">
          <span className="form-field__label">Prompt</span>
          <textarea
            className="form-field__input form-field__textarea"
            onChange={(event) => setPrompt(event.target.value)}
            placeholder="Describe the task to run..."
            rows={7}
            value={prompt}
          />
        </label>

        <div className="form-grid">
          <label className="form-field">
            <span className="form-field__label">Agent</span>
            <input
              className="form-field__input"
              onChange={(event) => setAgent(event.target.value)}
              placeholder="codex"
              value={agent}
            />
          </label>

          <label className="form-field">
            <span className="form-field__label">Parent task ID</span>
            <input
              className="form-field__input"
              onChange={(event) => setParentTaskId(event.target.value)}
              placeholder="Optional root or parent task"
              value={parentTaskId}
            />
          </label>
        </div>

        <label className="form-toggle">
          <input
            checked={neverEnds}
            onChange={(event) => setNeverEnds(event.target.checked)}
            type="checkbox"
          />
          <span>Keep the task alive after completion signals (`never_ends`).</span>
        </label>

        {error ? <p className="form-error">{error}</p> : null}

        <div className="form-actions">
          <button className="toolbar__button" disabled={isSubmitting} type="submit">
            {isSubmitting ? 'Creating...' : 'Create task'}
          </button>
        </div>
      </form>
    </Panel>
  );
}
