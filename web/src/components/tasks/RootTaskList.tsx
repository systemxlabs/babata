import type { TaskSummary } from '../../api/types';
import { StatusBadge } from '../StatusBadge';

interface RootTaskListProps {
  onSelect: (task: TaskSummary) => void;
  rows: TaskSummary[];
  selectedRootTaskId: string | null;
  selectedTaskId: string | null;
  view: 'root' | 'timeline';
}

export function RootTaskList({
  onSelect,
  rows,
  selectedRootTaskId,
  selectedTaskId,
  view,
}: RootTaskListProps) {
  return (
    <section aria-label="Root task list" className="root-task-list">
      {!rows.length ? <p className="empty-state">No tasks are available yet.</p> : null}

      {rows.length ? (
        <ul className="root-task-list__items">
          {rows.map((task) => {
            const isSelected =
              view === 'timeline'
                ? task.task_id === selectedTaskId
                : task.root_task_id === selectedRootTaskId;

            return (
              <li key={task.task_id}>
                <button
                  aria-label={task.description}
                  aria-pressed={isSelected}
                  className={
                    isSelected
                      ? 'root-task-list__button root-task-list__button--selected'
                      : 'root-task-list__button'
                  }
                  onClick={() => onSelect(task)}
                  type="button"
                >
                  <div className="root-task-list__header">
                    <div>
                      <p className="root-task-list__title">{task.description}</p>
                      <p className="root-task-list__meta">
                        {task.agent ?? 'default agent'} · {task.task_id.slice(0, 8)} ·{' '}
                        {new Date(task.created_at).toLocaleString()}
                      </p>
                    </div>
                    <StatusBadge compact status={task.status} />
                  </div>

                  <p className="root-task-list__context">
                    Root {task.root_task_id.slice(0, 8)}
                    {task.never_ends ? ' · never ends' : ''}
                  </p>
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
