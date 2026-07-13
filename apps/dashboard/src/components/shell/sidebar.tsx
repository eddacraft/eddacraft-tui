import { Link } from '@tanstack/react-router';

import { SyntaxGlyph } from '@/components/brand/syntax-glyph';
import { Separator } from '@/components/ui/separator';
import { dashboardModuleRegistry } from '@/modules/registry';

export function DashboardSidebar() {
  return (
    <aside aria-label="Dashboard modules" className="dashboard-sidebar" data-desktop-sidebar>
      <Link className="dashboard-brand" search={{ severity: 'all', view: 'runs' }} to="/">
        <img
          alt=""
          className="anvil-brandmark"
          height="28"
          src="/anvil-brandmark-ember.svg"
          width="28"
        />
        <span>
          <strong>ANVIL</strong>
          <small>ANVIL // DASHBOARD</small>
        </span>
      </Link>

      <div className="dashboard-workspace">
        <span>Workspace</span>
        <strong>Current workspace</strong>
      </div>

      <nav className="dashboard-nav" aria-label="Primary">
        {dashboardModuleRegistry.manifests.map((manifest) => {
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
              <SyntaxGlyph kind={manifest.navigation.glyph ?? 'context'} />
              {manifest.navigation.label}
            </Link>
          );
        })}
      </nav>

      <div className="dashboard-sidebar-footer">
        <Separator />
        <ul aria-label="Dashboard connection properties" className="connection-properties">
          <li>
            <span aria-hidden="true" className="connection-status connection-status-ok">
              [ OK ]
            </span>{' '}
            Local only
          </li>
          <li>
            <span aria-hidden="true" className="connection-status">
              [ ]
            </span>{' '}
            Read-only
          </li>
          <li>
            <span aria-hidden="true" className="connection-status">
              [ ]
            </span>{' '}
            Local loopback API
          </li>
        </ul>
        <p className="dashboard-version">v0.1.0 · Wave 1</p>
        <button
          aria-describedby="dashboard-help-explanation"
          className="dashboard-help"
          disabled
          type="button"
        >
          <SyntaxGlyph kind="unavailable" />
          <span>
            Help
            <small id="dashboard-help-explanation">Help unavailable in Wave 1</small>
          </span>
        </button>
      </div>
    </aside>
  );
}
