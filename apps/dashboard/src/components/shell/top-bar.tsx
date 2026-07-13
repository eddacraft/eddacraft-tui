import { SyntaxGlyph } from '@/components/brand/syntax-glyph';
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
        <SyntaxGlyph kind="context" />
        <span>SEARCH</span>
        <kbd>⌘ K</kbd>
      </Button>
    </header>
  );
}
