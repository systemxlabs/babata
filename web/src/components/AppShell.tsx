import type { PropsWithChildren, ReactNode } from 'react';
import { NavLink } from 'react-router-dom';

interface NavigationItem {
  href: string;
  label: string;
}

interface AppShellProps extends PropsWithChildren {
  copy: string;
  eyebrow: string;
  navigationItems: NavigationItem[];
  statusSlot?: ReactNode;
  title: string;
  toolbar: ReactNode;
}

export function AppShell({
  children,
  copy,
  eyebrow,
  navigationItems,
  statusSlot,
  title,
  toolbar,
}: AppShellProps) {
  return (
    <div className="forge-shell">
      <div className="forge-shell__mesh" aria-hidden="true" />
      <div className="forge-shell__flare forge-shell__flare--north" aria-hidden="true" />
      <div className="forge-shell__flare forge-shell__flare--south" aria-hidden="true" />

      <header className="masthead">
        <div className="masthead__copy">
          <p className="masthead__eyebrow">{eyebrow}</p>
          <h1>{title}</h1>
          <p className="masthead__lede">{copy}</p>
        </div>
        <div className="masthead__status">{statusSlot}</div>
      </header>

      <div className="command-bar">
        <nav aria-label="Primary" className="command-nav">
          {navigationItems.map((item) => (
            <NavLink
              key={item.href}
              className={({ isActive }) =>
                isActive ? 'command-nav__link command-nav__link--active' : 'command-nav__link'
              }
              end={item.href === '/'}
              to={item.href}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="command-bar__toolbar">{toolbar}</div>
      </div>

      <main className="shell-layout">{children}</main>
    </div>
  );
}
