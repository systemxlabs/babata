import { useCallback, useEffect, useState } from 'react';
import {
  createProvider,
  deleteProvider,
  listProviders,
  updateProvider,
} from '../../api';
import type { CompatibleApi, ProviderConfig, ProviderName } from '../../types';
import './Providers.css';

type ProviderFormState = {
  name: ProviderName;
  api_key: string;
  base_url: string;
  compatible_api: CompatibleApi;
};

const PROVIDER_OPTIONS: { value: ProviderName; label: string; hint: string }[] = [
  { value: 'openai', label: 'OpenAI', hint: '官方 OpenAI 兼容配置' },
  { value: 'anthropic', label: 'Anthropic', hint: 'Claude 官方接口' },
  { value: 'deepseek', label: 'DeepSeek', hint: 'DeepSeek 官方接口' },
  { value: 'kimi', label: 'Kimi', hint: '月之暗面 Kimi 接口' },
  { value: 'moonshot', label: 'Moonshot', hint: 'Moonshot 官方接口' },
  { value: 'minimax', label: 'MiniMax', hint: 'MiniMax 官方接口' },
  { value: 'custom', label: 'Custom', hint: '自定义兼容 OpenAI/Anthropic 接口' },
];

function toFormState(provider?: ProviderConfig | null): ProviderFormState {
  if (!provider) {
    return {
      name: 'openai',
      api_key: '',
      base_url: '',
      compatible_api: 'openai',
    };
  }

  if (provider.name === 'custom') {
    return {
      name: provider.name,
      api_key: provider.api_key,
      base_url: provider.base_url,
      compatible_api: provider.compatible_api,
    };
  }

  return {
    name: provider.name,
    api_key: provider.api_key,
    base_url: '',
    compatible_api: 'openai',
  };
}

function toProviderConfig(form: ProviderFormState): ProviderConfig {
  if (form.name === 'custom') {
    return {
      name: 'custom',
      api_key: form.api_key.trim(),
      base_url: form.base_url.trim(),
      compatible_api: form.compatible_api,
    };
  }

  return {
    name: form.name,
    api_key: form.api_key.trim(),
  };
}

function maskApiKey(value: string): string {
  if (!value) return '未配置';
  if (value.length <= 8) return '••••••••';
  return `${value.slice(0, 4)}••••${value.slice(-4)}`;
}

interface ProviderModalProps {
  isOpen: boolean;
  mode: 'create' | 'edit';
  provider: ProviderConfig | null;
  onClose: () => void;
  onSubmit: (provider: ProviderConfig) => Promise<void>;
}

