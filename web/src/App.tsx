import { useState, useEffect, useCallback, useMemo } from 'react';
import './App.css';
import { api } from './api';
import type { Task, Agent, Skill } from './types';
import { Agents } from './pages/Agents/Agents';
import { Skills } from './pages/Skills/Skills';
import { Tasks } from './pages/Tasks/Tasks';

// 格式化时间显示
function formatTimeAgo(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp * 1000;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return '刚刚';
  if (minutes < 60) return `${minutes}分钟前`;
  if (hours < 24) return `${hours}小时前`;
  return `${days}天前`;
}

// 状态颜色映射
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

// 状态文本映射
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

// 导航项类型
type PageType = 'dashboard' | 'tasks' | 'agents' | 'skills';

// 侧边栏导航组件
function Sidebar({ currentPage, onPageChange }: { currentPage: PageType; onPageChange: (page: PageType) => void }) {
  const navItems: { key: PageType; label: string; icon: string }[] = [
    { key: 'dashboard', label: 'Dashboard', icon: '📊' },
    { key: 'tasks', label: 'Tasks', icon: '📋' },
    { key: 'agents', label: 'Agents', icon: '🤖' },
    { key: 'skills', label: 'Skills', icon: '🛠️' },
  ];

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="brand-icon">🧠</span>
        <span className="brand-text">Babata</span>
      </div>
      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <button
            key={item.key}
            className={`nav-item ${currentPage === item.key ? 'active' : ''}`}
            onClick={() => onPageChange(item.key)}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
          </button>
        ))}
      </nav>
    </aside>
  );
}

// Dashboard 页面
function DashboardPage() {
  // 统计数据
  const [runningCount, setRunningCount] = useState<number>(0);
  const [totalCount, setTotalCount] = useState<number>(0);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);

  // 任务列表
  const [tasks, setTasks] = useState<Task[]>([]);
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());

  // 创建任务表单
  const [selectedAgent, setSelectedAgent] = useState<string>('');
  const [taskDescription, setTaskDescription] = useState<string>('');
  const [isCreating, setIsCreating] = useState<boolean>(false);

  // 加载状态
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  // 获取统计数据
  const fetchStats = useCallback(async () => {
    try {
      const [runningRes, totalRes, agentsRes, skillsRes] = await Promise.all([
        api.getRunningTasksCount(),
        api.getTotalTasksCount(),
        api.getAgents(),
        api.getSkills(),
      ]);
      setRunningCount(runningRes.count);
      setTotalCount(totalRes.count);
      setAgents(agentsRes.agents);
      setSkills(skillsRes.skills);
      if (agentsRes.agents.length > 0 && !selectedAgent) {
        // 优先选择 default agent，否则选择第一个
        const defaultAgent = agentsRes.agents.find(a => a.name === 'default');
        setSelectedAgent(defaultAgent?.name || agentsRes.agents[0].name);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取统计数据失败');
    }
  }, [selectedAgent]);

  // 获取任务列表
  const fetchTasks = useCallback(async () => {
    try {
      const res = await api.getRunningTasks(20);
      setTasks(res.tasks);
      setLastUpdate(new Date());
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取任务列表失败');
    }
  }, []);

  // 刷新所有数据
  const refreshData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    await Promise.all([fetchStats(), fetchTasks()]);
    setIsLoading(false);
  }, [fetchStats, fetchTasks]);

  // 初始加载
  useEffect(() => {
    refreshData();
  }, [refreshData]);

  // 自动刷新（每10秒）
  useEffect(() => {
    const interval = setInterval(() => {
      fetchStats();
      fetchTasks();
    }, 10000);
    return () => clearInterval(interval);
  }, [fetchStats, fetchTasks]);

  // 创建任务
  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
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
      // 刷新任务列表
      await fetchTasks();
      await fetchStats();
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建任务失败');
    } finally {
      setIsCreating(false);
    }
  };

  // 计算根任务及其子任务数
  const rootTasksWithChildren = useMemo(() => {
    // 获取所有根任务（parent_task_id 为 null 或 undefined）
    const rootTasks = tasks.filter(
      (task) => !task.parent_task_id || task.parent_task_id === null
    );

    // 计算每个根任务的子任务数
    return rootTasks.map((rootTask) => {
      const childrenCount = tasks.filter(
        (task) =>
          task.root_task_id === rootTask.task_id &&
          task.parent_task_id &&
          task.parent_task_id !== null
      ).length;
      return { ...rootTask, childrenCount };
    });
  }, [tasks]);

  return (
    <div className="dashboard-page">
      {/* 头部 */}
      <header className="dashboard-header">
        <h1>Dashboard</h1>
        <div className="header-actions">
          <span className="last-update">
            最后更新: {lastUpdate.toLocaleTimeString()}
          </span>
          <button
            className="refresh-btn"
            onClick={refreshData}
            disabled={isLoading}
          >
            {isLoading ? '⏳' : '🔄'} 刷新
          </button>
        </div>
      </header>

      {/* 错误提示 */}
      {error && (
        <div className="error-banner">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {/* 统计卡片 */}
      <section className="stats-grid">
        <div className="stats-card running">
          <div className="stats-icon">🏃</div>
          <div className="stats-content">
            <div className="stats-value">{runningCount}</div>
            <div className="stats-label">运行中任务</div>
          </div>
        </div>
        <div className="stats-card total">
          <div className="stats-icon">📋</div>
          <div className="stats-content">
            <div className="stats-value">{totalCount}</div>
            <div className="stats-label">总任务数</div>
          </div>
        </div>
        <div className="stats-card agents">
          <div className="stats-icon">🤖</div>
          <div className="stats-content">
            <div className="stats-value">{agents.length}</div>
            <div className="stats-label">Agents</div>
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

      {/* 快速创建任务 */}
      <section className="create-task-section">
        <h2>🚀 快速创建任务</h2>
        <form className="create-task-form" onSubmit={handleCreateTask}>
          <select
            value={selectedAgent}
            onChange={(e) => setSelectedAgent(e.target.value)}
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
            onChange={(e) => setTaskDescription(e.target.value)}
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

      {/* 正在运行的根任务 */}
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
                    <span className="task-description">
                      📄 {task.description}
                    </span>
                    <span className="task-agent">🤖 {task.agent}</span>
                  </div>
                  <div className="task-meta">
                    <span
                      className="task-status"
                      style={{ color: getStatusColor(task.status) }}
                    >
                      🔄 {getStatusText(task.status)}
                    </span>
                    <span className="task-time">
                      ⏱️ {formatTimeAgo(task.created_at)}
                    </span>
                    {task.childrenCount > 0 && (
                      <span className="task-children">
                        📎 {task.childrenCount}个子任务
                      </span>
                    )}
                    {task.never_ends && (
                      <span className="task-never-ends">♾️ 常驻</span>
                    )}
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

// Tasks 页面
function TasksPage() {
  return <Tasks />;
}

// Agents 页面包装器
function AgentsPage() {
  return <Agents />;
}

// Skills 页面包装器
function SkillsPage() {
  return <Skills />;
}

// 主应用组件
function App() {
  const [currentPage, setCurrentPage] = useState<PageType>('dashboard');

  return (
    <div className="app">
      <Sidebar currentPage={currentPage} onPageChange={setCurrentPage} />
      <main className="main-content">
        {currentPage === 'dashboard' && <DashboardPage />}
        {currentPage === 'tasks' && <TasksPage />}
        {currentPage === 'agents' && <AgentsPage />}
        {currentPage === 'skills' && <SkillsPage />}
      </main>
    </div>
  );
}

export default App;
