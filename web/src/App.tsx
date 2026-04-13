import { BrowserRouter, NavLink, Navigate, Route, Routes } from 'react-router-dom';
import './App.css';
import { Dashboard } from './pages/Dashboard/Dashboard';
import { Agents } from './pages/Agents/Agents';
import { Channels } from './pages/Channels/Channels';
import { Providers } from './pages/Providers/Providers';
import { Skills } from './pages/Skills/Skills';
import { Tasks } from './pages/Tasks/Tasks';

type PageType = 'dashboard' | 'tasks' | 'agents' | 'providers' | 'channels' | 'skills';

const navItems: { key: PageType; path: string; label: string; icon: string }[] = [
  { key: 'dashboard', path: '/', label: 'Dashboard', icon: '📊' },
  { key: 'tasks', path: '/tasks', label: 'Tasks', icon: '📋' },
  { key: 'agents', path: '/agents', label: 'Agents', icon: '🤖' },
  { key: 'providers', path: '/providers', label: 'Providers', icon: '🔌' },
  { key: 'channels', path: '/channels', label: 'Channels', icon: '📡' },
  { key: 'skills', path: '/skills', label: 'Skills', icon: '🛠️' },
];

function Sidebar() {
  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="brand-icon">🧠</span>
        <span className="brand-text">Babata</span>
      </div>
      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <NavLink
            key={item.key}
            to={item.path}
            end={item.path === '/'}
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}

function AppContent() {
  return (
    <div className="app">
      <Sidebar />
      <main className="main-content">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/agents" element={<Agents />} />
          <Route path="/providers" element={<Providers />} />
          <Route path="/channels" element={<Channels />} />
          <Route path="/skills" element={<Skills />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  );
}

function App() {
  return (
    <BrowserRouter>
      <AppContent />
    </BrowserRouter>
  );
}

export default App;
