import { useState, useEffect, useCallback } from 'react';
import type { Task, FileEntry } from '../../types';
import { getTask, getTaskFiles, getTaskLogs, controlTask } from '../../api';
import { TaskStatusBadge } from '../../pages/Tasks/components/TaskStatusBadge';
import { TaskDirectoryTab } from './components/TaskDirectoryTab/TaskDirectoryTab';
import { TaskLogsTab } from './components/TaskLogsTab/TaskLogsTab';
import './TaskDetailModal.css';

type TabType = 'directory' | 'logs';

interface TaskDetailModalProps {
  taskId: string | null;
  isOpen: boolean;
  onClose: () => void;
}

export function TaskDetailModal({ taskId, isOpen, onClose }: TaskDetailModalProps) {
  const [task, setTask] = useState<Task | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [activeTab, setActiveTab] = useState<TabType>('directory');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchTaskDetail = useCallback(async () => {
    if (!taskId) return;
    
    setLoading(true);
    setError(null);
    
    try {
      const [taskData, filesData, logsData] = await Promise.all([
        getTask(taskId),
        getTaskFiles(taskId),
        getTaskLogs(taskId, 1000),
      ]);
      
      setTask(taskData);
      setFiles(filesData);
      setLogs(logsData);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载任务详情失败');
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  useEffect(() => {
    if (isOpen && taskId) {
      fetchTaskDetail();
    }
  }, [isOpen, taskId, fetchTaskDetail]);

  // 处理任务控制操作
  const handleControlTask = async (action: 'pause' | 'resume' | 'cancel') => {
    if (!taskId) return;
    
    try {
      await controlTask(taskId, action);
      // 刷新任务状态
      fetchTaskDetail();
    } catch (err) {
      alert(err instanceof Error ? err.message : '操作失败');
    }
  };

  // 格式化时间戳
  const formatTime = (timestamp: string | number): string => {
    const date = new Date(timestamp);
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  // 获取可用的控制按钮
  const getControlButtons = () => {
    if (!task) return null;
    
    const buttons = [];
    
    if (task.status === 'running') {
      buttons.push(
        <button
          key="pause"
          className="control-btn pause"
          onClick={() => handleControlTask('pause')}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="6" y="4" width="4" height="16" />
            <rect x="14" y="4" width="4" height="16" />
          </svg>
          暂停
        </button>
      );
      buttons.push(
        <button
          key="cancel"
          className="control-btn cancel"
          onClick={() => handleControlTask('cancel')}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <path d="m15 9-6 6" />
            <path d="m9 9 6 6" />
          </svg>
          取消
        </button>
      );
    } else if (task.status === 'paused') {
      buttons.push(
        <button
          key="resume"
          className="control-btn resume"
          onClick={() => handleControlTask('resume')}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
          恢复
        </button>
      );
    }
    
    return buttons;
  };

  if (!isOpen) return null;

  return (
    <div className="task-detail-modal-overlay" onClick={onClose}>
      <div className="task-detail-modal" onClick={(e) => e.stopPropagation()}>
        {/* 头部 */}
        <div className="modal-header">
          <h2>任务详情</h2>
          <button className="close-btn" onClick={onClose}>
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>

        {/* 内容 */}
        <div className="modal-content">
          {loading ? (
            <div className="modal-loading">
              <div className="loading-spinner"></div>
              <p>加载中...</p>
            </div>
          ) : error ? (
            <div className="modal-error">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="10" />
                <path d="m15 9-6 6" />
                <path d="m9 9 6 6" />
              </svg>
              <p>{error}</p>
              <button className="retry-btn" onClick={fetchTaskDetail}>重试</button>
            </div>
          ) : task ? (
            <>
              {/* 任务信息面板 */}
              <div className="task-info-panel">
                <div className="info-row">
                  <span className="info-label">ID:</span>
                  <span className="info-value task-id">{task.task_id}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">描述:</span>
                  <span className="info-value">{task.description}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">状态:</span>
                  <TaskStatusBadge status={task.status} size="md" />
                </div>
                <div className="info-row">
                  <span className="info-label">Agent:</span>
                  <span className="info-value">{task.agent}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">创建时间:</span>
                  <span className="info-value">{formatTime(task.created_at)}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Never Ends:</span>
                  <span className="info-value">{task.never_ends ? '是' : '否'}</span>
                </div>
              </div>

              {/* 控制按钮 */}
              <div className="task-controls">
                {getControlButtons()}
              </div>

              {/* 标签页 */}
              <div className="modal-tabs">
                <button
                  className={`tab-btn ${activeTab === 'directory' ? 'active' : ''}`}
                  onClick={() => setActiveTab('directory')}
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
                  </svg>
                  任务目录
                </button>
                <button
                  className={`tab-btn ${activeTab === 'logs' ? 'active' : ''}`}
                  onClick={() => setActiveTab('logs')}
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                    <polyline points="14 2 14 8 20 8" />
                    <line x1="16" y1="13" x2="8" y2="13" />
                    <line x1="16" y1="17" x2="8" y2="17" />
                    <line x1="10" y1="9" x2="8" y2="9" />
                  </svg>
                  任务日志
                </button>
              </div>

              {/* 标签页内容 */}
              <div className="tab-content">
                {activeTab === 'directory' && taskId && (
                  <TaskDirectoryTab taskId={taskId} files={files} />
                )}
                {activeTab === 'logs' && (
                  <TaskLogsTab logs={logs} onRefresh={fetchTaskDetail} />
                )}
              </div>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
