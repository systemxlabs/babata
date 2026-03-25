const navigationItems = ['Overview', 'Tasks', 'Create', 'System'];

export function App() {
  return (
    <div className="app-shell">
      <div className="app-shell__glow app-shell__glow--left" />
      <div className="app-shell__glow app-shell__glow--right" />
      <header className="hero-panel">
        <p className="hero-panel__eyebrow">Babata dashboard</p>
        <h1>Local task control plane</h1>
        <p className="hero-panel__copy">
          The embedded dashboard shell is wired up. API integration and real task
          views land in later tasks.
        </p>
      </header>

      <main className="dashboard-grid">
        <nav aria-label="Primary" className="dashboard-card">
          <p className="dashboard-card__label">Navigation</p>
          <ul className="nav-list">
            {navigationItems.map((item) => (
              <li key={item} className="nav-list__item">
                <span>{item}</span>
              </li>
            ))}
          </ul>
        </nav>

        <section className="dashboard-card dashboard-card--status">
          <p className="dashboard-card__label">Status</p>
          <h2>Shell ready</h2>
          <p>
            Static assets are bundled with Rust and only served as HTML for
            browser-style requests.
          </p>
        </section>
      </main>
    </div>
  );
}
