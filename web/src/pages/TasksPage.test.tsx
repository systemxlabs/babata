import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';

import { TasksPage } from './TasksPage';

const defaultActions = {
  pause: false,
  resume: false,
  cancel: false,
  relaunch: false,
} as const;

const rootAlpha = {
  task_id: 'root-alpha',
  description: 'Root Alpha',
  agent: 'codex',
  status: 'running',
  actions: defaultActions,
  parent_task_id: null,
  root_task_id: 'root-alpha',
  created_at: 1_730_000_000_100,
  never_ends: false,
};

const alphaChild = {
  task_id: 'alpha-child',
  description: 'Alpha child',
  agent: 'codex',
  status: 'done',
  actions: defaultActions,
  parent_task_id: 'root-alpha',
  root_task_id: 'root-alpha',
  created_at: 1_730_000_000_200,
  never_ends: false,
};

const rootBeta = {
  task_id: 'root-beta',
  description: 'Root Beta',
  agent: 'claude',
  status: 'running',
  actions: defaultActions,
  parent_task_id: null,
  root_task_id: 'root-beta',
  created_at: 1_730_000_000_300,
  never_ends: false,
};

const betaChild = {
  task_id: 'beta-child',
  description: 'Beta child',
  agent: 'claude',
  status: 'running',
  actions: defaultActions,
  parent_task_id: 'root-beta',
  root_task_id: 'root-beta',
  created_at: 1_730_000_000_400,
  never_ends: false,
};

const betaLeaf = {
  task_id: 'beta-leaf',
  description: 'Beta leaf',
  agent: 'claude',
  status: 'paused',
  actions: defaultActions,
  parent_task_id: 'beta-child',
  root_task_id: 'root-beta',
  created_at: 1_730_000_000_500,
  never_ends: false,
};

const rootOnlyTasks = [rootAlpha, rootBeta];
const allTasks = [betaLeaf, rootAlpha, betaChild, rootBeta, alphaChild];

function jsonResponse(body: unknown) {
  return Promise.resolve(
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    }),
  );
}

interface FetchMockOptions {
  betaChildPreviewResponse?: () => Promise<Response>;
}

function buildFetchMock(options: FetchMockOptions = {}) {
  return vi.fn().mockImplementation((input: RequestInfo | URL, init?: RequestInit) => {
    const requestUrl = typeof input === 'string' ? input : input.toString();
    const url = new URL(requestUrl, 'http://localhost');
    const method = init?.method ?? 'GET';

    if (method !== 'GET') {
      throw new Error(`Unexpected request: ${method} ${requestUrl}`);
    }

    if (requestUrl === '/api/tasks?root_only=true') {
      return jsonResponse({ tasks: rootOnlyTasks });
    }

    if (requestUrl === '/api/tasks') {
      return jsonResponse({ tasks: allTasks });
    }

    if (requestUrl === '/api/tasks/root-alpha/tree') {
      return jsonResponse({
        root_task_id: rootAlpha.task_id,
        root: {
          task: rootAlpha,
          children: [{ task: alphaChild, children: [] }],
        },
      });
    }

    if (requestUrl === '/api/tasks/root-beta/tree') {
      return jsonResponse({
        root_task_id: rootBeta.task_id,
        root: {
          task: rootBeta,
          children: [
            {
              task: betaChild,
              children: [{ task: betaLeaf, children: [] }],
            },
          ],
        },
      });
    }

    if (requestUrl === '/api/tasks/root-alpha/artifacts') {
      return jsonResponse({
        task_id: rootAlpha.task_id,
        artifacts: [{ path: 'plan.md', size_bytes: 42, is_text: true }],
      });
    }

    if (requestUrl === '/api/tasks/root-beta/artifacts') {
      return jsonResponse({
        task_id: rootBeta.task_id,
        artifacts: [{ path: 'summary.md', size_bytes: 18, is_text: true }],
      });
    }

    if (requestUrl === '/api/tasks/alpha-child/artifacts') {
      return jsonResponse({
        task_id: alphaChild.task_id,
        artifacts: [{ path: 'notes/result.txt', size_bytes: 12, is_text: true }],
      });
    }

    if (requestUrl === '/api/tasks/beta-child/artifacts') {
      return jsonResponse({
        task_id: betaChild.task_id,
        artifacts: [
          { path: 'logs/debug.log', size_bytes: 12, is_text: true },
          { path: 'notes/output.txt', size_bytes: 24, is_text: true },
        ],
      });
    }

    if (requestUrl === '/api/tasks/beta-leaf/artifacts') {
      return jsonResponse({
        task_id: betaLeaf.task_id,
        artifacts: [{ path: 'report.md', size_bytes: 16, is_text: true }],
      });
    }

    if (url.pathname === '/api/tasks/beta-child/artifacts/content') {
      if (options.betaChildPreviewResponse) {
        return options.betaChildPreviewResponse();
      }

      return jsonResponse({
        task_id: betaChild.task_id,
        path: url.searchParams.get('path'),
        is_text: true,
        size_bytes: 12,
        content: 'child log preview',
      });
    }

    if (url.pathname === '/api/tasks/root-beta/artifacts/content') {
      return jsonResponse({
        task_id: rootBeta.task_id,
        path: url.searchParams.get('path'),
        is_text: true,
        size_bytes: 18,
        content: 'root summary preview',
      });
    }

    throw new Error(`Unexpected request: ${method} ${requestUrl}`);
  });
}

