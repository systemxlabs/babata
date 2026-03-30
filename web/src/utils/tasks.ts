import type { TaskArtifact, TaskSummary, TaskTreeNode } from '../api/types';

const compareTasks = (a: TaskSummary, b: TaskSummary): number => {
  if (a.created_at !== b.created_at) {
    return b.created_at - a.created_at;
  }

  return a.task_id.localeCompare(b.task_id);
};

export function deriveRootTaskRows(tasks: TaskSummary[]): TaskSummary[] {
  return [...tasks]
    .filter((task) => task.parent_task_id === null)
    .sort(compareTasks);
}

export function deriveTimelineRows(tasks: TaskSummary[]): TaskSummary[] {
  return [...tasks].sort(compareTasks);
}

export function buildTaskTree(tasks: TaskSummary[], rootTaskId: string): TaskTreeNode | null {
  const nodeMap = new Map<string, TaskTreeNode>();

  tasks.forEach((task) => {
    nodeMap.set(task.task_id, { task, children: [] });
  });

  tasks.forEach((task) => {
    if (task.parent_task_id) {
      const parent = nodeMap.get(task.parent_task_id);
      const child = nodeMap.get(task.task_id);
      if (parent && child) {
        parent.children.push(child);
      }
    }
  });

  nodeMap.forEach((node) => node.children.sort((a, b) => compareTasks(a.task, b.task)));

  return nodeMap.get(rootTaskId) ?? null;
}

export function flattenTreeIds(node: TaskTreeNode): string[] {
  const ids: string[] = [];

  function walk(current: TaskTreeNode) {
    ids.push(current.task.task_id);
    current.children.forEach(walk);
  }

  walk(node);
  return ids;
}

export interface ArtifactTreeNode {
  name: string;
  kind: 'directory' | 'file';
  size_bytes?: number;
  is_text?: boolean;
  children: ArtifactTreeNode[];
}

export function buildArtifactTree(artifacts: TaskArtifact[]): ArtifactTreeNode {
  const root: ArtifactTreeNode = { name: '', kind: 'directory', children: [] };

  const insertNode = (pathSegments: string[], meta: TaskArtifact) => {
    let cursor = root;
    pathSegments.forEach((segment, index) => {
      const existing = cursor.children.find((child) => child.name === segment);
      const isFile = index === pathSegments.length - 1;

      if (existing) {
        cursor = existing;
        return;
      }

      const node: ArtifactTreeNode = {
        name: segment,
        kind: isFile ? 'file' : 'directory',
        children: [],
      };

      if (isFile) {
        node.size_bytes = meta.size_bytes;
        node.is_text = meta.is_text;
      }

      cursor.children.push(node);
      cursor = node;
    });
  };

  artifacts.forEach((artifact) => {
    const segments = artifact.path.split('/').filter(Boolean);
    if (segments.length === 0) {
      return;
    }
    insertNode(segments, artifact);
  });

  const sortChildren = (node: ArtifactTreeNode) => {
    node.children.sort((a, b) => {
      if (a.kind !== b.kind) {
        return a.kind === 'directory' ? -1 : 1;
      }

      return a.name.localeCompare(b.name);
    });

    node.children.forEach(sortChildren);
  };

  sortChildren(root);

  return root;
}

export function selectInitialTaskId(rootTree: TaskTreeNode | null): string | null {
  if (!rootTree || !rootTree.task) {
    return null;
  }

  return rootTree.task.task_id;
}
