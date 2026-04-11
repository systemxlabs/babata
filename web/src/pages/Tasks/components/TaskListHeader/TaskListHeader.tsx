import { useState, useCallback } from 'react';
import type { TaskFilter, TaskStatus } from '../../../../types';
import { STATUS_LABELS } from '../../../../types';
import './TaskListHeader.css';

interface TaskListHeaderProps {
  filter: TaskFilter;
  agents: string[];
  onFilterChange: (filter: Partial<TaskFilter>) => void;
  loading?: boolean;
}

const STATUS_OPTIONS: (TaskStatus | 'all')[] = ['all', 'running', 'completed', 'failed', 'paused', 'canceled'];

export function TaskListHeader({ filter, agents, onFilterChange, loading }: TaskListHeaderProps) {
  const [searchValue, setSearchValue] = useState(filter.search || '');

  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setSearchValue(e.target.value);
  }, []);

  const handleSearchSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    onFilterChange({ search: searchValue });
  }, [searchValue, onFilterChange]);

  const handleStatusChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    onFilterChange({ status: e.target.value as TaskStatus | 'all' });
  }, [onFilterChange]);

  const handleAgentChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    onFilterChange({ agent: e.target.value });
  }, [onFilterChange]);

  return (
    <div className="task-list-header">
      <div className="filter-row">
        <div className="filter-group">
          <label htmlFor="status-filter">状态:</label>
          <select
            id="status-filter"
            value={filter.status || 'all'}
            onChange={handleStatusChange}
            disabled={loading}
            className="filter-select"
          >
            {STATUS_OPTIONS.map(status => (
              <option key={status} value={status}>
                {STATUS_LABELS[status]}
              </option>
            ))}
          </select>
        </div>

        <div className="filter-group">
          <label htmlFor="agent-filter">Agent:</label>
          <select
            id="agent-filter"
            value={filter.agent || 'all'}
            onChange={handleAgentChange}
            disabled={loading}
            className="filter-select"
          >
            <option value="all">全部</option>
            {agents.map(agent => (
              <option key={agent} value={agent}>
                {agent}
              </option>
            ))}
          </select>
        </div>

        <form className="search-group" onSubmit={handleSearchSubmit}>
          <div className="search-input-wrapper">
            <svg className="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="11" cy="11" r="8"></circle>
              <path d="m21 21-4.35-4.35"></path>
            </svg>
            <input
              type="text"
              placeholder="搜索任务描述..."
              value={searchValue}
              onChange={handleSearchChange}
              disabled={loading}
              className="search-input"
            />
          </div>
          <button type="submit" className="search-btn" disabled={loading}>
            搜索
          </button>
        </form>
      </div>
    </div>
  );
}
