import { useCallback, useEffect, useState } from 'react';
import {
  createChannel,
  deleteChannel,
  listChannels,
  updateChannel,
} from '../../api';
import type { ChannelConfig, ChannelName } from '../../types';
import './Channels.css';

type ChannelFormState = {
  name: ChannelName;
  bot_token: string;
  user_id: string;
};

const CHANNEL_OPTIONS: { value: ChannelName; label: string; hint: string }[] = [
  { value: 'telegram', label: 'Telegram', hint: '通过 Telegram Bot 接收用户消息' },
  { value: 'wechat', label: 'Wechat', hint: '通过 Wechat 渠道接收用户消息' },
];

function toFormState(channel?: ChannelConfig | null): ChannelFormState {
  if (!channel) {
    return {
      name: 'telegram',
      bot_token: '',
      user_id: '',
    };
  }

  return {
    name: channel.name,
    bot_token: channel.bot_token,
    user_id: String(channel.user_id),
  };
}

function toChannelConfig(form: ChannelFormState): ChannelConfig {
  if (form.name === 'telegram') {
    return {
      name: 'telegram',
      bot_token: form.bot_token.trim(),
      user_id: Number(form.user_id),
    };
  }

  return {
    name: 'wechat',
    bot_token: form.bot_token.trim(),
    user_id: form.user_id.trim(),
  };
}

function maskSecret(value: string): string {
  if (!value) return '未配置';
  if (value.length <= 8) return '••••••••';
  return `${value.slice(0, 4)}••••${value.slice(-4)}`;
}

interface ChannelModalProps {
  isOpen: boolean;
  mode: 'create' | 'edit';
  channel: ChannelConfig | null;
  onClose: () => void;
  onSubmit: (channel: ChannelConfig) => Promise<void>;
}

function ChannelModal({ isOpen, mode, channel, onClose, onSubmit }: ChannelModalProps) {
  const [formState, setFormState] = useState<ChannelFormState>(toFormState());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    setFormState(toFormState(channel));
    setError(null);
  }, [channel, isOpen]);

  if (!isOpen) return null;

  const currentOption = CHANNEL_OPTIONS.find((option) => option.value === formState.name);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!formState.bot_token.trim()) {
      setError('Bot Token 不能为空');
      return;
    }
    if (!formState.user_id.trim()) {
      setError('User ID 不能为空');
      return;
    }
    if (formState.name === 'telegram' && (!/^\d+$/.test(formState.user_id) || Number(formState.user_id) <= 0)) {
      setError('Telegram User ID 必须是正整数');
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await onSubmit(toChannelConfig(formState));
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存 Channel 失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="channels-modal-overlay" onClick={onClose}>
      <div className="channels-modal" onClick={(event) => event.stopPropagation()}>
        <div className="channels-modal-header">
          <div>
            <h2>{mode === 'create' ? '➕ 创建 Channel' : '✏️ 编辑 Channel'}</h2>
            <p>{currentOption?.hint}</p>
          </div>
          <button className="channels-modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="channels-modal-body">
            {error && <div className="channels-error-banner">{error}</div>}

            <div className="channels-form-group">
              <label>Channel 类型</label>
              <select
                value={formState.name}
                onChange={(event) =>
                  setFormState((current) => ({
                    ...current,
                    name: event.target.value as ChannelName,
                    user_id: '',
                  }))
                }
                disabled={loading || mode === 'edit'}
              >
                {CHANNEL_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>

            <div className="channels-form-group">
              <label>Bot Token</label>
              <input
                type="password"
                value={formState.bot_token}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, bot_token: event.target.value }))
                }
                placeholder="输入 Bot Token"
                disabled={loading}
              />
            </div>

            <div className="channels-form-group">
              <label>{formState.name === 'telegram' ? 'Telegram User ID' : 'Wechat User ID'}</label>
              <input
                type={formState.name === 'telegram' ? 'number' : 'text'}
                value={formState.user_id}
                onChange={(event) =>
                  setFormState((current) => ({ ...current, user_id: event.target.value }))
                }
                placeholder={formState.name === 'telegram' ? '例如 123456789' : '例如 wxid_xxx'}
                disabled={loading}
              />
            </div>
          </div>

          <div className="channels-modal-footer">
            <button
              type="button"
              className="channels-button channels-button-secondary"
              onClick={onClose}
              disabled={loading}
            >
              取消
            </button>
            <button type="submit" className="channels-button channels-button-primary" disabled={loading}>
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
  channel: ChannelConfig | null;
  onClose: () => void;
  onConfirm: () => Promise<void>;
}

