import Link from 'next/link';

// Tombstone (GHCLIAUTH-007, ADR-066 decision 5): browser activation is
// retired — the page must keep resolving (no 404) because outstanding
// invite emails and already-shipped CLIs still link here, with or without
// a ?code= parameter. Activation now happens entirely in the terminal via
// the GitHub device flow or --otp.
export default function ActivatePage() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-void font-mono text-text-primary">
      <div className="w-full max-w-md px-6">
        {/* Header */}
        <div className="mb-8">
          <Link
            href="/"
            className="text-xs text-text-muted transition-colors hover:text-text-primary"
          >
            {'<-'} back to anvil
          </Link>
        </div>

        <h1 className="mb-8 text-sm text-text-muted">
          <span className="text-edda">$</span> anvil :: activate
        </h1>

        <div className="space-y-6">
          <p className="text-sm text-edda">Activation has moved into the CLI.</p>
          <p className="text-xs text-text-muted">
            If you received an activation code by email, it is no longer valid &mdash; sign in from
            your terminal instead:
          </p>

          <div className="border border-structure bg-surface px-3 py-2">
            <p className="font-mono text-sm">$ anvil auth login</p>
          </div>
          <p className="text-xs text-text-muted">
            You&apos;ll be shown a short code and a github.com link &mdash; open it on any device
            and approve to finish signing in with GitHub.
          </p>

          <div className="border border-structure bg-surface px-3 py-2">
            <p className="font-mono text-sm">$ anvil auth login --otp</p>
          </div>
          <p className="text-xs text-text-muted">
            No GitHub account? The --otp flag sends a one-time code to your invited email address.
          </p>
        </div>
      </div>
    </main>
  );
}