function LocationProbe() {
  const location = useLocation();
  return <output data-testid="location">{`${location.pathname}${location.search}`}</output>;
}

function renderPage(initialEntry = '/tasks', options?: FetchMockOptions) {
  const fetchMock = buildFetchMock(options);
  vi.stubGlobal('fetch', fetchMock);

  render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <TasksPage />
      <LocationProbe />
    </MemoryRouter>,
  );

  return { fetchMock };
}

function countCalls(fetchMock: ReturnType<typeof vi.fn>, requestUrl: string) {
  return fetchMock.mock.calls.filter(([input]) => input === requestUrl).length;
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test('tasks page defaults to root-task mode and fetches root-only tasks', async () => {
  const { fetchMock } = renderPage();

  expect(await screen.findByRole('button', { name: 'Root tasks' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  expect(fetchMock).toHaveBeenCalledWith('/api/tasks?root_only=true', expect.anything());
});

test('tasks page normalizes the initial URL without reloading the workbench twice', async () => {
  const { fetchMock } = renderPage();

  await waitFor(() => {
    expect(countCalls(fetchMock, '/api/tasks/root-beta/tree')).toBeGreaterThan(0);
  });

  expect(countCalls(fetchMock, '/api/tasks?root_only=true')).toBe(1);
  expect(countCalls(fetchMock, '/api/tasks/root-beta/tree')).toBe(1);
  expect(countCalls(fetchMock, '/api/tasks/root-beta/artifacts')).toBe(1);
});

test('tasks page toggles to all-task timeline mode', async () => {
  const { fetchMock } = renderPage();

  await screen.findByRole('button', { name: 'Root tasks' });
  fireEvent.click(screen.getByRole('button', { name: 'Timeline' }));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/tasks', expect.anything());
  });

  expect(screen.getByRole('button', { name: 'Timeline' })).toHaveAttribute('aria-pressed', 'true');
});

test('clicking a root row updates the tree pane', async () => {
  const { fetchMock } = renderPage();
  const rootList = await screen.findByRole('region', { name: 'Root task list' });

  fireEvent.click(within(rootList).getByRole('button', { name: 'Root Alpha' }));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/tasks/root-alpha/tree', expect.anything());
  });

  const treePane = await screen.findByRole('region', { name: 'Task tree' });
  expect(await within(treePane).findByRole('button', { name: 'Alpha child' })).toBeInTheDocument();
});

