import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

import { CreatePage } from './CreatePage';

test('blocks create submit when prompt is empty', async () => {
  render(
    <MemoryRouter>
      <CreatePage />
    </MemoryRouter>,
  );

  fireEvent.click(screen.getByRole('button', { name: /create task/i }));

  expect(await screen.findByText(/prompt is required/i)).toBeInTheDocument();
});
