import { useRef, useEffect } from 'react';
import './TaskLogsTab.css';

interface TaskLogsTabProps {
  logs: string[];
  onRefresh?: () => void;
}

export function TaskLogsTab({ logs, onRefresh }: TaskLogsTabProps) {
  const logsEndRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  // 解析日志级别
  const parseLogLevel = (log: string): { level: string; color: string } => {
    const upperLog = log.toUpperCase();
    if (upperLog.includes('[ERROR]') || upperLog.includes(' ERROR ')) {
      return { level: 'ERROR', color: '#ef4444' };
    }
    if (upperLog.includes('[WARN]') || upperLog.includes(' WARNING ') || upperLog.includes(' WARN ')) {
      return { level: 'WARN', color: '#f59e0b' };
    }
    if (upperLog.includes('[INFO]') || upperLog.includes(' INFO ')) {
      return { level: 'INFO', color: '#3b82f6' };
    }
    if (upperLog.includes('[DEBUG]') || upperLog.includes(' DEBUG ')) {
      return { level: 'DEBUG', color: '#8b5cf6' };
    }
    return { level: 'LOG', color: '#94a3b8' };
  };

  // 复制日志到剪贴板
  const handleCopyLogs = () => {
    const logText = logs.join('\n');
    navigator.clipboard.writeText(logText).then(() => {
      alert('日志已复制到剪贴板');
    }).catch(() => {
      alert('复制失败，请手动复制');
    });
  };

  return (
    <div className="task-logs-tab">
      <div className="logs-header">
        <div className="logs-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <line x1="10" y1="9" x2="8" y2="9" />
          </svg>
          任务执行日志
          <span className="logs-count">({logs.length} 条)</span>
        </div>
        <div className="logs-actions">
          <button className="logs-action-btn" onClick={onRefresh} title="刷新">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
              <path d="M3 3v5h5" />
              <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
              <path d="M16 16h5v5" />
            </svg>
            刷新
          </button>
          <button className="logs-action-btn" onClick={handleCopyLogs} title="复制全部">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
              <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
            </svg>
            复制
          </button>
        </div>
      </div>

      <div className="logs-container">
        {logs.length === 0 ? (
          <div className="logs-empty">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="16" y1="13" x2="8" y2="13" />
              <line x1="16" y1="17" x2="8" y2="17" />
              <line x1="10" y1="9" x2="8" y2="9" />
            </svg>
            <p>暂无日志</p>
          </div>
        ) : (
          <div className="logs-list">
            {logs.map((log, index) => {
              const { level, color } = parseLogLevel(log);
              return (
                <div key={index} className="log-line">
                  <span className="log-index">{index + 1}</span>
                  <span className="log-level" style={{ color }}>{level}</span>
                  <span className="log-content">{log}</span>
                </div>
              );
            })}
            <div ref={logsEndRef} />
          </div>
        )}
      </div>
    </div>
  );
}
