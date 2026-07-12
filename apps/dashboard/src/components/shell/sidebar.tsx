import { Link } from '@tanstack/react-router';
import { CircleHelp, ShieldCheck } from 'lucide-react';

import { Separator } from '@/components/ui/separator';
import { dashboardModuleRegistry } from '@/modules/registry';

export function DashboardSidebar() {
  return (
    <aside aria-label="Dashboard modules" className="dashboard-sidebar" data-desktop-sidebar>
      <Link className="dashboard-brand" search={{ severity: 'all', view: 'runs' }} to="/">
        <span aria-hidden="true" className="dashboard-brand-mark">
          A
        </span>
        <span>
          <strong>ANVIL</strong>
          <small>Anvil Dashboard</small>
        </span>
      </Link>

      <div className="dashboard-workspace">
        <span>Workspace</span>
        <strong>anvil-001</strong>
      </div>

      <nav className="dashboard-nav" aria-label="Primary">
        {dashboardModuleRegistry.manifests.map((manifest) => {
          const Icon = manifest.navigation.icon ?? ShieldCheck;
          return (
            <Link
              activeOptions={{
                exact: manifest.navigation.path === '/',
                includeSearch: false,
              }}
              key={manifest.id}
              search={{ severity: 'all', view: 'runs' }}
              to={manifest.navigation.path}
            >
              <Icon aria-hidden="true" />
              {manifest.navigation.label}
            </Link>
          );
        })}
      </nav>

      <div className="dashboard-sidebar-footer">
        <Separator />
        <ul aria-label="Dashboard connection properties" className="connection-properties">
          <li>
            <span aria-hidden="true" className="status-dot status-dot-green" /> Local only
          </li>
          <li>
            <span aria-hidden="true" className="status-dot" /> Read-only
          </li>
          <li>
            <span aria-hidden="true" className="status-dot" /> No network calls
          </li>
        </ul>
        <p className="dashboard-version">v0.1.0 · Wave 1</p>
        <button
          aria-describedby="dashboard-help-explanation"
          className="dashboard-help"
          disabled
          type="button"
        >
          <CircleHelp aria-hidden="true" />
          <span>
            Help
            <small id="dashboard-help-explanation">Help unavailable in Wave 1</small>
          </span>
        </button>
      </div>
    </aside>
  );
}
