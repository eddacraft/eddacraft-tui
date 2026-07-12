import { Outlet } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import { CommandPalette } from '@/components/command-palette';
import { MobileNavigation } from '@/components/shell/mobile-navigation';
import { DashboardSidebar } from '@/components/shell/sidebar';
import { TopBar } from '@/components/shell/top-bar';

export function DashboardShell() {
  const [searchOpen, setSearchOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setSearchOpen((open) => !open);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <>
      <a className="dashboard-skip-link" href="#main-content">
        Skip to dashboard content
      </a>
      <div className="dashboard-root" data-dashboard-root>
        <DashboardSidebar />
        <div className="dashboard-content-shell">
          <MobileNavigation onSearch={() => setSearchOpen(true)} />
          <TopBar onSearch={() => setSearchOpen(true)} />
          <main className="dashboard-stage" id="main-content" tabIndex={-1}>
            <Outlet />
          </main>
        </div>
      </div>
      <CommandPalette onOpenChange={setSearchOpen} open={searchOpen} />
    </>
  );
}
