import { useState, useEffect } from 'react';
import type { AgentDetail, CreateAgentRequest, UpdateAgentRequest } from '../../../types';
import './AgentModal.css';

interface AgentModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: CreateAgentRequest | UpdateAgentRequest) => void;
  agent?: AgentDetail | null;
  loading?: boolean;
}

const EMPTY_FORM: CreateAgentRequest = {
  name: '',
  description: '',
  provider: 'openai',
  model: 'gpt-4o',
  allowed_tools: [],
  default: false,
  body: '',
};

const COMMON_PROVIDERS = ['openai', 'anthropic', 'google', 'azure', 'ollama', 'custom'];
const COMMON_MODELS = ['gpt-4o', 'gpt-4o-mini', 'claude-3-opus', 'claude-3-sonnet', 'gemini-pro', 'custom'];

export function AgentModal({ isOpen, onClose, onSubmit, agent, loading }: AgentModalProps) {
  const isEditing = !!agent;
  const [formData, setFormData] = useState<CreateAgentRequest>(EMPTY_FORM);
  const [toolInput, setToolInput] = useState('');

  // 初始化表单数据
  useEffect(() => {
    if (isOpen) {
      if (agent) {
        setFormData({
          name: agent.name,
          description: agent.description,
          provider: agent.provider,
          model: agent.model,
          allowed_tools: [...agent.allowed_tools],
          default: agent.default,
          body: agent.body,
        });
      } else {
        setFormData(EMPTY_FORM);
      }
      setToolInput('');
    }
  }, [isOpen, agent]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (isEditing && agent) {
      const updateData: UpdateAgentRequest = {
        description: formData.description,
        provider: formData.provider,
        model: formData.model,
        allowed_tools: formData.allowed_tools,
        default: formData.default,
        body: formData.body,
      };
      onSubmit(updateData);
    } else {
      onSubmit(formData);
    }
  };

  const handleChange = (field: keyof CreateAgentRequest, value: string | boolean | string[]) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  const addTool = () => {
    const tool = toolInput.trim();
    if (tool && !formData.allowed_tools.includes(tool)) {
      handleChange('allowed_tools', [...formData.allowed_tools, tool]);
      setToolInput('');
    }
  };

  const removeTool = (toolToRemove: string) => {
    handleChange(
      'allowed_tools',
      formData.allowed_tools.filter((t) => t !== toolToRemove)
    );
  };

  const handleToolKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      addTool();
    }
  };

  return (
    <div className="agent-modal-overlay" onClick={onClose}>
      <div className="agent-modal" onClick={(e) => e.stopPropagation()}>
        <div className="agent-modal-header">
          <h2>{isEditing ? '✏️ 编辑 Agent' : '➕ 创建 Agent'}</h2>
          <button className="agent-modal-close" onClick={onClose} disabled={loading}>
            ✕
          </button>
        </div>

        <form className="agent-modal-form" onSubmit={handleSubmit}>
          <div className="agent-modal-content">
            {/* 基本信息 */}
            <div className="form-section">
              <h3 className="form-section-title">基本信息</h3>
              <div className="form-row">
                <div className="form-group">
                  <label htmlFor="agent-name">名称 *</label>
                  <input
                    id="agent-name"
                    type="text"
                    value={formData.name}
                    onChange={(e) => handleChange('name', e.target.value)}
                    placeholder="输入 Agent 名称"
                    disabled={isEditing || loading}
                    required
                  />
                  {isEditing && (
                    <span className="form-hint">Agent 名称不可修改</span>
                  )}
                </div>
                <div className="form-group checkbox-group">
                  <label className="checkbox-label">
                    <input
                      type="checkbox"
                      checked={formData.default}
                      onChange={(e) => handleChange('default', e.target.checked)}
                      disabled={loading}
                    />
                    <span>设为默认 Agent</span>
                  </label>
                </div>
              </div>

              <div className="form-group">
                <label htmlFor="agent-description">描述</label>
                <input
                  id="agent-description"
                  type="text"
                  value={formData.description}
                  onChange={(e) => handleChange('description', e.target.value)}
                  placeholder="输入 Agent 描述"
                  disabled={loading}
                />
              </div>
            </div>

            {/* 模型配置 */}
            <div className="form-section">
              <h3 className="form-section-title">模型配置</h3>
              <div className="form-row">
                <div className="form-group">
                  <label htmlFor="agent-provider">Provider</label>
                  <select
                    id="agent-provider"
                    value={formData.provider}
                    onChange={(e) => handleChange('provider', e.target.value)}
                    disabled={loading}
                  >
                    {COMMON_PROVIDERS.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="form-group">
                  <label htmlFor="agent-model">Model</label>
                  <select
                    id="agent-model"
                    value={formData.model}
                    onChange={(e) => handleChange('model', e.target.value)}
                    disabled={loading}
                  >
                    {COMMON_MODELS.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </div>

            {/* 允许的工具 */}
            <div className="form-section">
              <h3 className="form-section-title">允许的工具</h3>
              <div className="tools-input-group">
                <input
                  type="text"
                  value={toolInput}
                  onChange={(e) => setToolInput(e.target.value)}
                  onKeyDown={handleToolKeyDown}
                  placeholder="输入工具名称，按回车添加"
                  disabled={loading}
                />
                <button
                  type="button"
                  className="btn-add-tool"
                  onClick={addTool}
                  disabled={!toolInput.trim() || loading}
                >
                  添加
                </button>
              </div>
              <div className="tools-list">
                {formData.allowed_tools.length === 0 ? (
                  <span className="tools-empty">暂无工具</span>
                ) : (
                  formData.allowed_tools.map((tool) => (
                    <span key={tool} className="tool-tag">
                      {tool}
                      <button
                        type="button"
                        className="tool-remove"
                        onClick={() => removeTool(tool)}
                        disabled={loading}
                      >
                        ✕
                      </button>
                    </span>
                  ))
                )}
              </div>
            </div>

            {/* Body */}
            <div className="form-section">
              <h3 className="form-section-title">Agent Body (Prompt)</h3>
              <div className="form-group">
                <textarea
                  value={formData.body}
                  onChange={(e) => handleChange('body', e.target.value)}
                  placeholder="输入 Agent 的 Prompt/Instructions..."
                  rows={8}
                  disabled={loading}
                  className="body-textarea"
                />
              </div>
            </div>
          </div>

          <div className="agent-modal-footer">
            <button
              type="button"
              className="btn-secondary"
              onClick={onClose}
              disabled={loading}
            >
              取消
            </button>
            <button
              type="submit"
              className="btn-primary"
              disabled={!formData.name.trim() || loading}
            >
              {loading ? '⏳ 保存中...' : isEditing ? '💾 保存' : '➕ 创建'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
