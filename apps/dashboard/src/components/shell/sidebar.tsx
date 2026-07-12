import { Link } from '@tanstack/react-router';
import { CircleHelp, FileText, ShieldCheck } from 'lucide-react';

import { Separator } from '@/components/ui/separator';

const exactRouteMatch = { exact: true } as const;

export function DashboardSidebar() {
  return (
    <aside aria-label="Dashboard modules" className="dashboard-sidebar" data-desktop-sidebar>
      <Link className="dashboard-brand" to="/">
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
        <Link activeOptions={exactRouteMatch} to="/">
          <ShieldCheck aria-hidden="true" />
          Protection
        </Link>
        <span aria-disabled="true" className="dashboard-nav-disabled">
          <FileText aria-hidden="true" />
          Plans
          <small>soon</small>
        </span>
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
        <button className="dashboard-help" type="button">
          <CircleHelp aria-hidden="true" /> Help
        </button>
      </div>
    </aside>
  );
}
