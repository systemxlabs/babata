import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

import { CreatePage } from './CreatePage';

test('renders create form with a page-level heading', async () => {
  render(
    <MemoryRouter>
      <CreatePage />
    </MemoryRouter>,
  );

  expect(screen.getByRole('heading', { level: 1, name: 'Launch forge' })).toBeInTheDocument();

  fireEvent.click(screen.getByRole('button', { name: /create task/i }));

  expect(await screen.findByText(/prompt is required/i)).toBeInTheDocument();
});
