import { BrowserRouter, Routes, Route, Link, useLocation } from 'react-router-dom';
import { Tasks } from './pages/Tasks/Tasks';
import './App.css';

// 导航组件
function Navigation() {
  const location = useLocation();
  
  return (
    <nav className="main-nav">
      <div className="nav-brand">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z" />
          <path d="M2 17l10 5 10-5" />
          <path d="M2 12l10 5 10-5" />
        </svg>
        <span>Babata</span>
      </div>
      <div className="nav-links">
        <Link 
          to="/" 
          className={`nav-link ${location.pathname === '/' ? 'active' : ''}`}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect width="7" height="9" x="3" y="3" rx="1" />
            <rect width="7" height="5" x="14" y="3" rx="1" />
            <rect width="7" height="9" x="14" y="12" rx="1" />
            <rect width="7" height="5" x="3" y="16" rx="1" />
          </svg>
          概览
        </Link>
        <Link 
          to="/tasks" 
          className={`nav-link ${location.pathname.startsWith('/tasks') ? 'active' : ''}`}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M9 11l3 3L22 4" />
            <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />
          </svg>
          任务
        </Link>
        <Link 
          to="/agents" 
          className={`nav-link ${location.pathname.startsWith('/agents') ? 'active' : ''}`}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" />
            <path d="M12 2v2" />
            <path d="M12 20v2" />
            <path d="m4.93 4.93 1.41 1.41" />
            <path d="m17.66 17.66 1.41 1.41" />
            <path d="M2 12h2" />
            <path d="M20 12h2" />
            <path d="m6.34 17.66-1.41 1.41" />
            <path d="m19.07 4.93-1.41 1.41" />
          </svg>
          智能体
        </Link>
        <Link 
          to="/channels" 
          className={`nav-link ${location.pathname.startsWith('/channels') ? 'active' : ''}`}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          频道
        </Link>
      </div>
    </nav>
  );
}

// 概览页面
function Home() {
  return (
    <div className="home-page">
      <h1>Babata 任务管理系统</h1>
      <p>欢迎使用 Babata 多智能体任务管理系统</p>
      <div className="quick-links">
        <a href="/tasks" className="quick-link">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M9 11l3 3L22 4" />
            <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" />
          </svg>
          查看任务列表
        </a>
      </div>
    </div>
  );
}

// 智能体页面（占位）
function Agents() {
  return (
    <div className="page-placeholder">
      <h1>智能体管理</h1>
      <p>智能体管理功能开发中...</p>
    </div>
  );
}

// 频道页面（占位）
function Channels() {
  return (
    <div className="page-placeholder">
      <h1>频道管理</h1>
      <p>频道管理功能开发中...</p>
    </div>
  );
}

function App() {
  return (
    <BrowserRouter>
      <div className="app">
        <Navigation />
        <main className="main-content">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/tasks" element={<Tasks />} />
            <Route path="/agents" element={<Agents />} />
            <Route path="/channels" element={<Channels />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
