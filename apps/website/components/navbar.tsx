'use client';

import Link from 'next/link';
import { scrollToWaitlist } from '@/lib/scroll';

export function Navbar() {
  return (
    <nav className="fixed inset-x-0 top-0 z-50 h-14 border-b border-structure bg-void">
      <div className="site-container flex h-full items-center justify-between font-mono text-xs">
        <div className="flex h-full items-center gap-6 sm:gap-8">
          <Link href="/" className="text-edda transition-colors hover:text-off-white">
            eddacraft
          </Link>
          <a
            href="#product"
            className="flex h-full items-center border-b-2 border-anvil text-anvil transition-colors hover:text-off-white"
          >
            anvil
          </a>
        </div>

        <div className="flex items-center gap-4 sm:gap-6">
          <Link
            href="https://docs.eddacraft.ai/anvil/overview"
            className="hidden text-ghost-grey transition-colors hover:text-off-white sm:inline"
          >
            docs
          </Link>
          <Link
            href="/security"
            className="hidden text-ghost-grey transition-colors hover:text-off-white md:inline"
          >
            security
          </Link>
          <button
            type="button"
            onClick={scrollToWaitlist}
            className="border border-anvil bg-anvil/5 px-3 py-2 text-anvil transition-colors hover:bg-anvil/10 sm:px-4"
          >
            [ = ] request access
          </button>
        </div>
      </div>
    </nav>
  );
}
