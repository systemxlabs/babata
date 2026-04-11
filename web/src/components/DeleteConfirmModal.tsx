import { useState, useCallback, useEffect } from 'react';
import './DeleteConfirmModal.css';

interface DeleteConfirmModalProps {
  isOpen: boolean;
  taskId: string;
  taskDescription?: string;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
}

export function DeleteConfirmModal({
  isOpen,
  taskId,
  taskDescription,
  onConfirm,
  onCancel,
}: DeleteConfirmModalProps) {
  const [isDeleting, setIsDeleting] = useState(false);
  const [isClosing, setIsClosing] = useState(false);

  // 根据 isOpen 和 isClosing 计算可见性
  const isVisible = isOpen || isClosing;

  const handleClose = useCallback(() => {
    if (isDeleting) return; // 删除中不允许关闭
    setIsClosing(true);
    setTimeout(() => {
      setIsClosing(false);
      onCancel();
    }, 200);
  }, [isDeleting, onCancel]);

  // 处理背景滚动
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
    } else if (!isClosing) {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen, isClosing]);

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget && !isDeleting) {
      handleClose();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape' && !isDeleting) {
      handleClose();
    }
  };

  const handleConfirm = async () => {
    setIsDeleting(true);
    try {
      await onConfirm();
      handleClose();
    } catch {
      // 错误由父组件处理
      setIsDeleting(false);
    }
  };

  if (!isVisible) {
    return null;
  }

  return (
    <div
      className={`delete-modal-overlay ${isClosing ? 'closing' : ''}`}
      onClick={handleOverlayClick}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-modal-title"
      aria-describedby="delete-modal-desc"
    >
      <div
        className={`delete-modal-content ${isClosing ? 'closing' : ''}`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* 警告图标 */}
        <div className="delete-modal-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>

        {/* 标题 */}
        <h3 id="delete-modal-title" className="delete-modal-title">
          确认删除任务？
        </h3>

        {/* 描述 */}
        <div id="delete-modal-desc" className="delete-modal-desc">
          <p>此操作无法撤销，任务将被永久删除。</p>
          {taskDescription && (
            <div className="delete-task-info">
              <span className="delete-task-label">任务描述：</span>
              <span className="delete-task-value" title={taskDescription}>
                {taskDescription.length > 50
                  ? `${taskDescription.substring(0, 50)}...`
                  : taskDescription}
              </span>
            </div>
          )}
          <div className="delete-task-info">
            <span className="delete-task-label">任务 ID：</span>
            <code className="delete-task-id">{taskId}</code>
          </div>
        </div>

        {/* 按钮组 */}
        <div className="delete-modal-actions">
          <button
            className="delete-btn-cancel"
            onClick={handleClose}
            disabled={isDeleting}
            type="button"
          >
            取消
          </button>
          <button
            className="delete-btn-confirm"
            onClick={handleConfirm}
            disabled={isDeleting}
            type="button"
          >
            {isDeleting ? (
              <>
                <span className="spinner-small"></span>
                <span>删除中...</span>
              </>
            ) : (
              '确认删除'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
