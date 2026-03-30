import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './App';
import './styles/app.css';

const rootElement = document.getElementById('app');

if (!rootElement) {
  throw new Error('Dashboard root element was not found');
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