function ProviderModal({ isOpen, mode, provider, onClose, onSubmit }: ProviderModalProps) {
  const [formState, setFormState] = useState<ProviderFormState>(toFormState());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    setFormState(toFormState(provider));
    setError(null);
  }, [isOpen, provider]);

  if (!isOpen) return null;

  const isCustom = formState.name === 'custom';
  const currentOption = PROVIDER_OPTIONS.find((option) => option.value === formState.name);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!formState.api_key.trim()) {
      setError('API Key 不能为空');
      return;
    }
    if (isCustom && !formState.base_url.trim()) {
      setError('自定义 Provider 需要填写 Base URL');
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await onSubmit(toProviderConfig(formState));
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存 Provider 失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="providers-modal-overlay" onClick={onClose}>
      <div className="providers-modal" onClick={(event) => event.stopPropagation()}>
        <div className="providers-modal-header">
          <div>
            <h2>{mode === 'create' ? '➕ 创建 Provider' : '✏️ 编辑 Provider'}</h2>
            <p>{currentOption?.hint}</p>
          </div>
          <button className="providers-modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="providers-modal-body">
            {error && <div className="providers-error-banner">{error}</div>}

            <div className="providers-form-group">
              <label>Provider 类型</label>
              <select
                value={formState.name}
                onChange={(event) =>
                  setFormState((current) => ({
                    ...current,
                    name: event.target.value as ProviderName,
                    base_url: event.target.value === 'custom' ? current.base_url : '',
                    compatible_api: current.compatible_api ?? 'openai',
                  }))
                }
                disabled={loading || mode === 'edit'}
              >
                {PROVIDER_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>

            <div className="providers-form-group">
              <label>API Key</label>
              <input
                type="password"
                value={formState.api_key}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, api_key: event.target.value }))
                }
                placeholder="输入 Provider API Key"
                disabled={loading}
              />
            </div>

            {isCustom && (
              <>
                <div className="providers-form-group">
                  <label>Base URL</label>
                  <input
                    type="url"
                    value={formState.base_url}
                    onChange={(event) =>
                      setFormState((current) => ({ ...current, base_url: event.target.value }))
                    }
                    placeholder="https://example.com/v1"
                    disabled={loading}
                  />
                </div>

                <div className="providers-form-group">
                  <label>兼容 API</label>
                  <select
                    value={formState.compatible_api}
                    onChange={(event) =>
                      setFormState((current) => ({
                        ...current,
                        compatible_api: event.target.value as CompatibleApi,
                      }))
                    }
                    disabled={loading}
                  >
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                  </select>
                </div>
              </>
            )}
          </div>

          <div className="providers-modal-footer">
            <button
              type="button"
              className="providers-button providers-button-secondary"
              onClick={onClose}
              disabled={loading}
            >
              取消
            </button>
            <button type="submit" className="providers-button providers-button-primary" disabled={loading}>
              {loading ? '⏳ 保存中...' : mode === 'create' ? '➕ 创建' : '💾 保存'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

interface DeleteModalProps {
  isOpen: boolean;
  provider: ProviderConfig | null;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

function DeleteModal({ isOpen, provider, onClose, onConfirm }: DeleteModalProps) {
  const [loading, setLoading] = useState(false);

  if (!isOpen || !provider) return null;

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
    <div className="providers-modal-overlay" onClick={onClose}>
      <div className="providers-modal providers-modal-small" onClick={(event) => event.stopPropagation()}>
        <div className="providers-modal-header">
          <div>
            <h2>🗑️ 删除 Provider</h2>
            <p>此操作不可撤销。</p>
          </div>
          <button className="providers-modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="providers-modal-body">
          <p className="providers-delete-copy">
            确定要删除 <strong>{provider.name}</strong> Provider 吗？
          </p>
        </div>
        <div className="providers-modal-footer">
          <button
            type="button"
            className="providers-button providers-button-secondary"
            onClick={onClose}
            disabled={loading}
          >
            取消
          </button>
          <button
            type="button"
            className="providers-button providers-button-danger"
            onClick={handleConfirm}
            disabled={loading}
          >
            {loading ? '⏳ 删除中...' : '🗑️ 删除'}
          </button>
        </div>
      </div>
    </div>
  );
}

interface ProviderCardProps {
  provider: ProviderConfig;
  onEdit: (provider: ProviderConfig) => void;
  onDelete: (provider: ProviderConfig) => void;
}

function ProviderCard({ provider, onEdit, onDelete }: ProviderCardProps) {
  return (
    <article className="provider-card">
      <div className="provider-card-header">
        <div>
          <div className="provider-card-kicker">Provider</div>
          <h3>{provider.name}</h3>
        </div>
        <div className="provider-card-actions">
          <button className="provider-icon-button" onClick={() => onEdit(provider)} title="编辑">
            ✏️
          </button>
          <button
            className="provider-icon-button provider-icon-button-danger"
            onClick={() => onDelete(provider)}
            title="删除"
          >
            🗑️
          </button>
        </div>
      </div>

      <div className="provider-card-body">
        <div className="provider-field">
          <span className="provider-field-label">API Key</span>
          <span className="provider-field-value provider-field-secret">{maskApiKey(provider.api_key)}</span>
        </div>

        {provider.name === 'custom' && (
          <>
            <div className="provider-field">
              <span className="provider-field-label">Base URL</span>
              <span className="provider-field-value">{provider.base_url}</span>
            </div>
            <div className="provider-field">
              <span className="provider-field-label">兼容 API</span>
              <span className="provider-field-value">{provider.compatible_api}</span>
            </div>
          </>
        )}
      </div>
    </article>
  );
}

export function Providers() {
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalMode, setModalMode] = useState<'create' | 'edit'>('create');
  const [modalOpen, setModalOpen] = useState(false);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<ProviderConfig | null>(null);

  const fetchProviders = useCallback(async () => {
    try {
      setLoading(true);
      const providerList = await listProviders();
      setProviders(providerList);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取 Provider 列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchProviders();
  }, [fetchProviders]);

  const handleCreate = async (provider: ProviderConfig) => {
    await createProvider(provider);
    await fetchProviders();
  };

  const handleEdit = async (provider: ProviderConfig) => {
    if (!selectedProvider) return;
    await updateProvider(selectedProvider.name, provider);
    await fetchProviders();
  };

  const handleDelete = async () => {
    if (!selectedProvider) return;
    await deleteProvider(selectedProvider.name);
    await fetchProviders();
    setSelectedProvider(null);
  };

  const openCreateModal = () => {
    setModalMode('create');
    setSelectedProvider(null);
    setModalOpen(true);
  };

  const openEditModal = (provider: ProviderConfig) => {
    setModalMode('edit');
    setSelectedProvider(provider);
    setModalOpen(true);
  };

  const openDeleteModal = (provider: ProviderConfig) => {
    setSelectedProvider(provider);
    setDeleteModalOpen(true);
  };

  if (loading && providers.length === 0) {
    return (
      <div className="providers-page">
        <div className="providers-header">
          <h1>🔌 Provider 管理</h1>
        </div>
        <div className="loading-state">
          <div className="loading-spinner"></div>
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error && providers.length === 0) {
    return (
      <div className="providers-page">
        <div className="providers-header">
          <h1>🔌 Provider 管理</h1>
        </div>
        <div className="error-state">
          <p>❌ {error}</p>
          <button onClick={() => void fetchProviders()}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="providers-page">
      <div className="providers-header">
        <div>
          <h1>🔌 Provider 管理</h1>
          <p>管理模型供应商凭据和自定义兼容 API 接口。</p>
        </div>
        <div className="providers-header-actions">
          <span className="providers-count">共 {providers.length} 个 Provider</span>
          <button className="providers-button providers-button-primary" onClick={openCreateModal}>
            ➕ 创建 Provider
          </button>
        </div>
      </div>

      {error && (
        <div className="providers-error-banner providers-error-banner-inline">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {providers.length === 0 ? (
        <div className="providers-empty-state">
          <div className="providers-empty-icon">🔌</div>
          <h2>还没有 Provider</h2>
          <p>先配置 Provider，再去 Agent 页面绑定模型和提示词。</p>
          <button className="providers-button providers-button-primary" onClick={openCreateModal}>
            ➕ 创建第一个 Provider
          </button>
        </div>
      ) : (
        <div className="providers-grid">
          {providers.map((provider) => (
            <ProviderCard
              key={provider.name}
              provider={provider}
              onEdit={openEditModal}
              onDelete={openDeleteModal}
            />
          ))}
        </div>
      )}

      <ProviderModal
        isOpen={modalOpen}
        mode={modalMode}
        provider={selectedProvider}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === 'create' ? handleCreate : handleEdit}
      />

      <DeleteModal
        isOpen={deleteModalOpen}
        provider={selectedProvider}
        onClose={() => setDeleteModalOpen(false)}
        onConfirm={handleDelete}
      />
    </div>
  );
}
