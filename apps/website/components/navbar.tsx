"use client"

import Link from "next/link"

export function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 h-14 border-b border-structure bg-void">
      <div className="mx-auto flex h-full max-w-6xl items-center justify-between px-4 sm:px-6">
        <Link href="/" className="flex items-center gap-2 font-mono text-sm text-text-primary shrink-0">
          <img 
            src="/images/eddacraft-brandmark-white.svg" 
            alt="eddacraft" 
            width={18} 
            height={18}
          />
          <span className="hidden sm:inline">eddacraft</span>
        </Link>
        
        <div className="flex items-center gap-4 sm:gap-8 font-mono text-xs sm:text-sm">
          <Link 
            href="#" 
            className="text-anvil transition-colors hover:text-text-primary"
          >
            Anvil
          </Link>
          <Link 
            href="#" 
            className="text-text-muted transition-colors hover:text-text-primary hidden sm:inline"
          >
            Edda
          </Link>
          <Link 
            href="#" 
            className="text-text-muted transition-colors hover:text-text-primary"
          >
            Docs
          </Link>
          <Link 
            href="#" 
            className="text-text-muted transition-colors hover:text-text-primary"
          >
            Login
          </Link>
        </div>
      </div>
    </nav>
  )
}
