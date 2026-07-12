import { Search } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { workspace } from '@/modules/protection/fixture';

interface TopBarProps {
  onSearch: () => void;
}

export function TopBar({ onSearch }: TopBarProps) {
  return (
    <header className="dashboard-topbar">
      <dl className="topbar-context">
        <div>
          <dt>Workspace root</dt>
          <dd>{workspace.root}</dd>
        </div>
        <div>
          <dt>Last refreshed</dt>
          <dd>{workspace.refreshedAt}</dd>
        </div>
      </dl>
      <Button
        aria-label="Search dashboard"
        className="topbar-search"
        onClick={onSearch}
        type="button"
        variant="outline"
      >
        <Search aria-hidden="true" />
        <span>Search</span>
        <kbd>⌘ K</kbd>
      </Button>
    </header>
  );
}
