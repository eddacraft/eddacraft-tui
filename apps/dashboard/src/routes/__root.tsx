import { Link, Outlet } from '@tanstack/react-router';

const exactRouteMatch = { exact: true } as const;

export function DashboardRootRoute() {
  return (
    <>
      <a className="dashboard-skip-link" href="#main-content">
        Skip to dashboard content
      </a>
      <div className="dashboard-root" data-dashboard-root>
        <aside className="dashboard-sidebar" aria-label="Dashboard modules">
          <Link className="dashboard-brand" to="/">
            <span aria-hidden="true" className="dashboard-brand-mark" />
            <span>Anvil Dashboard</span>
          </Link>
          <nav className="dashboard-nav" aria-label="Primary">
            <Link activeOptions={exactRouteMatch} to="/">
              Protection overview
            </Link>
          </nav>
        </aside>
        <main className="dashboard-stage" id="main-content" tabIndex={-1}>
          <Outlet />
        </main>
      </div>
    </>
  );
}
