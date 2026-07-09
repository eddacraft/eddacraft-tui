import { Outlet } from '@tanstack/react-router';

export function DashboardRootRoute() {
  return (
    <main className="dashboard-root" data-dashboard-root>
      <aside className="dashboard-sidebar" aria-label="Dashboard modules">
        <a className="dashboard-brand" href="/">
          <span aria-hidden="true" className="dashboard-brand-mark" />
          <span>Anvil Dashboard</span>
        </a>
        <nav className="dashboard-nav" aria-label="Primary">
          <a aria-current="page" href="/">
            Protection overview
          </a>
        </nav>
      </aside>
      <section className="dashboard-stage">
        <Outlet />
      </section>
    </main>
  );
}