test('clicking a tree node loads task folder content', async () => {
  const { fetchMock } = renderPage();
  const treePane = await screen.findByRole('region', { name: 'Task tree' });

  fireEvent.click(await within(treePane).findByRole('button', { name: 'Beta child' }));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith('/api/tasks/beta-child/artifacts', expect.anything());
  });

  const folderPane = await screen.findByRole('region', { name: 'Task folder' });
  fireEvent.click(await within(folderPane).findByRole('button', { name: 'logs' }));
  expect(await within(folderPane).findByRole('button', { name: 'debug.log' })).toBeInTheDocument();
});

test('clicking a file updates the preview pane', async () => {
  const { fetchMock } = renderPage('/tasks?root_task_id=root-beta&task_id=beta-child');
  const folderPane = await screen.findByRole('region', { name: 'Task folder' });

  fireEvent.click(await within(folderPane).findByRole('button', { name: 'debug.log' }));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/tasks/beta-child/artifacts/content?path=logs%2Fdebug.log',
      expect.anything(),
    );
  });

  expect(await screen.findByText('child log preview')).toBeInTheDocument();
});

test('collapsed tree branches stay collapsed after selecting a file', async () => {
  renderPage('/tasks?root_task_id=root-beta&task_id=beta-child');
  const treePane = await screen.findByRole('region', { name: 'Task tree' });

  expect(await within(treePane).findByRole('button', { name: 'Beta leaf' })).toBeInTheDocument();
  fireEvent.click(within(treePane).getByRole('button', { name: 'Collapse Beta child' }));

  await waitFor(() => {
    expect(within(treePane).queryByRole('button', { name: 'Beta leaf' })).not.toBeInTheDocument();
  });

  const folderPane = await screen.findByRole('region', { name: 'Task folder' });
  fireEvent.click(await within(folderPane).findByRole('button', { name: 'debug.log' }));

  expect(await screen.findByText('child log preview')).toBeInTheDocument();
  expect(within(treePane).queryByRole('button', { name: 'Beta leaf' })).not.toBeInTheDocument();
});

test('preview failures stay inside the folder pane', async () => {
  renderPage('/tasks?root_task_id=root-beta&task_id=beta-child', {
    betaChildPreviewResponse: () =>
      Promise.resolve(
        new Response(JSON.stringify({ error: 'preview unavailable' }), {
          status: 500,
          headers: { 'Content-Type': 'application/json' },
        }),
      ),
  });
  const folderPane = await screen.findByRole('region', { name: 'Task folder' });

  fireEvent.click(await within(folderPane).findByRole('button', { name: 'debug.log' }));

  expect(await within(folderPane).findByText('Failed to load preview: preview unavailable')).toBeInTheDocument();
  expect(screen.getByRole('region', { name: 'Root task list' })).toBeInTheDocument();
  expect(screen.queryByText('Workbench error')).not.toBeInTheDocument();
});

test('empty text previews do not fall back to the unavailable message', async () => {
  renderPage('/tasks?root_task_id=root-beta&task_id=beta-child', {
    betaChildPreviewResponse: () =>
      jsonResponse({
        task_id: betaChild.task_id,
        path: 'logs/debug.log',
        is_text: true,
        size_bytes: 0,
        content: '',
      }),
  });
  const folderPane = await screen.findByRole('region', { name: 'Task folder' });

  fireEvent.click(await within(folderPane).findByRole('button', { name: 'debug.log' }));

  expect(await within(folderPane).findByText('logs/debug.log')).toBeInTheDocument();
  expect(within(folderPane).queryByText('Preview is not available for this file.')).not.toBeInTheDocument();
});

test('url search params keep view, root_task_id, task_id, and file', async () => {
  renderPage('/tasks?view=timeline&root_task_id=root-beta&task_id=beta-child&file=logs%2Fdebug.log');

  expect(await screen.findByRole('button', { name: 'Timeline' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  expect(await screen.findByText('child log preview')).toBeInTheDocument();
  expect(screen.getByTestId('location')).toHaveTextContent(
    '/tasks?view=timeline&root_task_id=root-beta&task_id=beta-child&file=logs%2Fdebug.log',
  );
});
