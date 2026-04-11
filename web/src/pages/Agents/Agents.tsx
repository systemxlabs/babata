import { useState, useEffect } from 'react';
import { api } from '../../api';
import type { Agent } from '../../types';
import './Agents.css';

export function Agents() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchAgents = async () => {
      try {
        setLoading(true);
        const response = await api.getAgents();
        setAgents(response.agents);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : '获取 Agent 列表失败');
      } finally {
        setLoading(false);
      }
    };

    fetchAgents();
  }, []);

  if (loading) {
    return (
      <div className="agents-page">
        <h1>🤖 Agent 管理</h1>
        <div className="loading-state">
          <div className="loading-spinner"></div>
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="agents-page">
        <h1>🤖 Agent 管理</h1>
        <div className="error-state">
          <p>❌ {error}</p>
          <button onClick={() => window.location.reload()}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="agents-page">
      <div className="agents-header">
        <h1>🤖 Agent 管理</h1>
        <span className="agents-count">共 {agents.length} 个 Agent</span>
      </div>

      {agents.length === 0 ? (
        <div className="empty-state">
          <p>📭 暂无配置的 Agent</p>
        </div>
      ) : (
        <div className="agents-grid">
          {agents.map((agent) => (
            <div key={agent.name} className="agent-card">
              <div className="agent-icon">🤖</div>
              <div className="agent-info">
                <h3 className="agent-name">{agent.name}</h3>
                <p className="agent-description">
                  {agent.description || '暂无描述'}
                </p>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
