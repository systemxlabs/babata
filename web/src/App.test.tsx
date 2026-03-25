import { render, screen } from '@testing-library/react';

import { App } from './App';

test('renders the dashboard shell navigation and refresh controls', async () => {
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

  expect(await screen.findAllByText('Initial shell task')).not.toHaveLength(0);

  expect(
    screen.getByRole('heading', { name: 'Local task control plane' }),
  ).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Overview' })).toHaveAttribute('href', '/');
  expect(screen.getByRole('link', { name: 'Tasks' })).toHaveAttribute('href', '/tasks');
  expect(screen.getByRole('link', { name: 'Create' })).toHaveAttribute('href', '/create');
  expect(screen.getByRole('link', { name: 'System' })).toHaveAttribute('href', '/system');
  expect(screen.getByRole('button', { name: 'Refresh' })).toBeInTheDocument();
  expect(screen.getByRole('checkbox', { name: 'Auto refresh' })).toBeChecked();
});
