import type { AgentDetail } from '../../../types';
import './AgentCard.css';

interface AgentCardProps {
  agent: AgentDetail;
  onEdit: (agent: AgentDetail) => void;
  onDelete: (agent: AgentDetail) => void;
}

export function AgentCard({ agent, onEdit, onDelete }: AgentCardProps) {
  return (
    <div className="agent-card">
      <div className="agent-card-header">
        <div className="agent-card-icon">🤖</div>
        <div className="agent-card-actions">
          <button
            className="agent-card-btn edit"
            onClick={(e) => {
              e.stopPropagation();
              onEdit(agent);
            }}
            title="编辑"
          >
            ✏️
          </button>
          <button
            className="agent-card-btn delete"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(agent);
            }}
            title="删除"
          >
            🗑️
          </button>
        </div>
      </div>
      <div className="agent-card-content">
        <h3 className="agent-card-name">
          {agent.name}
          {agent.default && <span className="agent-default-badge">默认</span>}
        </h3>
        <p className="agent-card-description">
          {agent.description || '暂无描述'}
        </p>
        <div className="agent-card-meta">
          <div className="agent-meta-item">
            <span className="meta-label">Provider</span>
            <span className="meta-value">{agent.provider}</span>
          </div>
          <div className="agent-meta-item">
            <span className="meta-label">Model</span>
            <span className="meta-value">{agent.model}</span>
          </div>
          <div className="agent-meta-item tools">
            <span className="meta-label">工具</span>
            <span className="meta-value">
              {agent.allowed_tools.length > 0
                ? `${agent.allowed_tools.length} 个`
                : '无'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
