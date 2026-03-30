import type { PropsWithChildren, ReactNode } from 'react';

import { SidebarNav } from './SidebarNav';

interface NavigationItem {
  href: string;
  label: string;
}

interface AppShellProps extends PropsWithChildren {
  navigationItems: NavigationItem[];
  statusSlot?: ReactNode;
  toolbar: ReactNode;
}

export function AppShell({
  children,
  navigationItems,
  statusSlot,
  toolbar,
}: AppShellProps) {
  return (
    <div className="forge-shell">
      <aside className="shell-rail">
        <div className="shell-rail__brand">
          <p className="shell-rail__eyebrow">Babata dashboard</p>
          <p className="shell-rail__title">Control</p>
        </div>
        <SidebarNav items={navigationItems} />
        {statusSlot ? <div className="shell-rail__status">{statusSlot}</div> : null}
      </aside>

      <div className="shell-workbench">
        <header className="shell-toolbar">{toolbar}</header>
        <main className="shell-layout">{children}</main>
      </div>
    </div>
  );
}
