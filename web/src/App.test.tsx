import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';

import { App } from './App';

test('renders the dashboard shell with left navigation rail', async () => {
  vi.stubGlobal(
    'fetch',
    vi.fn().mockImplementation(() =>
      Promise.resolve(
        new Response(
          JSON.stringify({
            status_counts: {
              total: 1,
              running: 1,
              paused: 0,
              canceled: 0,
              done: 0,
            },
            recent_tasks: [
              {
                task_id: '11111111-1111-1111-1111-111111111111',
                description: 'Initial shell task',
                agent: 'codex',
                status: 'running',
                actions: { pause: true, resume: false, cancel: true, relaunch: true },
                parent_task_id: null,
                root_task_id: '11111111-1111-1111-1111-111111111111',
                created_at: 1730000000000,
                never_ends: false,
              },
            ],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      ),
    ),
  );

  render(<App />);

  expect(await screen.findByRole('navigation', { name: 'Primary' })).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Tasks' })).toBeInTheDocument();
  expect(
    screen.queryByRole('heading', { name: 'Local task control plane' }),
  ).not.toBeInTheDocument();
});

test('shell refresh keeps collapsed task branches collapsed on the tasks route', async () => {
  window.history.pushState({}, 'Tasks', '/tasks?root_task_id=root-beta&task_id=beta-child');

  const fetchMock = vi.fn().mockImplementation((input: RequestInfo | URL) => {
    const requestUrl = typeof input === 'string' ? input : input.toString();
    const url = new URL(requestUrl, 'http://localhost');

    if (requestUrl === '/api/tasks?root_only=true') {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            tasks: [
              {
                task_id: 'root-beta',
                description: 'Root Beta',
                agent: 'claude',
                status: 'running',
                actions: { pause: false, resume: false, cancel: false, relaunch: false },
                parent_task_id: null,
                root_task_id: 'root-beta',
                created_at: 1730000000300,
                never_ends: false,
              },
            ],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (requestUrl === '/api/tasks/root-beta/tree') {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            root_task_id: 'root-beta',
            root: {
              task: {
                task_id: 'root-beta',
                description: 'Root Beta',
                agent: 'claude',
                status: 'running',
                actions: { pause: false, resume: false, cancel: false, relaunch: false },
                parent_task_id: null,
                root_task_id: 'root-beta',
                created_at: 1730000000300,
                never_ends: false,
              },
              children: [
                {
                  task: {
                    task_id: 'beta-child',
                    description: 'Beta child',
                    agent: 'claude',
                    status: 'running',
                    actions: { pause: false, resume: false, cancel: false, relaunch: false },
                    parent_task_id: 'root-beta',
                    root_task_id: 'root-beta',
                    created_at: 1730000000400,
                    never_ends: false,
                  },
                  children: [
                    {
                      task: {
                        task_id: 'beta-leaf',
                        description: 'Beta leaf',
                        agent: 'claude',
                        status: 'paused',
                        actions: { pause: false, resume: false, cancel: false, relaunch: false },
                        parent_task_id: 'beta-child',
                        root_task_id: 'root-beta',
                        created_at: 1730000000500,
                        never_ends: false,
                      },
                      children: [],
                    },
                  ],
                },
              ],
            },
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (requestUrl === '/api/tasks/beta-child/artifacts') {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: 'beta-child',
            artifacts: [
              { path: 'logs/debug.log', size_bytes: 12, is_text: true },
            ],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (url.pathname === '/api/tasks/beta-child/artifacts/content') {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: 'beta-child',
            path: url.searchParams.get('path'),
            is_text: true,
            size_bytes: 12,
            content: 'child log preview',
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    throw new Error(`Unexpected request: ${requestUrl}`);
  });

  vi.stubGlobal('fetch', fetchMock);

  render(<App />);

  const treePane = await screen.findByRole('region', { name: 'Task tree' });
  expect(await within(treePane).findByRole('button', { name: 'Beta leaf' })).toBeInTheDocument();

  fireEvent.click(within(treePane).getByRole('button', { name: 'Collapse Beta child' }));
  await waitFor(() => {
    expect(within(treePane).queryByRole('button', { name: 'Beta leaf' })).not.toBeInTheDocument();
  });

  fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));

  await waitFor(() => {
    expect(within(treePane).queryByRole('button', { name: 'Beta leaf' })).not.toBeInTheDocument();
  });
});
