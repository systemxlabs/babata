import type { TaskTreeNode } from '../../api/types';
import { StatusBadge } from '../StatusBadge';

interface TaskTreePaneProps {
  expandedIds: ReadonlySet<string>;
  onSelect: (taskId: string) => void;
  onToggle: (taskId: string) => void;
  root: TaskTreeNode | null;
  selectedTaskId: string | null;
}

export function TaskTreePane({
  expandedIds,
  onSelect,
  onToggle,
  root,
  selectedTaskId,
}: TaskTreePaneProps) {
  return (
    <section aria-label="Task tree" className="task-tree-pane">
      {!root ? <p className="empty-state">Select a root task to inspect its hierarchy.</p> : null}

      {root ? (
        <ul className="task-tree-list">
          <TaskTreeBranch
            expandedIds={expandedIds}
            node={root}
            onSelect={onSelect}
            onToggle={onToggle}
            selectedTaskId={selectedTaskId}
          />
        </ul>
      ) : null}
    </section>
  );
}

interface TaskTreeBranchProps {
  expandedIds: ReadonlySet<string>;
  node: TaskTreeNode;
  onSelect: (taskId: string) => void;
  onToggle: (taskId: string) => void;
  selectedTaskId: string | null;
}

function TaskTreeBranch({
  expandedIds,
  node,
  onSelect,
  onToggle,
  selectedTaskId,
}: TaskTreeBranchProps) {
  const isExpanded = expandedIds.has(node.task.task_id);
  const isSelected = selectedTaskId === node.task.task_id;

  return (
    <li className="task-tree-node">
      <div className="task-tree-node__row">
        {node.children.length ? (
          <button
            aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${node.task.description}`}
            className="task-tree-node__toggle"
            onClick={() => onToggle(node.task.task_id)}
            type="button"
          >
            {isExpanded ? '−' : '+'}
          </button>
        ) : (
          <span className="task-tree-node__spacer" aria-hidden="true" />
        )}

        <button
          aria-label={node.task.description}
          aria-pressed={isSelected}
          className={
            isSelected ? 'task-tree-node__select task-tree-node__select--selected' : 'task-tree-node__select'
          }
          onClick={() => onSelect(node.task.task_id)}
          type="button"
        >
          <span className="task-tree-node__title">{node.task.description}</span>
          <span className="task-tree-node__meta">
            {node.task.task_id.slice(0, 8)} · {new Date(node.task.created_at).toLocaleString()}
          </span>
        </button>

        <StatusBadge compact status={node.task.status} />
      </div>

      {node.children.length && isExpanded ? (
        <ul className="task-tree-list task-tree-list--nested">
          {node.children.map((child) => (
            <TaskTreeBranch
              expandedIds={expandedIds}
              key={child.task.task_id}
              node={child}
              onSelect={onSelect}
              onToggle={onToggle}
              selectedTaskId={selectedTaskId}
            />
          ))}
        </ul>
      ) : null}
    </li>
  );
}
