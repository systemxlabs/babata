import { useState, useEffect, useCallback } from 'react';
import type { FileEntry } from '../types';
import { getTaskFiles, getTaskFile } from '../api';
import './TaskFiles.css';

interface TaskFilesProps {
  taskId: string;
}

interface FileNode extends FileEntry {
  children?: FileNode[];
  isExpanded?: boolean;
}

export function TaskFiles({ taskId }: TaskFilesProps) {
  const [files, setFiles] = useState<FileNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPath, setCurrentPath] = useState<string>('');
  const [previewFile, setPreviewFile] = useState<{ path: string; content: string } | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const isTextFile = (filename: string): boolean => {
    const textExtensions = ['.md', '.txt', '.log', '.rs', '.ts', '.tsx', '.js', '.jsx', '.json', '.yaml', '.yml', '.css', '.html', '.xml', '.sh', '.py', '.java', '.c', '.cpp', '.h', '.go'];
    const ext = filename.toLowerCase().substring(filename.lastIndexOf('.'));
    return textExtensions.includes(ext);
  };

  const buildFileTree = (entries: FileEntry[]): FileNode[] => {
    const root: FileNode[] = [];
    const map = new Map<string, FileNode>();

    // 首先创建所有节点的映射
    entries.forEach((entry) => {
      map.set(entry.path, { ...entry, children: entry.is_dir ? [] : undefined });
    });

    // 构建树结构
    entries.forEach((entry) => {
      const node = map.get(entry.path)!;
      if (entry.path.includes('/')) {
        const parentPath = entry.path.substring(0, entry.path.lastIndexOf('/'));
        const parent = map.get(parentPath);
        if (parent && parent.children) {
          parent.children.push(node);
        }
      } else {
        root.push(node);
      }
    });

    // 按目录在前、文件在后排序
    const sortNodes = (nodes: FileNode[]) => {
      nodes.sort((a, b) => {
        if (a.is_dir && !b.is_dir) return -1;
        if (!a.is_dir && b.is_dir) return 1;
        return a.name.localeCompare(b.name);
      });
      nodes.forEach((node) => {
        if (node.children) sortNodes(node.children);
      });
    };
    sortNodes(root);

    return root;
  };

  const fetchFiles = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getTaskFiles(taskId);
      const tree = buildFileTree(data);
      setFiles(tree);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load files');
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  useEffect(() => {
    fetchFiles();
  }, [fetchFiles]);

  const toggleFolder = (_node: FileNode, path: string) => {
    const toggleInTree = (nodes: FileNode[]): boolean => {
      for (const n of nodes) {
        if (n.path === path) {
          n.isExpanded = !n.isExpanded;
          return true;
        }
        if (n.children && toggleInTree(n.children)) {
          return true;
        }
      }
      return false;
    };

    const newFiles = [...files];
    toggleInTree(newFiles);
    setFiles(newFiles);
  };

  const handleFileClick = async (node: FileNode) => {
    if (node.is_dir) {
      toggleFolder(node, node.path);
    } else if (isTextFile(node.name)) {
      setPreviewLoading(true);
      try {
        const content = await getTaskFile(taskId, node.path);
        setPreviewFile({ path: node.path, content });
      } catch (err) {
        alert('无法加载文件内容: ' + (err instanceof Error ? err.message : '未知错误'));
      } finally {
        setPreviewLoading(false);
      }
    } else {
      // 非文本文件，提供下载链接
      const downloadUrl = `/api/tasks/${taskId}/files/${node.path}`;
      window.open(downloadUrl, '_blank');
    }
  };

  const navigateTo = (path: string) => {
    setCurrentPath(path);
  };

  const getBreadcrumbPaths = () => {
    if (!currentPath) return [{ name: '根目录', path: '' }];
    const parts = currentPath.split('/');
    const paths: { name: string; path: string }[] = [{ name: '根目录', path: '' }];
    let accumulated = '';
    parts.forEach((part, index) => {
      accumulated = index === 0 ? part : `${accumulated}/${part}`;
      paths.push({ name: part, path: accumulated });
    });
    return paths;
  };

  const renderFileIcon = (node: FileNode) => {
    if (node.is_dir) {
      return node.isExpanded ? '📂' : '📁';
    }
    // 根据文件类型返回不同图标
    const ext = node.name.toLowerCase().substring(node.name.lastIndexOf('.') + 1);
    const iconMap: Record<string, string> = {
      'md': '📝',
      'txt': '📄',
      'log': '📋',
      'rs': '⚙️',
      'ts': '📘',
      'tsx': '⚛️',
      'js': '📒',
      'jsx': '⚛️',
      'json': '📋',
      'css': '🎨',
      'html': '🌐',
      'py': '🐍',
      'java': '☕',
      'go': '🔵',
    };
    return iconMap[ext] || '📄';
  };

  const renderFileTree = (nodes: FileNode[], level: number = 0) => {
    return nodes.map((node) => (
      <div key={node.path} className="file-tree-item">
        <div
          className={`file-item ${node.is_dir ? 'folder' : 'file'}`}
          style={{ paddingLeft: `${level * 20 + 12}px` }}
          onClick={() => handleFileClick(node)}
        >
          <span className="file-icon">{renderFileIcon(node)}</span>
          <span className="file-name" title={node.name}>
            {node.name}
            {!node.is_dir && node.size !== null && (
              <span className="file-size">({formatFileSize(node.size)})</span>
            )}
          </span>
          {!node.is_dir && isTextFile(node.name) && (
            <span className="file-preview-hint">点击预览</span>
          )}
        </div>
        {node.is_dir && node.isExpanded && node.children && (
          <div className="file-children">
            {renderFileTree(node.children, level + 1)}
          </div>
        )}
      </div>
    ));
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  if (loading) {
    return (
      <div className="task-files">
        <div className="files-loading">加载文件列表...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="task-files">
        <div className="files-error">
          <p>加载失败</p>
          <p className="error-message">{error}</p>
          <button onClick={fetchFiles} className="retry-btn">重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="task-files">
      {/* 面包屑导航 */}
      <div className="breadcrumb">
        {getBreadcrumbPaths().map((item, index, arr) => (
          <span key={item.path}>
            <button
              className="breadcrumb-item"
              onClick={() => navigateTo(item.path)}
            >
              {item.name}
            </button>
            {index < arr.length - 1 && (
              <span className="breadcrumb-separator">/</span>
            )}
          </span>
        ))}
      </div>

      {/* 文件列表 */}
      <div className="files-list">
        {files.length === 0 ? (
          <div className="files-empty">
            <p>📂 目录为空</p>
          </div>
        ) : (
          renderFileTree(files)
        )}
      </div>

      {/* 文件预览弹窗 */}
      {previewFile && (
        <div className="file-preview-modal" onClick={() => setPreviewFile(null)}>
          <div className="file-preview-content" onClick={(e) => e.stopPropagation()}>
            <div className="file-preview-header">
              <h4>{previewFile.path}</h4>
              <button
                className="close-preview-btn"
                onClick={() => setPreviewFile(null)}
                aria-label="关闭预览"
              >
                ×
              </button>
            </div>
            <div className="file-preview-body">
              {previewLoading ? (
                <div className="preview-loading">加载中...</div>
              ) : (
                <pre className="file-preview-code">
                  <code>{previewFile.content}</code>
                </pre>
              )}
            </div>
          </div>
        </div>
      )}

      {previewLoading && !previewFile && (
        <div className="preview-loading-overlay">
          <div className="preview-loading-spinner">加载中...</div>
        </div>
      )}
    </div>
  );
}
