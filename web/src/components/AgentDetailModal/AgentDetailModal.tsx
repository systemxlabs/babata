import { useCallback, useEffect, useState } from 'react';
import { getAgent, getAgentFile, getAgentFiles } from '../../api';
import type { AgentDetail, FileEntry } from '../../types';
import { FileExplorer } from '../FileExplorer/FileExplorer';
import './AgentDetailModal.css';

interface AgentDetailModalProps {
  agentName: string | null;
  isOpen: boolean;
  onClose: () => void;
}

export function AgentDetailModal({ agentName, isOpen, onClose }: AgentDetailModalProps) {
  const [agent, setAgent] = useState<AgentDetail | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchAgentDetail = useCallback(async () => {
    if (!agentName) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const [agentResponse, filesResponse] = await Promise.all([
        getAgent(agentName),
        getAgentFiles(agentName),
      ]);

      if (!agentResponse) {
        throw new Error(`Agent "${agentName}" 不存在`);
      }

      setAgent(agentResponse);
      setFiles(filesResponse);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载 Agent 详情失败');
      setAgent(null);
      setFiles([]);
    } finally {
      setLoading(false);
    }
  }, [agentName]);

  const loadAgentFile = useCallback(async (path: string) => {
    if (!agentName) {
      throw new Error('Agent 不存在');
    }

    return getAgentFile(agentName, path);
  }, [agentName]);

  useEffect(() => {
    if (!isOpen || !agentName) {
      return;
    }

    void fetchAgentDetail();
  }, [agentName, fetchAgentDetail, isOpen]);

  if (!isOpen || !agentName) {
    return null;
  }

  return (
    <div className="agent-detail-modal-overlay" onClick={onClose}>
      <div className="agent-detail-modal" onClick={(event) => event.stopPropagation()}>
        <div className="agent-detail-header">
          <div className="agent-detail-title">
            <h2>{agentName}</h2>
            <p className="agent-detail-subtitle">Agent 详情与目录内容</p>
          </div>
          <button className="agent-detail-close" onClick={onClose} aria-label="关闭 Agent 详情">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>

        <div className="agent-detail-body">
          {loading ? (
            <div className="agent-detail-loading">
              <div className="loading-spinner"></div>
              <p>加载中...</p>
            </div>
          ) : error ? (
            <div className="agent-detail-error">
              <p>{error}</p>
              <button className="agent-detail-retry" onClick={fetchAgentDetail}>重试</button>
            </div>
          ) : agent ? (
            <>
              <div className="agent-summary-grid">
                <div className="agent-summary-card">
                  <p className="agent-summary-label">描述</p>
                  <p className="agent-summary-text">{agent.description || '暂无描述'}</p>
                </div>
                <div className="agent-summary-card">
                  <p className="agent-summary-label">模型</p>
                  <p className="agent-summary-text">{agent.provider} / {agent.model}</p>
                </div>
                <div className="agent-summary-card full-width">
                  <p className="agent-summary-label">属性</p>
                  <div className="agent-summary-tags">
                    <span className={`agent-summary-tag ${agent.default ? 'default' : ''}`}>
                      {agent.default ? '默认 Agent' : '普通 Agent'}
                    </span>
                    {agent.allowed_tools.length === 0 ? (
                      <span className="agent-summary-tag">无工具限制配置</span>
                    ) : (
                      agent.allowed_tools.map((tool) => (
                        <span key={tool} className="agent-summary-tag">{tool}</span>
                      ))
                    )}
                  </div>
                </div>
              </div>

              <div className="agent-body-card">
                <p className="agent-summary-label">Body</p>
                <pre className="agent-body-content">{agent.body}</pre>
              </div>

              <FileExplorer
                files={files}
                loadFileContent={loadAgentFile}
                treeTitle="Agent 文件"
                emptyMessage="暂无 Agent 文件"
                placeholderMessage="选择 Agent 文件查看内容"
                defaultSelectedPath="AGENT.md"
              />
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
