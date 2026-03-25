import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

import { TasksPage } from './TasksPage';

test('tasks page sends status and query filters to the tasks API', async () => {
  const fetchMock = vi.fn().mockImplementation(() =>
    Promise.resolve(
      new Response(
        JSON.stringify({
          tasks: [],
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        },
      ),
    ),
  );

  vi.stubGlobal('fetch', fetchMock);

  render(
    <MemoryRouter initialEntries={['/tasks?status=running&query=forge']}>
      <TasksPage />
    </MemoryRouter>,
  );

  expect(await screen.findByText('No tasks matched the current filters.')).toBeInTheDocument();
  expect(fetchMock).toHaveBeenCalledWith(
    '/api/tasks?status=running&query=forge',
    expect.objectContaining({
      headers: { Accept: 'application/json' },
    }),
  );
});
