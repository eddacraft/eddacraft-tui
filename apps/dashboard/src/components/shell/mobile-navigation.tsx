import { Link } from '@tanstack/react-router';

import { SyntaxGlyph } from '@/components/brand/syntax-glyph';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetClose,
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
          aria-label="anvil dashboard home"
          className="mobile-brand"
          search={{ severity: 'all', view: 'runs' }}
          to="/"
        >
          <img
            alt=""
            className="anvil-brandmark"
            height="26"
            src="/anvil-brandmark-ember.svg"
            width="26"
          />
          <strong>ANVIL</strong>
        </Link>
        <span className="mobile-workspace">Current workspace</span>
        <div className="mobile-header-actions">
          <Button
            aria-label="Search dashboard"
            onClick={onSearch}
            size="icon-sm"
            type="button"
            variant="ghost"
          >
            <SyntaxGlyph kind="context" />
          </Button>
          <Sheet>
            <SheetTrigger asChild>
              <Button aria-label="Open navigation" size="icon-sm" type="button" variant="ghost">
                <SyntaxGlyph kind="history" />
              </Button>
            </SheetTrigger>
            <SheetContent className="mobile-sheet" side="right">
              <SheetHeader>
                <SheetTitle>ANVIL // DASHBOARD</SheetTitle>
                <SheetDescription>Local workspace dashboard</SheetDescription>
              </SheetHeader>
              <nav aria-label="Mobile menu" className="mobile-sheet-nav">
                {dashboardModuleRegistry.manifests.map((manifest) => {
                  return (
                    <SheetClose asChild key={manifest.id}>
                      <Link
                        activeOptions={{
                          exact: manifest.navigation.path === '/',
                          includeSearch: false,
                        }}
                        search={{ severity: 'all', view: 'runs' }}
                        to={manifest.navigation.path}
                      >
                        <SyntaxGlyph kind={manifest.navigation.glyph ?? 'context'} />{' '}
                        {manifest.navigation.label}
                      </Link>
                    </SheetClose>
                  );
                })}
                <button disabled title="Help is unavailable in Wave 1" type="button">
                  <SyntaxGlyph kind="unavailable" /> Help unavailable in Wave 1
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
        <button disabled title="Help is unavailable in Wave 1" type="button">
          <SyntaxGlyph kind="unavailable" />
          Help unavailable
        </button>
      </nav>
    </>
  );
}
