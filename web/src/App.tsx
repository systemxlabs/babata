import { useEffectEvent, useRef } from 'react';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';

import { AppShell } from './components/AppShell';
import { ShellToolbarProvider } from './components/ShellToolbarContext';
import { StatusBadge } from './components/StatusBadge';
import { Toolbar } from './components/Toolbar';
import { usePolling } from './hooks/usePolling';
import { CreatePage } from './pages/CreatePage';
import { OverviewPage } from './pages/OverviewPage';
import { SystemPage } from './pages/SystemPage';
import { TaskDetailPage } from './pages/TaskDetailPage';
import { TasksPage } from './pages/TasksPage';

const navigationItems = [
  { href: '/', label: 'Overview' },
  { href: '/tasks', label: 'Tasks' },
  { href: '/create', label: 'Create' },
  { href: '/system', label: 'System' },
];

export function App() {
  return (
    <BrowserRouter>
      <DashboardShell />
    </BrowserRouter>
  );
}

function DashboardShell() {
  const pageRefreshHandlerRef = useRef<(() => Promise<void> | void) | null>(null);
  const shellPulse = usePolling(
    async () => ({
      cycleStartedAt: Date.now(),
    }),
    { intervalMs: 5000, immediate: false },
  );
  const registerRefreshHandler = useEffectEvent((handler: (() => Promise<void> | void) | null) => {
    pageRefreshHandlerRef.current = handler;
  });

  return (
    <AppShell
      navigationItems={navigationItems}
      statusSlot={
        <div className="status-cluster" aria-label="Shell status">
          <StatusBadge compact status="running" />
          <p className="status-cluster__copy">
            Shared shell online
            <span>Embedded assets, router wiring, and polling controls are ready.</span>
          </p>
        </div>
      }
      toolbar={
        <Toolbar
          autoRefresh={shellPulse.autoRefresh}
          isRefreshing={shellPulse.isRefreshing || shellPulse.isLoading}
          lastRefreshedAt={shellPulse.lastRefreshedAt}
          onAutoRefreshChange={shellPulse.setAutoRefresh}
          onRefresh={() => {
            void shellPulse.refresh();
            if (pageRefreshHandlerRef.current) {
              void Promise.resolve(pageRefreshHandlerRef.current());
            }
          }}
        />
      }
    >
      <ShellToolbarProvider
        value={{
          autoRefresh: shellPulse.autoRefresh,
          registerRefreshHandler,
        }}
      >
        <Routes>
          <Route
            path="/"
            element={<OverviewPage />}
          />
          <Route
            path="/tasks"
            element={<TasksPage />}
          />
          <Route
            path="/tasks/:taskId"
            element={<TaskDetailPage />}
          />
          <Route
            path="/create"
            element={<CreatePage />}
          />
          <Route
            path="/system"
            element={<SystemPage />}
          />
          <Route path="*" element={<Navigate replace to="/" />} />
        </Routes>
      </ShellToolbarProvider>
    </AppShell>
  );
}
