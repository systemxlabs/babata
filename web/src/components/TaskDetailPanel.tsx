import { useState, useEffect, useCallback } from 'react';
import type { Task } from '../types';
import { getTask } from '../api';
import { TaskFiles } from './TaskFiles';
import { TaskLogs } from './TaskLogs';
import './TaskDetailPanel.css';

type Tab = 'files' | 'logs';

interface TaskDetailPanelProps {
  taskId: string;
  onTaskClick?: (taskId: string) => void;
  onClose?: () => void;
}

export function TaskDetailPanel({ taskId, onTaskClick, onClose }: TaskDetailPanelProps) {
  const [task, setTask] = useState<Task | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>('files');

  const fetchTask = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getTask(taskId);
      setTask(data as Task);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load task');
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  useEffect(() => {
    fetchTask();
  }, [fetchTask]);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running':
        return '#3b82f6'; // blue
      case 'completed':
        return '#22c55e'; // green
      case 'failed':
        return '#ef4444'; // red
      case 'canceled':
        return '#6b7280'; // gray
      case 'paused':
        return '#f59e0b'; // amber
      default:
        return '#6b7280';
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  if (loading) {
    return (
      <div className="task-detail-panel">
        <div className="task-detail-loading">加载中...</div>
      </div>
    );
  }

  if (error || !task) {
    return (
      <div className="task-detail-panel">
        <div className="task-detail-error">
          <p>加载失败</p>
          <p className="error-message">{error}</p>
          <button onClick={fetchTask} className="retry-btn">重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="task-detail-panel">
      <div className="task-detail-header">
        <h3>任务详情</h3>
        {onClose && (
          <button className="close-btn" onClick={onClose} aria-label="关闭">
            ×
          </button>
        )}
      </div>

      <div className="task-info-section">
        <div className="info-row">
          <span className="info-label">描述</span>
          <span className="info-value description">{task.description}</span>
        </div>
        <div className="info-row">
          <span className="info-label">Agent</span>
          <span className="info-value">{task.agent}</span>
        </div>
        <div className="info-row">
          <span className="info-label">状态</span>
          <span
            className="info-value status-badge"
            style={{ backgroundColor: getStatusColor(task.status) }}
          >
            {task.status}
          </span>
        </div>
        <div className="info-row">
          <span className="info-label">创建时间</span>
          <span className="info-value">{formatDate(task.created_at)}</span>
        </div>
        {task.parent_task_id && (
          <div className="info-row">
            <span className="info-label">父任务</span>
            <button
              className="info-value link-btn"
              onClick={() => onTaskClick?.(task.parent_task_id!)}
            >
              {task.parent_task_id}
            </button>
          </div>
        )}
        {task.root_task_id && (
          <div className="info-row">
            <span className="info-label">根任务</span>
            <button
              className="info-value link-btn"
              onClick={() => onTaskClick?.(task.root_task_id!)}
            >
              {task.root_task_id}
            </button>
          </div>
        )}
        {task.never_ends && (
          <div className="info-row">
            <span className="info-label">常驻任务</span>
            <span className="info-value permanent-badge">是</span>
          </div>
        )}
      </div>

      <div className="task-tabs">
        <button
          className={`tab-btn ${activeTab === 'files' ? 'active' : ''}`}
          onClick={() => setActiveTab('files')}
        >
          目录
        </button>
        <button
          className={`tab-btn ${activeTab === 'logs' ? 'active' : ''}`}
          onClick={() => setActiveTab('logs')}
        >
          日志
        </button>
      </div>

      <div className="tab-content">
        {activeTab === 'files' ? (
          <TaskFiles taskId={taskId} />
        ) : (
          <TaskLogs taskId={taskId} />
        )}
      </div>
    </div>
  );
}
