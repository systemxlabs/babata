import { useState, useCallback } from 'react';
import type { FileEntry } from '../../../../types';
import { getTaskFile } from '../../../../api';
import './TaskDirectoryTab.css';

interface TaskDirectoryTabProps {
  taskId: string;
  files: FileEntry[];
}

type FileTreeNode = FileEntry | FileTreeDirectory;

interface FileTreeDirectory {
  [key: string]: FileTreeNode;
}

function isFileEntry(node: FileTreeNode): node is FileEntry {
  return 'path' in node;
}

export function TaskDirectoryTab({ taskId, files }: TaskDirectoryTabProps) {
  const [selectedFile, setSelectedFile] = useState<FileEntry | null>(null);
  const [fileContent, setFileContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 按目录组织文件
  const organizeFiles = (files: FileEntry[]) => {
    const root: FileTreeDirectory = {};
    
    files.forEach(file => {
      const parts = file.path.split('/').filter(p => p);
      let current = root;
      
      parts.forEach((part, index) => {
        if (index === parts.length - 1) {
          current[part] = file;
        } else {
          if (!current[part] || isFileEntry(current[part])) {
            current[part] = {};
          }
          current = current[part] as FileTreeDirectory;
        }
      });
    });
    
    return root;
  };

  const handleFileClick = useCallback(async (file: FileEntry) => {
    if (file.is_dir) return;
    
    setSelectedFile(file);
    setLoading(true);
    setError(null);
    
    try {
      const content = await getTaskFile(taskId, file.path);
      setFileContent(content);
    } catch (err) {
      setError(err instanceof Error ? err.message : '加载文件失败');
      setFileContent('');
    } finally {
      setLoading(false);
    }
  }, [taskId]);

  // 渲染文件树
  const renderFileTree = (items: FileTreeDirectory, level = 0) => {
    const sortedKeys = Object.keys(items).sort((a, b) => {
      const aIsDir = !isFileEntry(items[a]);
      const bIsDir = !isFileEntry(items[b]);
      if (aIsDir && !bIsDir) return -1;
      if (!aIsDir && bIsDir) return 1;
      return a.localeCompare(b);
    });

    return sortedKeys.map(key => {
      const item = items[key];
      const isFile = isFileEntry(item);
      const file = isFile ? item : null;
      const directory = isFile ? null : item;
      const isSelected = file && selectedFile?.path === file.path;

      if (isFile && file) {
        return (
          <div
            key={file.path}
            className={`file-tree-item file ${isSelected ? 'selected' : ''}`}
            style={{ paddingLeft: `${12 + level * 16}px` }}
            onClick={() => handleFileClick(file)}
          >
            <svg className="file-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
            <span className="file-name">{file.name}</span>
            {file.size !== null && (
              <span className="file-size">{formatFileSize(file.size)}</span>
            )}
          </div>
        );
      } else if (directory) {
        return (
          <div key={key}>
            <div
              className="file-tree-item directory"
              style={{ paddingLeft: `${12 + level * 16}px` }}
            >
              <svg className="file-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
              </svg>
              <span className="file-name">{key}</span>
            </div>
            {renderFileTree(directory, level + 1)}
          </div>
        );
      }

      return null;
    });
  };

  // 格式化文件大小
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  // 获取文件语言用于代码高亮
  const getFileLanguage = (filename: string): string => {
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    const langMap: { [key: string]: string } = {
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
  };

  const fileTree = organizeFiles(files);

  return (
    <div className="task-directory-tab">
      <div className="directory-layout">
        {/* 文件树 */}
        <div className="file-tree-panel">
          <div className="panel-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
            </svg>
            文件列表
          </div>
          <div className="file-tree-content">
            {files.length === 0 ? (
              <div className="empty-files">
                <p>暂无文件</p>
              </div>
            ) : (
              renderFileTree(fileTree)
            )}
          </div>
        </div>

        {/* 文件内容 */}
        <div className="file-content-panel">
          {selectedFile ? (
            <>
              <div className="panel-header file-header">
                <span className="file-path">{selectedFile.path}</span>
                <span className="file-meta">
                  {selectedFile.size !== null && formatFileSize(selectedFile.size)}
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
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <p>选择文件查看内容</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
