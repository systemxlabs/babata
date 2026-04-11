import { useState, useEffect, useCallback } from 'react';
import { api, deleteAgent } from '../../api';
import type { AgentFrontmatter, AgentDetail, CreateAgentRequest, UpdateAgentRequest } from '../../types';
import './Agents.css';

// 创建/编辑 Agent 弹窗
interface AgentModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: CreateAgentRequest | UpdateAgentRequest) => Promise<void>;
  agent?: AgentDetail | null;
  mode: 'create' | 'edit';
}

function AgentModal({ isOpen, onClose, onSubmit, agent, mode }: AgentModalProps) {
  const [formData, setFormData] = useState<CreateAgentRequest>({
    name: '',
    description: '',
    provider: 'openai',
    model: 'gpt-4',
    allowed_tools: [],
    default: false,
    body: '',
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen && agent && mode === 'edit') {
      setFormData({
        name: agent.name,
        description: agent.description,
        provider: agent.provider,
        model: agent.model,
        allowed_tools: agent.allowed_tools,
        default: agent.default,
        body: agent.body,
      });
    } else if (isOpen && mode === 'create') {
      setFormData({
        name: '',
        description: '',
        provider: 'openai',
        model: 'gpt-4',
        allowed_tools: [],
        default: false,
        body: '',
      });
    }
    setError(null);
  }, [isOpen, agent, mode]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.name.trim() || !formData.body.trim()) {
      setError('名称和 Body 不能为空');
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await onSubmit(formData);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : '操作失败');
    } finally {
      setLoading(false);
    }
  };

  const handleToolsChange = (value: string) => {
    const tools = value.split(',').map(t => t.trim()).filter(t => t);
    setFormData({ ...formData, allowed_tools: tools });
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>{mode === 'create' ? '➕ 创建 Agent' : '✏️ 编辑 Agent'}</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            {error && <div className="modal-error">❌ {error}</div>}
            <div className="form-group">
              <label>名称 *</label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                disabled={mode === 'edit' || loading}
                placeholder="输入 Agent 名称"
                required
              />
            </div>
            <div className="form-group">
              <label>描述</label>
              <input
                type="text"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                disabled={loading}
                placeholder="输入 Agent 描述"
              />
            </div>
            <div className="form-row">
              <div className="form-group">
                <label>Provider</label>
                <select
                  value={formData.provider}
                  onChange={(e) => setFormData({ ...formData, provider: e.target.value })}
                  disabled={loading}
                >
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="google">Google</option>
                  <option value="local">Local</option>
                </select>
              </div>
              <div className="form-group">
                <label>Model</label>
                <input
                  type="text"
                  value={formData.model}
                  onChange={(e) => setFormData({ ...formData, model: e.target.value })}
                  disabled={loading}
                  placeholder="gpt-4"
                />
              </div>
            </div>
            <div className="form-group">
              <label>允许的工具（逗号分隔）</label>
              <input
                type="text"
                value={formData.allowed_tools.join(', ')}
                onChange={(e) => handleToolsChange(e.target.value)}
                disabled={loading}
                placeholder="shell, read_file, write_file"
              />
            </div>
            <div className="form-group checkbox">
              <label>
                <input
                  type="checkbox"
                  checked={formData.default}
                  onChange={(e) => setFormData({ ...formData, default: e.target.checked })}
                  disabled={loading}
                />
                设为默认 Agent
              </label>
            </div>
            <div className="form-group">
              <label>Body *</label>
              <textarea
                value={formData.body}
                onChange={(e) => setFormData({ ...formData, body: e.target.value })}
                disabled={loading}
                placeholder="输入 Agent 的系统提示词..."
                rows={8}
                required
              />
            </div>
          </div>
          <div className="modal-footer">
            <button type="button" className="btn-secondary" onClick={onClose} disabled={loading}>
              取消
            </button>
            <button type="submit" className="btn-primary" disabled={loading}>
              {loading ? '⏳ 保存中...' : mode === 'create' ? '➕ 创建' : '💾 保存'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// 删除确认弹窗
interface DeleteModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  agentName: string;
}

function DeleteModal({ isOpen, onClose, onConfirm, agentName }: DeleteModalProps) {
  const [loading, setLoading] = useState(false);

  if (!isOpen) return null;

  const handleConfirm = async () => {
    setLoading(true);
    try {
      await onConfirm();
      onClose();
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content modal-small" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>🗑️ 确认删除</h2>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="modal-body">
          <p className="delete-warning">
            确定要删除 Agent <strong>"{agentName}"</strong> 吗？<br />
            此操作不可撤销。
          </p>
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" onClick={onClose} disabled={loading}>
            取消
          </button>
          <button className="btn-danger" onClick={handleConfirm} disabled={loading}>
            {loading ? '⏳ 删除中...' : '🗑️ 删除'}
          </button>
        </div>
      </div>
    </div>
  );
}

// Agent 卡片组件
interface AgentCardProps {
  agent: AgentFrontmatter;
  onEdit: (agent: AgentFrontmatter) => void;
  onDelete: (agent: AgentFrontmatter) => void;
}

function AgentCard({ agent, onEdit, onDelete }: AgentCardProps) {
  return (
    <div className="agent-card">
      <div className="agent-card-header">
        <div className="agent-icon">🤖</div>
        <div className="agent-badges">
          {agent.default && <span className="badge badge-default">默认</span>}
        </div>
      </div>
      <div className="agent-info">
        <h3 className="agent-name">{agent.name}</h3>
        <p className="agent-description">{agent.description || '暂无描述'}</p>
        <div className="agent-meta">
          <span className="agent-provider">🔌 {agent.provider}</span>
          <span className="agent-model">🧠 {agent.model}</span>
        </div>
        {agent.allowed_tools.length > 0 && (
          <div className="agent-tools">
            {agent.allowed_tools.slice(0, 3).map((tool) => (
              <span key={tool} className="tool-tag">{tool}</span>
            ))}
            {agent.allowed_tools.length > 3 && (
              <span className="tool-tag tool-more">+{agent.allowed_tools.length - 3}</span>
            )}
          </div>
        )}
      </div>
      <div className="agent-actions">
        <button className="btn-icon" onClick={() => onEdit(agent)} title="编辑">
          ✏️
        </button>
        <button className="btn-icon btn-icon-danger" onClick={() => onDelete(agent)} title="删除">
          🗑️
        </button>
      </div>
    </div>
  );
}

// 主页面组件
export function Agents() {
  const [agents, setAgents] = useState<AgentFrontmatter[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<'create' | 'edit'>('create');
  const [selectedAgent, setSelectedAgent] = useState<AgentDetail | null>(null);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [deleteAgentName, setDeleteAgentName] = useState('');

  const fetchAgents = useCallback(async () => {
    try {
      setLoading(true);
      const response = await api.getAgents();
      // 将 Agent 转换为 AgentFrontmatter 格式
      const agentsList: AgentFrontmatter[] = response.agents.map(a => ({
        name: a.name,
        description: a.description || '',
        provider: (a as any).provider || 'unknown',
        model: (a as any).model || 'unknown',
        allowed_tools: (a as any).allowed_tools || [],
        default: (a as any).default || false,
      }));
      setAgents(agentsList);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取 Agent 列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);

  const handleCreate = async (data: CreateAgentRequest | UpdateAgentRequest) => {
    const createData = data as CreateAgentRequest;
    await api.createAgent(createData);
    await fetchAgents();
  };

  const handleEdit = async (data: CreateAgentRequest | UpdateAgentRequest) => {
    if (!selectedAgent) return;
    const updateData = data as UpdateAgentRequest;
    await api.updateAgent(selectedAgent.name, updateData);
    await fetchAgents();
  };

  const handleDelete = async () => {
    await deleteAgent(deleteAgentName);
    await fetchAgents();
  };

  const openCreateModal = () => {
    setModalMode('create');
    setSelectedAgent(null);
    setModalOpen(true);
  };

  const openEditModal = async (agent: AgentFrontmatter) => {
    try {
      const detail = await api.getAgent(agent.name);
      setSelectedAgent(detail);
      setModalMode('edit');
      setModalOpen(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取 Agent 详情失败');
    }
  };

  const openDeleteModal = (agent: AgentFrontmatter) => {
    setDeleteAgentName(agent.name);
    setDeleteModalOpen(true);
  };

  if (loading && agents.length === 0) {
    return (
      <div className="agents-page">
        <div className="agents-header">
          <h1>🤖 Agent 管理</h1>
        </div>
        <div className="loading-state">
          <div className="loading-spinner"></div>
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error && agents.length === 0) {
    return (
      <div className="agents-page">
        <div className="agents-header">
          <h1>🤖 Agent 管理</h1>
        </div>
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
        <div className="agents-header-actions">
          <span className="agents-count">共 {agents.length} 个 Agent</span>
          <button className="btn-primary" onClick={openCreateModal}>
            ➕ 创建 Agent
          </button>
        </div>
      </div>

      {error && (
        <div className="error-banner">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {agents.length === 0 ? (
        <div className="empty-state">
          <p>📭 暂无配置的 Agent</p>
          <button className="btn-primary" onClick={openCreateModal}>
            ➕ 创建第一个 Agent
          </button>
        </div>
      ) : (
        <div className="agents-grid">
          {agents.map((agent) => (
            <AgentCard
              key={agent.name}
              agent={agent}
              onEdit={openEditModal}
              onDelete={openDeleteModal}
            />
          ))}
        </div>
      )}

      <AgentModal
        isOpen={modalOpen}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === 'create' ? handleCreate : handleEdit}
        agent={selectedAgent}
        mode={modalMode}
      />

      <DeleteModal
        isOpen={deleteModalOpen}
        onClose={() => setDeleteModalOpen(false)}
        onConfirm={handleDelete}
        agentName={deleteAgentName}
      />
    </div>
  );
}
