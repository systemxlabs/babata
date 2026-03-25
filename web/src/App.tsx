import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';

import { AppShell } from './components/AppShell';
import { Panel } from './components/Panel';
import { StatusBadge } from './components/StatusBadge';
import { Toolbar } from './components/Toolbar';
import { usePolling } from './hooks/usePolling';

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
          element={
            <ShellPlaceholder
              eyebrow="Overview"
              lead="This landing zone will become the live runtime summary: status counts, active attention, and recent task movement."
              title="Runtime sightline"
            />
          }
        />
        <Route
          path="/tasks"
          element={
            <ShellPlaceholder
              eyebrow="Tasks"
              lead="The task explorer surface is reserved for filtered lists, direct controls, and drill-down into tree-aware detail views."
              title="Explorer rail"
            />
          }
        />
        <Route
          path="/create"
          element={
            <ShellPlaceholder
              eyebrow="Create"
              lead="This forge bay will host the explicit task creation form with prompt, agent, parent task, and never-ends controls."
              title="Launch forge"
            />
          }
        />
        <Route
          path="/system"
          element={
            <ShellPlaceholder
              eyebrow="System"
              lead="System telemetry lands here next: health, listen address, version signals, and local runtime context."
              title="Service ledger"
            />
          }
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
