import { NavLink } from 'react-router-dom';

interface NavigationItem {
  href: string;
  label: string;
}

interface SidebarNavProps {
  items: NavigationItem[];
}

export function SidebarNav({ items }: SidebarNavProps) {
  return (
    <nav aria-label="Primary" className="sidebar-nav">
      {items.map((item) => (
        <NavLink
          key={item.href}
          className={({ isActive }) =>
            isActive ? 'sidebar-nav__link sidebar-nav__link--active' : 'sidebar-nav__link'
          }
          end={item.href === '/'}
          to={item.href}
        >
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}
