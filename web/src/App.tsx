import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';

import { AppShell } from './components/AppShell';
import { Panel } from './components/Panel';
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
  const shellPulse = usePolling(
    async () => ({
      cycleStartedAt: Date.now(),
    }),
    { intervalMs: 5000, immediate: false },
  );

  return (
    <AppShell
      copy="Forge Panels is now a real shell layer: routed navigation, refresh controls, and shared surface primitives are in place for the task-focused pages that follow."
      eyebrow="Babata dashboard"
      navigationItems={navigationItems}
      statusSlot={
        <div className="status-cluster" aria-label="Shell status">
          <StatusBadge status="running" />
          <p className="status-cluster__copy">
            Shared shell online
            <span>Embedded assets, router wiring, and polling controls are ready.</span>
          </p>
        </div>
      }
      title="Local task control plane"
      toolbar={
        <Toolbar
          autoRefresh={shellPulse.autoRefresh}
          isRefreshing={shellPulse.isRefreshing || shellPulse.isLoading}
          lastRefreshedAt={shellPulse.lastRefreshedAt}
          onAutoRefreshChange={shellPulse.setAutoRefresh}
          onRefresh={() => {
            void shellPulse.refresh();
          }}
        />
      }
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
    </AppShell>
  );
}

interface ShellPlaceholderProps {
  eyebrow: string;
  lead: string;
  title: string;
}

function ShellPlaceholder({ eyebrow, lead, title }: ShellPlaceholderProps) {
  return (
    <>
      <Panel
        className="panel--hero"
        eyebrow={eyebrow}
        meta={<span className="panel__tag">Shared shell phase</span>}
        title={title}
      >
        <p className="panel__lede">{lead}</p>
      </Panel>

      <div className="placeholder-grid">
        <Panel
          eyebrow="Control cards"
          meta={<StatusBadge status="paused" />}
          title="Command surfaces"
        >
          <ul className="signal-list">
            <li>Navigation is route-aware instead of static decoration.</li>
            <li>Refresh controls are shared so pages can opt into the same polling rhythm.</li>
            <li>Panels and status badges give later pages a consistent visual language.</li>
          </ul>
        </Panel>

        <Panel
          eyebrow="Build notes"
          meta={<StatusBadge status="done" />}
          title="Shell contract"
        >
          <ul className="signal-list">
            <li>`api/client` and `api/types` are ready for page-level data fetching.</li>
            <li>`usePolling` tracks refresh cadence, manual refresh, and last sync time.</li>
            <li>Task-focused pages can now land without re-solving layout or motion.</li>
          </ul>
        </Panel>
      </div>
    </>
  );
}
