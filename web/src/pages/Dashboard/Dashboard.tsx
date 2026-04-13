import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from '../../api';
import type { Agent, ProviderConfig, Skill, Task } from '../../types';

function formatTimeAgo(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes}分钟前`;
  if (hours < 24) return `${hours}小时前`;
  return `${days}天前`;
}

function getStatusColor(status: string): string {
  switch (status) {
    case 'running':
      return '#52c41a';
    case 'paused':
      return '#faad14';
    case 'failed':
      return '#f5222d';
    case 'canceled':
      return '#8c8c8c';
    case 'completed':
      return '#1890ff';
    default:
      return '#8c8c8c';
  }
}

function getStatusText(status: string): string {
  switch (status) {
    case 'running':
      return '运行中';
    case 'paused':
      return '已暂停';
    case 'failed':
      return '失败';
    case 'canceled':
      return '已取消';
    case 'completed':
      return '已完成';
    default:
      return status;
  }
}

export function Dashboard() {
  const [runningCount, setRunningCount] = useState(0);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());
  const [selectedAgent, setSelectedAgent] = useState('');
  const [taskDescription, setTaskDescription] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStats = useCallback(async () => {
    try {
      const [runningRes, agentsRes, providersRes, skillsRes] = await Promise.all([
        api.getRunningTasksCount(),
        api.getAgents(),
        api.getProviders(),
        api.getSkills(),
      ]);

      setRunningCount(runningRes.count);
      setAgents(agentsRes.agents);
      setProviders(providersRes.providers);
      setSkills(skillsRes.skills);

      if (agentsRes.agents.length > 0 && !selectedAgent) {
        const defaultAgent = agentsRes.agents.find((agent) => agent.default);
        setSelectedAgent(defaultAgent?.name ?? agentsRes.agents[0].name);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取统计数据失败');
    }
  }, [selectedAgent]);

  const fetchTasks = useCallback(async () => {
    try {
      const response = await api.getRunningTasks(20);
      setTasks(response.tasks);
      setLastUpdate(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取任务列表失败');
    }
  }, []);

  const refreshData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    await Promise.all([fetchStats(), fetchTasks()]);
    setIsLoading(false);
  }, [fetchStats, fetchTasks]);

  useEffect(() => {
    refreshData();
  }, [refreshData]);

  useEffect(() => {
    const interval = setInterval(() => {
      void fetchStats();
      void fetchTasks();
    }, 10000);

    return () => clearInterval(interval);
  }, [fetchStats, fetchTasks]);

  const handleCreateTask = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!selectedAgent || !taskDescription.trim()) return;

    setIsCreating(true);
    try {
      await api.createTask({
        agent: selectedAgent,
        prompt: taskDescription.trim(),
        description: taskDescription.trim(),
        task_type: 'roottask',
      });
      setTaskDescription('');
      await refreshData();
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建任务失败');
    } finally {
      setIsCreating(false);
    }
  };

  const rootTasksWithChildren = useMemo(() => {
    const rootTasks = tasks.filter((task) => !task.parent_task_id);

    return rootTasks.map((rootTask) => {
      const childrenCount = tasks.filter(
        (task) => task.root_task_id === rootTask.task_id && task.parent_task_id
      ).length;
      return { ...rootTask, childrenCount };
    });
  }, [tasks]);

  return (
    <div className="dashboard-page">
      <header className="dashboard-header">
        <h1>Dashboard</h1>
        <div className="header-actions">
          <span className="last-update">最后更新: {lastUpdate.toLocaleTimeString()}</span>
          <button className="refresh-btn" onClick={() => void refreshData()} disabled={isLoading}>
            {isLoading ? '⏳' : '🔄'} 刷新
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      <section className="stats-grid">
        <div className="stats-card running">
          <div className="stats-icon">🏃</div>
          <div className="stats-content">
            <div className="stats-value">{runningCount}</div>
            <div className="stats-label">运行中任务</div>
          </div>
        </div>
        <div className="stats-card root-tasks">
          <div className="stats-icon">🌳</div>
          <div className="stats-content">
            <div className="stats-value">{rootTasksWithChildren.length}</div>
            <div className="stats-label">运行中的根任务</div>
          </div>
        </div>
        <div className="stats-card agents">
          <div className="stats-icon">🤖</div>
          <div className="stats-content">
            <div className="stats-value">{agents.length}</div>
            <div className="stats-label">Agents</div>
          </div>
        </div>
        <div className="stats-card providers">
          <div className="stats-icon">🔌</div>
          <div className="stats-content">
            <div className="stats-value">{providers.length}</div>
            <div className="stats-label">Providers</div>
          </div>
        </div>
        <div className="stats-card skills">
          <div className="stats-icon">🛠️</div>
          <div className="stats-content">
            <div className="stats-value">{skills.length}</div>
            <div className="stats-label">Skills</div>
          </div>
        </div>
      </section>

      <section className="create-task-section">
        <h2>🚀 快速创建任务</h2>
        <form className="create-task-form" onSubmit={handleCreateTask}>
          <select
            value={selectedAgent}
            onChange={(event) => setSelectedAgent(event.target.value)}
            disabled={isCreating || agents.length === 0}
            className="agent-select"
          >
            {agents.map((agent) => (
              <option key={agent.name} value={agent.name}>
                🤖 {agent.name}
              </option>
            ))}
          </select>
          <input
            type="text"
            placeholder="输入任务描述..."
            value={taskDescription}
            onChange={(event) => setTaskDescription(event.target.value)}
            disabled={isCreating}
            className="task-input"
          />
          <button
            type="submit"
            disabled={isCreating || !selectedAgent || !taskDescription.trim()}
            className="create-btn"
          >
            {isCreating ? '⏳ 创建中...' : '➕ 创建'}
          </button>
        </form>
      </section>

      <section className="tasks-section">
        <h2>
          ▶️ 正在运行的根任务
          <span className="task-count">({rootTasksWithChildren.length})</span>
        </h2>
        {rootTasksWithChildren.length === 0 ? (
          <div className="empty-state">
            <p>📭 暂无运行中的根任务</p>
          </div>
        ) : (
          <div className="task-list">
            {rootTasksWithChildren.map((task) => (
              <div key={task.task_id} className="task-item">
                <div className="task-main">
                  <div className="task-header">
                    <span className="task-description">📄 {task.description}</span>
                    <span className="task-agent">🤖 {task.agent}</span>
                  </div>
                  <div className="task-meta">
                    <span className="task-status" style={{ color: getStatusColor(task.status) }}>
                      🔄 {getStatusText(task.status)}
                    </span>
                    <span className="task-time">⏱️ {formatTimeAgo(task.created_at)}</span>
                    {task.childrenCount > 0 && (
                      <span className="task-children">📎 {task.childrenCount}个子任务</span>
                    )}
                    {task.never_ends && <span className="task-never-ends">♾️ 常驻</span>}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
