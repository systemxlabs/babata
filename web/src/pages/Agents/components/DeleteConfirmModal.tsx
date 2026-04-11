import './DeleteConfirmModal.css';

interface DeleteConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  agentName: string;
  loading?: boolean;
}

export function DeleteConfirmModal({
  isOpen,
  onClose,
  onConfirm,
  agentName,
  loading,
}: DeleteConfirmModalProps) {
  if (!isOpen) return null;

  return (
    <div className="delete-modal-overlay" onClick={onClose}>
      <div className="delete-modal" onClick={(e) => e.stopPropagation()}>
        <div className="delete-modal-header">
          <div className="delete-icon">🗑️</div>
          <h2>确认删除</h2>
        </div>

        <div className="delete-modal-content">
          <p className="delete-message">
            确定要删除 Agent <strong>"{agentName}"</strong> 吗？
          </p>
          <p className="delete-warning">
            此操作不可撤销，删除后该 Agent 将无法使用。
          </p>
        </div>

        <div className="delete-modal-footer">
          <button
            type="button"
            className="btn-secondary"
            onClick={onClose}
            disabled={loading}
          >
            取消
          </button>
          <button
            type="button"
            className="btn-danger"
            onClick={onConfirm}
            disabled={loading}
          >
            {loading ? '⏳ 删除中...' : '🗑️ 确认删除'}
          </button>
        </div>
      </div>
    </div>
  );
}
