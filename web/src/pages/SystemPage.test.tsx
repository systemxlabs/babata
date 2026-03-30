import { render, screen } from '@testing-library/react';

import { SystemPage } from './SystemPage';

test('renders system metadata with a page-level heading', async () => {
  const fetchMock = vi.fn().mockImplementation(() =>
    Promise.resolve(
      new Response(
        JSON.stringify({
          version: '0.1.0',
          http_addr: '127.0.0.1:18800',
        }),
        {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        },
      ),
    ),
  );

  vi.stubGlobal('fetch', fetchMock);

  render(<SystemPage />);

  expect(await screen.findByRole('heading', { level: 1, name: 'Service ledger' })).toBeInTheDocument();
  expect(await screen.findByText('0.1.0')).toBeInTheDocument();
  expect(screen.getByText('http://127.0.0.1:18800')).toBeInTheDocument();
  expect(fetchMock).toHaveBeenCalledWith(
    '/api/system',
    expect.objectContaining({
      headers: { Accept: 'application/json' },
    }),
  );
});
