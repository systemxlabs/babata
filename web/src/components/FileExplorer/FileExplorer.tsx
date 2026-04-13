import { useCallback, useEffect, useMemo, useState } from 'react';
import type { FileEntry } from '../../types';
import './FileExplorer.css';

interface FileExplorerProps {
  files: FileEntry[];
  loadFileContent: (path: string) => Promise<string>;
  treeTitle?: string;
  emptyMessage?: string;
  placeholderMessage?: string;
  defaultSelectedPath?: string;
}

type FileTreeNode = FileEntry | FileTreeDirectory;

interface FileTreeDirectory {
  [key: string]: FileTreeNode;
}

function isFileEntry(node: FileTreeNode): node is FileEntry {
  return 'path' in node;
}

function organizeFiles(files: FileEntry[]) {
  const root: FileTreeDirectory = {};

  files.forEach((file) => {
    const parts = file.path.split('/').filter(Boolean);
    let current = root;

    parts.forEach((part, index) => {
      if (index === parts.length - 1) {
        current[part] = file;
        return;
      }

      if (!current[part] || isFileEntry(current[part])) {
        current[part] = {};
      }

      current = current[part] as FileTreeDirectory;
    });
  });

  return root;
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const index = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, index)).toFixed(1))} ${sizes[index]}`;
}

function getFileLanguage(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  const langMap: Record<string, string> = {
    js: 'javascript',
    ts: 'typescript',
    jsx: 'jsx',
    tsx: 'tsx',
    py: 'python',
    rs: 'rust',
    java: 'java',
    go: 'go',
    html: 'html',
    css: 'css',
    json: 'json',
    md: 'markdown',
    yaml: 'yaml',
    yml: 'yaml',
    xml: 'xml',
    sql: 'sql',
    sh: 'bash',
    bash: 'bash',
  };
  return langMap[ext] || 'text';
}

export function FileExplorer({
  files,
  loadFileContent,
  treeTitle = '文件列表',
  emptyMessage = '暂无文件',
  placeholderMessage = '选择文件查看内容',
  defaultSelectedPath,
}: FileExplorerProps) {
  const [selectedPath, setSelectedPath] = useState<string | null>(defaultSelectedPath ?? null);
  const [fileContent, setFileContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fileTree = useMemo(() => organizeFiles(files), [files]);
  const selectedFile =
    selectedPath === null ? null : files.find((file) => file.path === selectedPath) ?? null;

  useEffect(() => {
    if (selectedPath && files.some((file) => file.path === selectedPath)) {
      return;
    }

    if (defaultSelectedPath && files.some((file) => file.path === defaultSelectedPath)) {
      setSelectedPath(defaultSelectedPath);
      return;
    }

    setSelectedPath(null);
    setFileContent('');
    setError(null);
  }, [defaultSelectedPath, files, selectedPath]);

  useEffect(() => {
    if (!selectedFile || selectedFile.is_dir) {
      setFileContent('');
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;

    const run = async () => {
      setLoading(true);
      setError(null);

      try {
        const content = await loadFileContent(selectedFile.path);
        if (!cancelled) {
          setFileContent(content);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : '加载文件失败');
          setFileContent('');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void run();

    return () => {
      cancelled = true;
    };
  }, [loadFileContent, selectedFile]);

  const handleFileClick = useCallback((file: FileEntry) => {
    if (file.is_dir) {
      return;
    }

    setSelectedPath(file.path);
  }, []);

  const renderFileTree = (items: FileTreeDirectory, level = 0): React.ReactNode[] => {
    const sortedKeys = Object.keys(items).sort((a, b) => {
      const aIsDir = !isFileEntry(items[a]);
      const bIsDir = !isFileEntry(items[b]);
      if (aIsDir && !bIsDir) return -1;
      if (!aIsDir && bIsDir) return 1;
      return a.localeCompare(b);
    });

    return sortedKeys.map((key) => {
      const item = items[key];

      if (isFileEntry(item)) {
        const isSelected = selectedFile?.path === item.path;

        return (
          <div
            key={item.path}
            className={`file-tree-item file ${isSelected ? 'selected' : ''}`}
            style={{ paddingLeft: `${12 + level * 16}px` }}
            onClick={() => handleFileClick(item)}
          >
            <svg
              className="file-icon"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            <span className="file-name">{item.name}</span>
            {item.size !== null && <span className="file-size">{formatFileSize(item.size)}</span>}
          </div>
        );
      }

      return (
        <div key={`${key}-${level}`}>
          <div
            className="file-tree-item directory"
            style={{ paddingLeft: `${12 + level * 16}px` }}
          >
            <svg
              className="file-icon"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
            </svg>
            <span className="file-name">{key}</span>
          </div>
          {renderFileTree(item, level + 1)}
        </div>
      );
    });
  };

  return (
    <div className="file-explorer">
      <div className="directory-layout">
        <div className="file-tree-panel">
          <div className="panel-header">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
            </svg>
            {treeTitle}
          </div>
          <div className="file-tree-content">
            {files.length === 0 ? (
              <div className="empty-files">{emptyMessage}</div>
            ) : (
              renderFileTree(fileTree)
            )}
          </div>
        </div>

        <div className="file-content-panel">
          {selectedFile ? (
            <>
              <div className="panel-header file-header">
                <span className="file-path">{selectedFile.path}</span>
                <span className="file-meta">
                  {selectedFile.size !== null ? formatFileSize(selectedFile.size) : ''}
                </span>
              </div>
              <div className="file-content">
                {loading ? (
                  <div className="content-loading">
                    <div className="loading-spinner"></div>
                    <p>加载中...</p>
                  </div>
                ) : error ? (
                  <div className="content-error">
                    <p>{error}</p>
                  </div>
                ) : (
                  <pre className={`code-block language-${getFileLanguage(selectedFile.name)}`}>
                    <code>{fileContent}</code>
                  </pre>
                )}
              </div>
            </>
          ) : (
            <div className="no-file-selected">
              <svg
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              >
                <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <p>{placeholderMessage}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
