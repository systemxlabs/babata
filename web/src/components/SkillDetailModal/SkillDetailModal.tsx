import { useCallback, useEffect, useState } from 'react';
import { getSkillFile, getSkillFiles } from '../../api';
import { FileExplorer } from '../FileExplorer/FileExplorer';
import type { FileEntry, Skill } from '../../types';
import './SkillDetailModal.css';

interface SkillDetailModalProps {
  skill: Skill | null;
  isOpen: boolean;
  onClose: () => void;
}

export function SkillDetailModal({ skill, isOpen, onClose }: SkillDetailModalProps) {
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchSkillFiles = useCallback(async () => {
    if (!skill) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await getSkillFiles(skill.name);
      setFiles(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载技能详情失败');
      setFiles([]);
    } finally {
      setLoading(false);
    }
  }, [skill]);

  const loadSkillFile = useCallback(async (path: string) => {
    if (!skill) {
      throw new Error('技能不存在');
    }

    return getSkillFile(skill.name, path);
  }, [skill]);

  useEffect(() => {
    if (!isOpen || !skill) {
      return;
    }

    void fetchSkillFiles();
  }, [fetchSkillFiles, isOpen, skill]);

  if (!isOpen || !skill) {
    return null;
  }

  return (
    <div className="skill-detail-modal-overlay" onClick={onClose}>
      <div className="skill-detail-modal" onClick={(event) => event.stopPropagation()}>
        <div className="skill-detail-header">
          <div className="skill-detail-title">
            <h2>{skill.name}</h2>
            <p className="skill-detail-subtitle">技能详情与目录内容</p>
          </div>
          <button className="skill-detail-close" onClick={onClose} aria-label="关闭技能详情">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>

        <div className="skill-detail-body">
          <div className="skill-summary-card">
            <p className="skill-summary-label">描述</p>
            <p className="skill-summary-text">{skill.description || '暂无描述'}</p>
          </div>

          {loading ? (
            <div className="skill-detail-loading">
              <div className="loading-spinner"></div>
              <p>加载中...</p>
            </div>
          ) : error ? (
            <div className="skill-detail-error">
              <p>{error}</p>
              <button className="skill-detail-retry" onClick={fetchSkillFiles}>重试</button>
            </div>
          ) : (
            <FileExplorer
              files={files}
              loadFileContent={loadSkillFile}
              treeTitle="技能文件"
              emptyMessage="暂无技能文件"
              placeholderMessage="选择技能文件查看内容"
              defaultSelectedPath="SKILL.md"
            />
          )}
        </div>
      </div>
    </div>
  );
}
