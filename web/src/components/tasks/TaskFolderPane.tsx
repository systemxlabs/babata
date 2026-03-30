import { useEffect, useRef, useState } from 'react';

import type { TaskArtifactContentResponse } from '../../api/types';
import type { ArtifactTreeNode } from '../../utils/tasks';

interface TaskFolderPaneProps {
  artifactTree: ArtifactTreeNode | null;
  error: Error | null;
  isLoading: boolean;
  onSelectFile: (path: string) => void;
  preview: TaskArtifactContentResponse | null;
  selectedPath: string | null;
  taskId: string | null;
}

export function TaskFolderPane({
  artifactTree,
  error,
  isLoading,
  onSelectFile,
  preview,
  selectedPath,
  taskId,
}: TaskFolderPaneProps) {
  const [expandedPaths, setExpandedPaths] = useState<string[]>([]);
  const lastTaskIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!artifactTree) {
      lastTaskIdRef.current = null;
      setExpandedPaths([]);
      return;
    }

    const nextPaths = collectDirectoryPaths(artifactTree);
    setExpandedPaths((current) => {
      if (taskId !== lastTaskIdRef.current) {
        return nextPaths;
      }

      const availablePaths = new Set(nextPaths);
      return current.filter((path) => availablePaths.has(path));
    });
    lastTaskIdRef.current = taskId;
  }, [artifactTree, taskId]);

  const expandedSet = new Set(expandedPaths);

  return (
    <section aria-label="Task folder" className="task-folder-pane">
      <div className="task-folder-pane__browser">
        {!artifactTree || !artifactTree.children.length ? (
          <p className="empty-state">No task artifacts are available for this task.</p>
        ) : (
          <ul className="artifact-tree">
            {artifactTree.children.map((node) => (
              <ArtifactBranch
                expandedPaths={expandedSet}
                key={node.name}
                node={node}
                onSelectFile={onSelectFile}
                onToggle={(path) => {
                  setExpandedPaths((current) =>
                    current.includes(path)
                      ? current.filter((entry) => entry !== path)
                      : [...current, path],
                  );
                }}
                parentPath=""
                selectedPath={selectedPath}
              />
            ))}
          </ul>
        )}
      </div>

      <div className="task-folder-pane__preview">
        {!selectedPath ? <p className="empty-state">Select a file to preview its output.</p> : null}
        {selectedPath && isLoading ? <p className="empty-state">Loading file preview...</p> : null}
        {selectedPath && error ? <p className="empty-state">Failed to load preview: {error.message}</p> : null}

        {selectedPath && !isLoading && !error && preview ? (
          <>
            <div className="task-folder-pane__preview-meta">
              <p className="task-folder-pane__preview-path">{preview.path}</p>
              <p className="task-folder-pane__preview-note">{preview.size_bytes} bytes</p>
            </div>

            {preview.is_text ? (
              <pre className="markdown-frame">{preview.content ?? ''}</pre>
            ) : (
              <p className="empty-state">{preview.reason ?? 'Preview is not available for this file.'}</p>
            )}
          </>
        ) : null}
      </div>
    </section>
  );
}

interface ArtifactBranchProps {
  expandedPaths: ReadonlySet<string>;
  node: ArtifactTreeNode;
  onSelectFile: (path: string) => void;
  onToggle: (path: string) => void;
  parentPath: string;
  selectedPath: string | null;
}

function ArtifactBranch({
  expandedPaths,
  node,
  onSelectFile,
  onToggle,
  parentPath,
  selectedPath,
}: ArtifactBranchProps) {
  const fullPath = parentPath ? `${parentPath}/${node.name}` : node.name;

  if (node.kind === 'file') {
    return (
      <li>
        <button
          className={
            selectedPath === fullPath
              ? 'artifact-tree__file artifact-tree__file--selected'
              : 'artifact-tree__file'
          }
          onClick={() => onSelectFile(fullPath)}
          type="button"
        >
          {node.name}
        </button>
      </li>
    );
  }

  const isExpanded = expandedPaths.has(fullPath);

  return (
    <li className="artifact-tree__directory">
      <button className="artifact-tree__toggle" onClick={() => onToggle(fullPath)} type="button">
        <span aria-hidden="true">{isExpanded ? '−' : '+'}</span>
        <span>{node.name}</span>
      </button>

      {isExpanded ? (
        <ul className="artifact-tree artifact-tree--nested">
          {node.children.map((child) => (
            <ArtifactBranch
              expandedPaths={expandedPaths}
              key={`${fullPath}/${child.name}`}
              node={child}
              onSelectFile={onSelectFile}
              onToggle={onToggle}
              parentPath={fullPath}
              selectedPath={selectedPath}
            />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

function collectDirectoryPaths(root: ArtifactTreeNode): string[] {
  const paths: string[] = [];

  function walk(node: ArtifactTreeNode, parentPath: string) {
    const fullPath = parentPath ? `${parentPath}/${node.name}` : node.name;

    if (node.kind === 'directory' && fullPath) {
      paths.push(fullPath);
    }

    node.children.forEach((child) => walk(child, fullPath));
  }

  root.children.forEach((child) => walk(child, ''));
  return paths;
}
