import { Link } from '@tanstack/react-router';
import { ChevronDown, CircleHelp, FileText, Menu, Search, ShieldCheck } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';

interface MobileNavigationProps {
  onSearch: () => void;
}

export function MobileNavigation({ onSearch }: MobileNavigationProps) {
  return (
    <>
      <header className="dashboard-mobile-header" data-mobile-header>
        <Link aria-label="Anvil Dashboard home" className="mobile-brand" to="/">
          <span aria-hidden="true" className="dashboard-brand-mark">
            A
          </span>
          <strong>ANVIL</strong>
        </Link>
        <span className="mobile-workspace">
          anvil-001 <ChevronDown aria-hidden="true" />
        </span>
        <div className="mobile-header-actions">
          <Button
            aria-label="Search dashboard"
            onClick={onSearch}
            size="icon-sm"
            type="button"
            variant="ghost"
          >
            <Search aria-hidden="true" />
          </Button>
          <Sheet>
            <SheetTrigger asChild>
              <Button aria-label="Open navigation" size="icon-sm" type="button" variant="ghost">
                <Menu aria-hidden="true" />
              </Button>
            </SheetTrigger>
            <SheetContent className="mobile-sheet" side="right">
              <SheetHeader>
                <SheetTitle>ANVIL</SheetTitle>
                <SheetDescription>Local workspace dashboard</SheetDescription>
              </SheetHeader>
              <nav aria-label="Mobile menu" className="mobile-sheet-nav">
                <Link to="/">
                  <ShieldCheck aria-hidden="true" /> Protection
                </Link>
                <span aria-disabled="true">
                  <FileText aria-hidden="true" /> Plans <small>soon</small>
                </span>
                <button type="button">
                  <CircleHelp aria-hidden="true" /> Help
                </button>
              </nav>
            </SheetContent>
          </Sheet>
        </div>
      </header>

      <nav
        aria-label="Mobile dashboard modules"
        className="mobile-bottom-nav"
        data-mobile-bottom-nav
      >
        <Link to="/">
          <ShieldCheck aria-hidden="true" />
          Protection
        </Link>
        <span aria-disabled="true">
          <FileText aria-hidden="true" />
          Plans
        </span>
        <button type="button">
          <CircleHelp aria-hidden="true" />
          Help
        </button>
      </nav>
    </>
  );
}
