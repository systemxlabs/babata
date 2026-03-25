import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

import { OverviewPage } from './OverviewPage';

test('renders overview counters and recent tasks from the API', async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(
      JSON.stringify({
        status_counts: {
          total: 4,
          running: 2,
          paused: 1,
          canceled: 0,
          done: 1,
        },
        recent_tasks: [
          {
            task_id: '11111111-1111-1111-1111-111111111111',
            description: 'Refine the runtime shell',
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
  );

  vi.stubGlobal('fetch', fetchMock);

  render(
    <MemoryRouter>
      <OverviewPage />
    </MemoryRouter>,
  );

  expect(await screen.findByText('Running')).toBeInTheDocument();
  expect(screen.getByText('2')).toBeInTheDocument();
  expect(screen.getAllByText('Refine the runtime shell')).not.toHaveLength(0);
  expect(fetchMock).toHaveBeenCalledWith(
    '/api/overview',
    expect.objectContaining({
      headers: { Accept: 'application/json' },
    }),
  );
});