function DeleteModal({ isOpen, channel, onClose, onConfirm }: DeleteModalProps) {
  const [loading, setLoading] = useState(false);

  if (!isOpen || !channel) return null;

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
    <div className="channels-modal-overlay" onClick={onClose}>
      <div className="channels-modal channels-modal-small" onClick={(event) => event.stopPropagation()}>
        <div className="channels-modal-header">
          <div>
            <h2>🗑️ 删除 Channel</h2>
            <p>此操作不可撤销。</p>
          </div>
          <button className="channels-modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="channels-modal-body">
          <p className="channels-delete-copy">
            确定要删除 <strong>{channel.name}</strong> Channel 吗？
          </p>
        </div>
        <div className="channels-modal-footer">
          <button
            type="button"
            className="channels-button channels-button-secondary"
            onClick={onClose}
            disabled={loading}
          >
            取消
          </button>
          <button
            type="button"
            className="channels-button channels-button-danger"
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

interface ChannelCardProps {
  channel: ChannelConfig;
  onEdit: (channel: ChannelConfig) => void;
  onDelete: (channel: ChannelConfig) => void;
}

function ChannelCard({ channel, onEdit, onDelete }: ChannelCardProps) {
  return (
    <article className="channel-card">
      <div className="channel-card-header">
        <div>
          <div className="channel-card-kicker">Channel</div>
          <h3>{channel.name}</h3>
        </div>
        <div className="channel-card-actions">
          <button className="channel-icon-button" onClick={() => onEdit(channel)} title="编辑">
            ✏️
          </button>
          <button
            className="channel-icon-button channel-icon-button-danger"
            onClick={() => onDelete(channel)}
            title="删除"
          >
            🗑️
          </button>
        </div>
      </div>

      <div className="channel-card-body">
        <div className="channel-field">
          <span className="channel-field-label">Bot Token</span>
          <span className="channel-field-value channel-field-secret">
            {maskSecret(channel.bot_token)}
          </span>
        </div>
        <div className="channel-field">
          <span className="channel-field-label">User ID</span>
          <span className="channel-field-value">{String(channel.user_id)}</span>
        </div>
      </div>
    </article>
  );
}

export function Channels() {
  const [channels, setChannels] = useState<ChannelConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalMode, setModalMode] = useState<'create' | 'edit'>('create');
  const [modalOpen, setModalOpen] = useState(false);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [selectedChannel, setSelectedChannel] = useState<ChannelConfig | null>(null);

  const fetchChannels = useCallback(async () => {
    try {
      setLoading(true);
      const channelList = await listChannels();
      setChannels(channelList);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '获取 Channel 列表失败');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchChannels();
  }, [fetchChannels]);

  const handleCreate = async (channel: ChannelConfig) => {
    await createChannel(channel);
    await fetchChannels();
  };

  const handleEdit = async (channel: ChannelConfig) => {
    if (!selectedChannel) return;
    await updateChannel(selectedChannel.name, channel);
    await fetchChannels();
  };

  const handleDelete = async () => {
    if (!selectedChannel) return;
    await deleteChannel(selectedChannel.name);
    await fetchChannels();
    setSelectedChannel(null);
  };

  const openCreateModal = () => {
    setModalMode('create');
    setSelectedChannel(null);
    setModalOpen(true);
  };

  const openEditModal = (channel: ChannelConfig) => {
    setModalMode('edit');
    setSelectedChannel(channel);
    setModalOpen(true);
  };

  const openDeleteModal = (channel: ChannelConfig) => {
    setSelectedChannel(channel);
    setDeleteModalOpen(true);
  };

  if (loading && channels.length === 0) {
    return (
      <div className="channels-page">
        <div className="channels-header">
          <h1>📡 Channel 管理</h1>
        </div>
        <div className="loading-state">
          <div className="loading-spinner"></div>
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error && channels.length === 0) {
    return (
      <div className="channels-page">
        <div className="channels-header">
          <h1>📡 Channel 管理</h1>
        </div>
        <div className="error-state">
          <p>❌ {error}</p>
          <button onClick={() => void fetchChannels()}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="channels-page">
      <div className="channels-header">
        <div>
          <h1>📡 Channel 管理</h1>
          <p>管理消息接入渠道，用于从 Telegram 或 Wechat 接收用户输入。</p>
        </div>
        <div className="channels-header-actions">
          <span className="channels-count">共 {channels.length} 个 Channel</span>
          <button className="channels-button channels-button-primary" onClick={openCreateModal}>
            ➕ 创建 Channel
          </button>
        </div>
      </div>

      {error && (
        <div className="channels-error-banner channels-error-banner-inline">
          <span>❌ {error}</span>
          <button onClick={() => setError(null)}>✕</button>
        </div>
      )}

      {channels.length === 0 ? (
        <div className="channels-empty-state">
          <div className="channels-empty-icon">📡</div>
          <h2>还没有 Channel</h2>
          <p>配置 Channel 后，系统才能从外部消息入口接收任务。</p>
          <button className="channels-button channels-button-primary" onClick={openCreateModal}>
            ➕ 创建第一个 Channel
          </button>
        </div>
      ) : (
        <div className="channels-grid">
          {channels.map((channel) => (
            <ChannelCard
              key={channel.name}
              channel={channel}
              onEdit={openEditModal}
              onDelete={openDeleteModal}
            />
          ))}
        </div>
      )}

      <ChannelModal
        isOpen={modalOpen}
        mode={modalMode}
        channel={selectedChannel}
        onClose={() => setModalOpen(false)}
        onSubmit={modalMode === 'create' ? handleCreate : handleEdit}
      />

      <DeleteModal
        isOpen={deleteModalOpen}
        channel={selectedChannel}
        onClose={() => setDeleteModalOpen(false)}
        onConfirm={handleDelete}
      />
    </div>
  );
}
