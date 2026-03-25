import { render, screen } from '@testing-library/react';

import { App } from './App';

test('renders the dashboard shell navigation and refresh controls', () => {
  render(<App />);

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
