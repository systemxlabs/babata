import { useCallback } from 'react';
import { getTaskFile, getTaskFiles } from '../../../../api';
import { FileExplorer } from '../../../FileExplorer/FileExplorer';
import type { FileEntry } from '../../../../types';

interface TaskDirectoryTabProps {
  taskId: string;
  files: FileEntry[];
}

export function TaskDirectoryTab({ taskId, files }: TaskDirectoryTabProps) {
  const loadTaskDirectory = useCallback(async (path?: string) => {
    return getTaskFiles(taskId, path);
  }, [taskId]);

  const loadTaskFile = useCallback(async (path: string) => {
    return getTaskFile(taskId, path);
  }, [taskId]);

  return (
    <FileExplorer
      files={files}
      loadDirectory={loadTaskDirectory}
      loadFileContent={loadTaskFile}
      treeTitle="文件列表"
      emptyMessage="暂无文件"
      placeholderMessage="选择文件查看内容"
    />
  );
}
