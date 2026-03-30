import { TaskActions, TaskArtifact, TaskSummary } from '../api/types';
import {
  buildArtifactTree,
  buildTaskTree,
  deriveRootTaskRows,
  deriveTimelineRows,
  flattenTreeIds,
  selectInitialTaskId,
} from './tasks';

const defaultActions: TaskActions = {
  pause: false,
  resume: false,
  cancel: false,
  relaunch: false,
};

const newerRootId = 'root-newer';
const olderRootId = 'root-older';

const tasks: TaskSummary[] = [
  {
    task_id: olderRootId,
    description: 'Older root',
    agent: null,
    status: 'running',
    actions: defaultActions,
    parent_task_id: null,
    root_task_id: olderRootId,
    created_at: 1_000,
    never_ends: false,
  },
  {
    task_id: `${olderRootId}-child`,
    description: 'Child of older root',
    agent: null,
    status: 'running',
    actions: defaultActions,
    parent_task_id: olderRootId,
    root_task_id: olderRootId,
    created_at: 1_001,
    never_ends: false,
  },
  {
    task_id: newerRootId,
    description: 'Newer root',
    agent: null,
    status: 'running',
    actions: defaultActions,
    parent_task_id: null,
    root_task_id: newerRootId,
    created_at: 2_000,
    never_ends: false,
  },
  {
    task_id: `${newerRootId}-child`,
    description: 'Child of newer root',
    agent: null,
    status: 'running',
    actions: defaultActions,
    parent_task_id: newerRootId,
    root_task_id: newerRootId,
    created_at: 2_001,
    never_ends: false,
  },
];

const artifacts: TaskArtifact[] = [
  { path: 'notes/output.md', size_bytes: 12, is_text: true },
];

test('buildTaskTree nests descendants beneath the selected root task', () => {
  const tree = buildTaskTree(tasks, newerRootId);
  expect(tree).not.toBeNull();
  expect(tree?.children).toHaveLength(1);
});

test('buildArtifactTree groups slash-delimited paths into directories', () => {
  expect(buildArtifactTree(artifacts)).toMatchObject({
    children: expect.arrayContaining([
      expect.objectContaining({ name: 'notes', kind: 'directory' }),
    ]),
  });
});

test('deriveRootTaskRows returns root tasks in descending created_at order', () => {
  expect(deriveRootTaskRows(tasks).map((task) => task.task_id)).toEqual([
    newerRootId,
    olderRootId,
  ]);
});

test('buildTaskTree returns null when root is missing', () => {
  expect(buildTaskTree(tasks, 'missing')).toBeNull();
});

test('equal timestamps keep deterministic ordering via task_id tie-breaker', () => {
  const tiedTasks: TaskSummary[] = [
    { ...tasks[0], task_id: 'root-old', parent_task_id: null, created_at: 1_000 },
    { ...tasks[2], task_id: 'aaa', parent_task_id: null, created_at: 1_000 },
  ];
  const roots = deriveRootTaskRows(tiedTasks);
  expect(roots.map((task) => task.task_id)).toEqual(['aaa', 'root-old']);
});

test('deriveTimelineRows keeps deterministic ordering when timestamps tie', () => {
  const tiedTasks: TaskSummary[] = [
    { ...tasks[0], task_id: 'task-b', created_at: 3_000 },
    { ...tasks[2], task_id: 'task-a', created_at: 3_000 },
    { ...tasks[1], task_id: 'task-c', created_at: 2_000 },
  ];

  expect(deriveTimelineRows(tiedTasks).map((task) => task.task_id)).toEqual([
    'task-a',
    'task-b',
    'task-c',
  ]);
});

test('flattenTreeIds walks the full tree depth-first', () => {
  const tree = buildTaskTree(tasks, newerRootId);
  expect(flattenTreeIds(tree!)).toEqual([
    newerRootId,
    `${newerRootId}-child`,
  ]);
});

test('selectInitialTaskId handles null roots defensively', () => {
  expect(selectInitialTaskId(null)).toBeNull();
});

test('buildTaskTree sorts siblings deterministically for the same parent', () => {
  const siblingTasks: TaskSummary[] = [
    {
      ...tasks[2],
      task_id: 'root',
      root_task_id: 'root',
      parent_task_id: null,
      created_at: 5_000,
    },
    {
      ...tasks[0],
      task_id: 'child-b',
      root_task_id: 'root',
      parent_task_id: 'root',
      created_at: 4_000,
    },
    {
      ...tasks[0],
      task_id: 'child-a',
      root_task_id: 'root',
      parent_task_id: 'root',
      created_at: 4_000,
    },
  ];

  const tree = buildTaskTree(siblingTasks, 'root');
  expect(tree?.children.map((child) => child.task.task_id)).toEqual([
    'child-a',
    'child-b',
  ]);
});

test('artifact tree sorts directories before files alphabetically', () => {
  const mixedArtifacts: TaskArtifact[] = [
    { path: 'b/file.txt', size_bytes: 1, is_text: true },
    { path: 'a/child/file.txt', size_bytes: 1, is_text: true },
    { path: 'a/file.txt', size_bytes: 1, is_text: true },
    { path: 'a/alpha.txt', size_bytes: 1, is_text: true },
  ];
  const tree = buildArtifactTree(mixedArtifacts);
  const firstLevel = tree.children.map((child) => child.name);
  expect(firstLevel).toEqual(['a', 'b']);

  const aNode = tree.children.find((child) => child.name === 'a');
  expect(aNode?.children.map((child) => `${child.kind}:${child.name}`)).toEqual([
    'directory:child',
    'file:alpha.txt',
    'file:file.txt',
  ]);
});
