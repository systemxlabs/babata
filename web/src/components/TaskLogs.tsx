import React, { useState, useEffect, useRef, useCallback } from 'react';
import { getTaskLogs } from '../api';
import type { LogEntry } from '../types';
import './TaskLogs.css';

interface TaskLogsProps {
  taskId: string;
}

export function TaskLogs({ taskId }: TaskLogsProps) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const logsContainerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  const fetchLogs = useCallback(async (showLoading = true) => {
    if (showLoading) {
      setLoading(true);
    } else {
      setRefreshing(true);
    }
    setError(null);
    try {
      const data = await getTaskLogs(taskId);
      setLogs(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load logs');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [taskId]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && logsEndRef.current && logsContainerRef.current) {
      logsContainerRef.current.scrollTop = logsContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const handleScroll = () => {
    if (logsContainerRef.current) {
      const { scrollTop, scrollHeight, clientHeight } = logsContainerRef.current;
      const isNearBottom = scrollHeight - scrollTop - clientHeight < 50;
      setAutoScroll(isNearBottom);
    }
  };

  const highlightLog = (log: string): React.ReactElement => {
    // 匹配时间戳格式: 2024-01-15 10:30:45 或 2024-01-15T10:30:45
    const timestampRegex = /(\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{3})?(?:Z|[+-]\d{2}:\d{2})?)/;
    // 匹配日志级别: [INFO], [ERROR], [WARN], [DEBUG], [TRACE]
    const levelRegex = /\[(INFO|ERROR|WARN|DEBUG|TRACE)\]/;

    const parts: (string | React.ReactElement)[] = [];
    let remaining = log;
    let key = 0;

    while (remaining.length > 0) {
      const timestampMatch = remaining.match(timestampRegex);
      const levelMatch = remaining.match(levelRegex);

      let nextMatch: { index: number; length: number; type: 'timestamp' | 'level'; text: string } | null = null;

      if (timestampMatch && timestampMatch.index !== undefined) {
        nextMatch = {
          index: timestampMatch.index,
          length: timestampMatch[0].length,
          type: 'timestamp',
          text: timestampMatch[0],
        };
      }

      if (levelMatch && levelMatch.index !== undefined) {
        if (!nextMatch || levelMatch.index < nextMatch.index) {
          nextMatch = {
            index: levelMatch.index,
            length: levelMatch[0].length,
            type: 'level',
            text: levelMatch[0],
          };
        }
      }

      if (nextMatch) {
        if (nextMatch.index > 0) {
          parts.push(remaining.substring(0, nextMatch.index));
        }

        if (nextMatch.type === 'timestamp') {
          parts.push(
            <span key={key++} className="log-timestamp">
              {nextMatch.text}
            </span>
          );
        } else {
          const levelClass = nextMatch.text.toLowerCase().replace(/[[\]]/g, '');
          parts.push(
            <span key={key++} className={`log-level log-level-${levelClass}`}>
              {nextMatch.text}
            </span>
          );
        }

        remaining = remaining.substring(nextMatch.index + nextMatch.length);
      } else {
        parts.push(remaining);
        break;
      }
    }

    return <>{parts}</>;
  };

  if (loading) {
    return (
      <div className="task-logs">
        <div className="logs-loading">
          <div className="spinner"></div>
          <span>加载日志...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="task-logs">
        <div className="logs-error">
          <p>加载失败</p>
          <p className="error-message">{error}</p>
          <button onClick={() => fetchLogs()} className="retry-btn">重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="task-logs">
      <div className="logs-toolbar">
        <div className="logs-info">
          <span className="logs-count">{logs.length} 条日志</span>
          <label className="auto-scroll-toggle">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
            />
            自动滚动
          </label>
        </div>
        <button
          className={`refresh-btn ${refreshing ? 'refreshing' : ''}`}
          onClick={() => fetchLogs(false)}
          disabled={refreshing}
          aria-label="刷新日志"
        >
          <svg className="refresh-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" />
          </svg>
          刷新
        </button>
      </div>

      <div
        className="logs-container"
        ref={logsContainerRef}
        onScroll={handleScroll}
      >
        {logs.length === 0 ? (
          <div className="logs-empty">
            <p>📝 暂无日志</p>
            <p className="empty-hint">任务尚未产生日志或日志已被清理</p>
          </div>
        ) : (
          <div className="logs-list">
            {logs.map((log, index) => (
              <div key={index} className="log-line">
                <span className="log-line-number">{index + 1}</span>
                <span className="log-content">{highlightLog(log)}</span>
              </div>
            ))}
            <div ref={logsEndRef} />
          </div>
        )}
      </div>

      {!autoScroll && logs.length > 0 && (
        <button
          className="scroll-to-bottom-btn"
          onClick={() => {
            setAutoScroll(true);
            if (logsContainerRef.current) {
              logsContainerRef.current.scrollTop = logsContainerRef.current.scrollHeight;
            }
          }}
        >
          ↓ 滚动到底部
        </button>
      )}
    </div>
  );
}
