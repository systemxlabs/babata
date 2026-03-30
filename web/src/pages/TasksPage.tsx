import { useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { apiGet, buildApiPath } from '../api/client';
import {
  type RootTaskTreeResponse,
  type TaskArtifactContentResponse,
  type TaskArtifactsResponse,
  type TaskListResponse,
  type TaskSummary,
  type TaskTreeNode,
} from '../api/types';
import { Panel } from '../components/Panel';
import { TaskFolderPane } from '../components/tasks/TaskFolderPane';
import { RootTaskList } from '../components/tasks/RootTaskList';
import { useShellToolbar } from '../components/ShellToolbarContext';
import { TaskTreePane } from '../components/tasks/TaskTreePane';
import { WorkbenchToolbar } from '../components/tasks/WorkbenchToolbar';
import {
  buildArtifactTree,
  deriveRootTaskRows,
  deriveTimelineRows,
  flattenTreeIds,
  selectInitialTaskId,
  type ArtifactTreeNode,
} from '../utils/tasks';

type WorkbenchView = 'root' | 'timeline';

export function TasksPage() {
  const shellToolbar = useShellToolbar();
  const registerRefreshHandler = shellToolbar?.registerRefreshHandler;
  const [searchParams, setSearchParams] = useSearchParams();
  const [rows, setRows] = useState<TaskSummary[]>([]);
  const [tree, setTree] = useState<RootTaskTreeResponse | null>(null);
  const [artifacts, setArtifacts] = useState<TaskArtifactsResponse | null>(null);
  const [preview, setPreview] = useState<TaskArtifactContentResponse | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const hasLoadedRef = useRef(false);
  const requestIdRef = useRef(0);
  const skipNextLoadRef = useRef<string | null>(null);
  const treeRef = useRef<RootTaskTreeResponse | null>(null);
  const expandedIdsRef = useRef<Set<string>>(new Set());

  const view: WorkbenchView = searchParams.get('view') === 'timeline' ? 'timeline' : 'root';
  const requestedRootTaskId = searchParams.get('root_task_id');
  const requestedTaskId = searchParams.get('task_id');
  const requestedFile = searchParams.get('file');

  async function loadWorkbench() {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    if (!hasLoadedRef.current) {
      setIsLoading(true);
    } else {
      setIsRefreshing(true);
    }

    try {
      const nextRowsResponse = await apiGet<TaskListResponse>(
        view === 'timeline' ? '/tasks' : '/tasks?root_only=true',
      );
      const nextRows =
        view === 'timeline'
          ? deriveTimelineRows(nextRowsResponse.tasks)
          : deriveRootTaskRows(nextRowsResponse.tasks);

      const normalizedRootTaskId = resolveRootTaskId(view, nextRows, requestedRootTaskId);
      let nextTree: RootTaskTreeResponse | null = null;
      let normalizedTaskId: string | null = null;
      let nextArtifacts: TaskArtifactsResponse | null = null;
      let normalizedFile: string | null = null;
      let nextPreview: TaskArtifactContentResponse | null = null;
      let nextFolderError: string | null = null;

      if (normalizedRootTaskId) {
        nextTree = await apiGet<RootTaskTreeResponse>(`/tasks/${normalizedRootTaskId}/tree`);
        normalizedTaskId = resolveTaskId(nextTree.root, requestedTaskId);

        if (normalizedTaskId) {
          try {
            nextArtifacts = await apiGet<TaskArtifactsResponse>(`/tasks/${normalizedTaskId}/artifacts`);
            normalizedFile = resolveFilePath(nextArtifacts, requestedFile);

            if (normalizedFile) {
              try {
                nextPreview = await apiGet<TaskArtifactContentResponse>(
                  buildApiPath(`/tasks/${normalizedTaskId}/artifacts/content`, { path: normalizedFile }),
                );
              } catch (nextError) {
                nextFolderError = normalizeError(nextError, 'Failed to load preview');
                nextPreview = null;
              }
            }
          } catch (nextError) {
            nextFolderError = normalizeError(nextError, 'Failed to load task folder');
            nextArtifacts = {
              task_id: normalizedTaskId,
              artifacts: [],
            };
            normalizedFile = null;
            nextPreview = null;
          }
        }
      }

      if (requestId !== requestIdRef.current) {
        return;
      }

      const nextSearchParams = buildWorkbenchSearchParams(
        view,
        normalizedRootTaskId,
        normalizedTaskId,
        normalizedFile,
      );

      hasLoadedRef.current = true;
      const nextExpandedIds = nextTree
        ? mergeExpandedTaskIds(treeRef.current, expandedIdsRef.current, nextTree)
        : new Set<string>();
      setRows(nextRows);
      treeRef.current = nextTree;
      setTree(nextTree);
      setArtifacts(nextArtifacts);
      setPreview(nextPreview);
      setFolderError(nextFolderError);
      expandedIdsRef.current = nextExpandedIds;
      setExpandedIds(nextExpandedIds);
      setError(null);
      setIsLoading(false);
      setIsRefreshing(false);

      if (nextSearchParams.toString() !== searchParams.toString()) {
        skipNextLoadRef.current = nextSearchParams.toString();
        setSearchParams(nextSearchParams, { replace: true });
      }
    } catch (nextError) {
      if (requestId !== requestIdRef.current) {
        return;
      }

      const message = nextError instanceof Error ? nextError.message : 'Failed to load tasks';
      setError(message);
      setRows([]);
      treeRef.current = null;
      setTree(null);
      setArtifacts(null);
      setPreview(null);
      setFolderError(null);
      expandedIdsRef.current = new Set();
      setExpandedIds(new Set());
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }

  useEffect(() => {
    if (skipNextLoadRef.current === searchParams.toString()) {
      skipNextLoadRef.current = null;
      return;
    }

    void loadWorkbench();
  }, [searchParams, view, requestedRootTaskId, requestedTaskId, requestedFile]);

  useEffect(() => {
    if (!shellToolbar?.autoRefresh) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void loadWorkbench();
    }, 5000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [shellToolbar?.autoRefresh, view, requestedRootTaskId, requestedTaskId, requestedFile]);

  useEffect(() => {
    if (!registerRefreshHandler) {
      return;
    }

    registerRefreshHandler(() => loadWorkbench());

    return () => {
      registerRefreshHandler(null);
    };
  }, [registerRefreshHandler, view, requestedRootTaskId, requestedTaskId, requestedFile]);

  function updateSearchParams(
    nextView: WorkbenchView,
    rootTaskId: string | null,
    taskId: string | null,
    file: string | null,
  ) {
    setSearchParams(buildWorkbenchSearchParams(nextView, rootTaskId, taskId, file));
  }

  function handleViewChange(nextView: WorkbenchView) {
    updateSearchParams(nextView, requestedRootTaskId, requestedTaskId, requestedFile);
  }

  function handleRootSelect(task: TaskSummary) {
    const nextRootTaskId = view === 'timeline' ? task.root_task_id : task.task_id;
    const nextTaskId = task.task_id;
    updateSearchParams(view, nextRootTaskId, nextTaskId, null);
  }

  function handleTreeSelect(taskId: string) {
    updateSearchParams(view, requestedRootTaskId, taskId, null);
  }

  function handleFileSelect(path: string) {
    updateSearchParams(view, requestedRootTaskId, requestedTaskId, path);
  }

  function handleToggle(taskId: string) {
    setExpandedIds((current) => {
      const next = new Set(current);
      if (next.has(taskId)) {
        next.delete(taskId);
      } else {
        next.add(taskId);
      }
      expandedIdsRef.current = next;
      return next;
    });
  }

  const artifactTree: ArtifactTreeNode | null = artifacts ? buildArtifactTree(artifacts.artifacts) : null;

  return (
    <div className="page-stack">
      <Panel
        className="panel--hero"
        eyebrow="Tasks"
        meta={<span className="panel__tag">Workbench</span>}
        title="Task workbench"
        titleLevel={1}
      >
        <p className="panel__lede">
          Move between root context, descendants, and task artifacts without leaving the page.
        </p>
      </Panel>

      {error ? (
        <Panel eyebrow="Tasks" title="Workbench error">
          <p className="empty-state">Failed to load tasks: {error}</p>
        </Panel>
      ) : null}

      {!error && isLoading ? (
        <Panel eyebrow="Tasks" title="Loading workbench">
          <p className="empty-state">Loading root tasks, task tree, and folder preview...</p>
        </Panel>
      ) : null}

      {!error && !isLoading ? (
        <>
          <div className="tasks-workbench-grid">
            <Panel eyebrow="List" title="Task timeline">
              <WorkbenchToolbar
                onViewChange={handleViewChange}
                view={view}
              />
              <RootTaskList
                onSelect={handleRootSelect}
                rows={rows}
                selectedRootTaskId={requestedRootTaskId}
                selectedTaskId={requestedTaskId}
                view={view}
              />
            </Panel>

            <Panel eyebrow="Tree" title="Hierarchy">
              <TaskTreePane
                expandedIds={expandedIds}
                onSelect={handleTreeSelect}
                onToggle={handleToggle}
                root={tree?.root ?? null}
                selectedTaskId={requestedTaskId}
              />
            </Panel>
          </div>

          <Panel eyebrow="Folder" title="Task folder">
            <TaskFolderPane
              artifactTree={artifactTree}
              error={folderError ? new Error(folderError) : null}
              isLoading={isRefreshing && Boolean(requestedFile)}
              onSelectFile={handleFileSelect}
              preview={preview}
              selectedPath={requestedFile}
              taskId={requestedTaskId}
            />
          </Panel>
        </>
      ) : null}
    </div>
  );
}

function mergeExpandedTaskIds(
  currentTree: RootTaskTreeResponse | null,
  currentExpandedIds: Set<string>,
  nextTree: RootTaskTreeResponse,
) {
  if (!nextTree.root || !nextTree.root.task) {
    return new Set<string>();
  }

  const nextIds = new Set(flattenTreeIds(nextTree.root));

  if (!currentTree || !currentTree.root || currentTree.root.task.task_id !== nextTree.root.task.task_id) {
    return nextIds;
  }

  const persisted = new Set<string>();
  currentExpandedIds.forEach((taskId) => {
    if (nextIds.has(taskId)) {
      persisted.add(taskId);
    }
  });
  return persisted;
}

function normalizeError(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function resolveRootTaskId(
  view: WorkbenchView,
  rows: TaskSummary[],
  requestedRootTaskId: string | null,
) {
  if (requestedRootTaskId) {
    const match = rows.find(
      (task) =>
        task.task_id === requestedRootTaskId ||
        task.root_task_id === requestedRootTaskId,
    );

    if (match) {
      return match.root_task_id;
    }
  }

  if (rows.length === 0) {
    return null;
  }

  return view === 'timeline' ? rows[0].root_task_id : rows[0].task_id;
}

function resolveTaskId(root: TaskTreeNode | null | undefined, requestedTaskId: string | null) {
  if (!root || !root.task) {
    return null;
  }

  if (requestedTaskId && flattenTreeIds(root).includes(requestedTaskId)) {
    return requestedTaskId;
  }

  return selectInitialTaskId(root);
}

function resolveFilePath(
  artifacts: TaskArtifactsResponse,
  requestedFile: string | null,
) {
  if (!requestedFile) {
    return null;
  }

  return artifacts.artifacts.some((artifact) => artifact.path === requestedFile)
    ? requestedFile
    : null;
}

function buildWorkbenchSearchParams(
  view: WorkbenchView,
  rootTaskId: string | null,
  taskId: string | null,
  file: string | null,
) {
  const next = new URLSearchParams();

  if (view === 'timeline') {
    next.set('view', 'timeline');
  }

  if (rootTaskId) {
    next.set('root_task_id', rootTaskId);
  }

  if (taskId) {
    next.set('task_id', taskId);
  }

  if (file) {
    next.set('file', file);
  }

  return next;
}
