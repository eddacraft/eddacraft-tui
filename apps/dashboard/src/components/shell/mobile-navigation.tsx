import { Link } from '@tanstack/react-router';
import { ChevronDown, CircleHelp, Menu, Search, ShieldCheck } from 'lucide-react';

import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { dashboardModuleRegistry } from '@/modules/registry';

interface MobileNavigationProps {
  onSearch: () => void;
}

export function MobileNavigation({ onSearch }: MobileNavigationProps) {
  return (
    <>
      <header className="dashboard-mobile-header" data-mobile-header>
        <Link
          aria-label="Anvil Dashboard home"
          className="mobile-brand"
          search={{ severity: 'all', view: 'runs' }}
          to="/"
        >
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
                {dashboardModuleRegistry.manifests.map((manifest) => {
                  const Icon = manifest.navigation.icon ?? ShieldCheck;
                  return (
                    <Link
                      key={manifest.id}
                      search={{ severity: 'all', view: 'runs' }}
                      to={manifest.navigation.path}
                    >
                      <Icon aria-hidden="true" /> {manifest.navigation.label}
                    </Link>
                  );
                })}
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
        {dashboardModuleRegistry.manifests.map((manifest) => {
          const Icon = manifest.navigation.icon ?? ShieldCheck;
          return (
            <Link
              key={manifest.id}
              search={{ severity: 'all', view: 'runs' }}
              to={manifest.navigation.path}
            >
              <Icon aria-hidden="true" />
              {manifest.navigation.label}
            </Link>
          );
        })}
        <button type="button">
          <CircleHelp aria-hidden="true" />
          Help
        </button>
      </nav>
    </>
  );
}
