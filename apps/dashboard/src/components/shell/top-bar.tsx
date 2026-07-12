import { Search } from 'lucide-react';

import { Button } from '@/components/ui/button';

interface TopBarProps {
  onSearch: () => void;
}

export function TopBar({ onSearch }: TopBarProps) {
  return (
    <header className="dashboard-topbar">
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
