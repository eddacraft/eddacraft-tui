import { Search } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { WorkspaceSwitcher } from '@/components/shell/workspace-switcher';
import { workspace } from '@/modules/protection/fixture';

interface TopBarProps {
  onSearch: () => void;
}

export function TopBar({ onSearch }: TopBarProps) {
  return (
    <header className="dashboard-topbar">
      <WorkspaceSwitcher refreshedAt={workspace.refreshedAt} root={workspace.root} />
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
