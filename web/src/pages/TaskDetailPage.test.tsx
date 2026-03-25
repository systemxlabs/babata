import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';

import { TaskDetailPage } from './TaskDetailPage';

const taskId = '11111111-1111-1111-1111-111111111111';

function buildFetchMock() {
  return vi.fn().mockImplementation((input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === 'string' ? input : input.toString();
    const method = init?.method ?? 'GET';

    if (method === 'GET' && url === `/api/tasks/${taskId}`) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: taskId,
            description: 'Inspect runtime shell',
            agent: 'codex',
            status: 'running',
            actions: { pause: true, resume: false, cancel: true, relaunch: true },
            parent_task_id: null,
            root_task_id: taskId,
            created_at: 1730000000000,
            never_ends: false,
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (method === 'GET' && url === `/api/tasks/${taskId}/content`) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: taskId,
            progress_markdown: 'current progress first',
            task_markdown: 'task definition second',
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (method === 'GET' && url === `/api/tasks/${taskId}/tree`) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            root_task_id: taskId,
            parent: null,
            current: {
              task_id: taskId,
              description: 'Inspect runtime shell',
              agent: 'codex',
              status: 'running',
              actions: { pause: true, resume: false, cancel: true, relaunch: true },
              parent_task_id: null,
              root_task_id: taskId,
              created_at: 1730000000000,
              never_ends: false,
            },
            children: [],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (method === 'GET' && url === `/api/tasks/${taskId}/logs`) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: taskId,
            supported: false,
            reason: 'No known log files for this agent',
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    if (method === 'GET' && url === `/api/tasks/${taskId}/artifacts`) {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            task_id: taskId,
            artifacts: [],
          }),
          {
            status: 200,
            headers: { 'Content-Type': 'application/json' },
          },
        ),
      );
    }

    return Promise.resolve(
      new Response(JSON.stringify({ ok: true, task_id: taskId, action: 'noop' }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );
  });
}

function renderDetailPage() {
  render(
    <MemoryRouter initialEntries={[`/tasks/${taskId}`]}>
      <Routes>
        <Route path="/tasks/:taskId" element={<TaskDetailPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

test('renders semantic progress before task definition', async () => {
  vi.stubGlobal('fetch', buildFetchMock());

  renderDetailPage();

  const progressText = await screen.findByText('current progress first');
  const taskText = screen.getByText('task definition second');

  expect(progressText.compareDocumentPosition(taskText) & Node.DOCUMENT_POSITION_FOLLOWING).toBe(
    Node.DOCUMENT_POSITION_FOLLOWING,
  );
});

test('cancel action requires confirmation', async () => {
  const fetchMock = buildFetchMock();
  vi.stubGlobal('fetch', fetchMock);
  const confirmMock = vi.spyOn(window, 'confirm').mockReturnValue(false);

  renderDetailPage();

  await screen.findByText('No known log files for this agent');
  fireEvent.click(screen.getByRole('button', { name: 'Cancel task' }));

  expect(confirmMock).toHaveBeenCalled();
  expect(
    fetchMock.mock.calls.some(
      ([input, init]) =>
        input === `/api/tasks/${taskId}/cancel` && (init as RequestInit | undefined)?.method === 'POST',
    ),
  ).toBe(false);
});

test('relaunch action requires a reason', async () => {
  const fetchMock = buildFetchMock();
  vi.stubGlobal('fetch', fetchMock);
  vi.spyOn(window, 'prompt').mockReturnValue('   ');

  renderDetailPage();

  await screen.findByText('No known log files for this agent');
  fireEvent.click(screen.getByRole('button', { name: 'Relaunch task' }));

  expect(
    fetchMock.mock.calls.some(
      ([input, init]) =>
        input === `/api/tasks/${taskId}/relaunch` &&
        (init as RequestInit | undefined)?.method === 'POST',
    ),
  ).toBe(false);
  expect(screen.getByText('Relaunch reason is required.')).toBeInTheDocument();
});
