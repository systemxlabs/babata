import { render, screen } from '@testing-library/react';

import { App } from './App';

test('renders the dashboard shell navigation', () => {
  render(<App />);

  expect(screen.getByRole('heading', { name: 'Local task control plane' })).toBeInTheDocument();
  expect(screen.getByText('Overview')).toBeInTheDocument();
  expect(screen.getByText('Tasks')).toBeInTheDocument();
  expect(screen.getByText('Create')).toBeInTheDocument();
  expect(screen.getByText('System')).toBeInTheDocument();
});
