import Link from 'next/link';

export const metadata = {
  title: 'Security — anvil by eddacraft',
  description:
    'Security practices and product security features for anvil, the AI governance tool by eddacraft.',
};

export default function SecurityPage() {
  return (
    <main className="min-h-screen bg-void font-mono text-text-primary">
      {/* Header */}
      <header className="border-b border-structure">
        <div className="mx-auto max-w-4xl px-6 py-4">
          <Link
            href="/"
            className="text-text-muted hover:text-text-primary transition-colors text-sm"
          >
            {'<-'} back to anvil
          </Link>
        </div>
      </header>

      {/* Man Page Content */}
      <div className="mx-auto max-w-4xl px-6 py-12 sm:py-16">
        {/* Man Page Header */}
        <div className="flex justify-between items-center text-text-muted text-xs sm:text-sm mb-8 border-b border-structure pb-4">
          <span>ANVIL-SECURITY(7)</span>
          <span>eddacraft Manual</span>
          <span>ANVIL-SECURITY(7)</span>
        </div>

        <div className="space-y-8 text-sm leading-relaxed">
          {/* NAME */}
          <section>
            <h2 className="text-anvil font-bold mb-2">NAME</h2>
            <p className="text-text-muted pl-6">
              anvil-security - security practices and product security features
            </p>
          </section>

          {/* SYNOPSIS */}
          <section>
            <h2 className="text-anvil font-bold mb-2">SYNOPSIS</h2>
            <p className="text-text-muted pl-6">
              This document describes how anvil secures your code, how we secure anvil itself, and
              how to report security vulnerabilities.
            </p>
          </section>

          {/* PRODUCT SECURITY */}
          <section>
            <h2 className="text-anvil font-bold mb-2">PRODUCT SECURITY FEATURES</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                anvil provides the following security capabilities for your development workflow:
              </p>

              <div>
                <p className="text-text-primary mb-1">Secret Detection</p>
                <p className="pl-4">
                  Scans generated code for exposed credentials, API keys, tokens, and other secrets
                  before they reach your codebase. Patterns are updated regularly.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Static Analysis (SAST)</p>
                <p className="pl-4">
                  Identifies common security vulnerabilities in generated code including injection
                  flaws, insecure configurations, and unsafe patterns.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Dependency Auditing</p>
                <p className="pl-4">
                  Validates dependencies against known vulnerability databases before they are added
                  to your project.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Policy Enforcement</p>
                <p className="pl-4">
                  Define custom security policies in Rego. Block patterns, require reviews, or flag
                  violations based on your organisation&apos;s standards.
                </p>
              </div>
            </div>
          </section>

          {/* HOW WE SECURE ANVIL */}
          <section>
            <h2 className="text-anvil font-bold mb-2">HOW WE SECURE ANVIL</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <div>
                <p className="text-text-primary mb-1">Local-First Architecture</p>
                <p className="pl-4">
                  anvil runs entirely on your machine. Your source code, policies, and AI outputs
                  never leave your infrastructure unless you explicitly configure remote features.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Deterministic Core</p>
                <p className="pl-4">
                  The governance engine is deterministic and contains no AI components. Policy
                  evaluation produces the same result every time for the same input.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Signed Releases</p>
                <p className="pl-4">
                  All CLI releases are cryptographically signed. Verify signatures with{' '}
                  <span className="text-anvil">anvil verify</span> or check against our public key.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Dependency Minimisation</p>
                <p className="pl-4">
                  The core CLI has minimal dependencies. We audit the full dependency tree and pin
                  versions to prevent supply chain attacks.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">No Telemetry by Default</p>
                <p className="pl-4">
                  Anonymous telemetry is opt-in. When enabled, it collects only aggregate
                  performance metrics, never code or content.
                </p>
              </div>
            </div>
          </section>

          {/* INFRASTRUCTURE */}
          <section>
            <h2 className="text-anvil font-bold mb-2">INFRASTRUCTURE</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <div>
                <p className="text-text-primary mb-1">Hosting</p>
                <p className="pl-4">
                  Web services are hosted on Vercel with automatic DDoS protection and edge caching.
                  Backend services run on isolated infrastructure in the EU and US.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Encryption</p>
                <p className="pl-4">
                  All data in transit uses TLS 1.3. Data at rest is encrypted using AES-256.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Access Control</p>
                <p className="pl-4">
                  Internal access follows principle of least privilege. All access is logged and
                  regularly audited.
                </p>
              </div>
            </div>
          </section>

          {/* RESPONSIBLE DISCLOSURE */}
          <section>
            <h2 className="text-anvil font-bold mb-2">RESPONSIBLE DISCLOSURE</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                We take security vulnerabilities seriously. If you discover a security issue, please
                report it responsibly.
              </p>

              <div>
                <p className="text-text-primary mb-1">How to Report</p>
                <p className="pl-4">
                  Email{' '}
                  <a href="mailto:security@eddacraft.ai" className="text-anvil hover:underline">
                    security@eddacraft.ai
                  </a>{' '}
                  with details of the vulnerability. Include steps to reproduce if possible.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">What to Expect</p>
                <ul className="pl-4 space-y-1">
                  <li>
                    <span className="text-edda">-</span> Acknowledgement within 48 hours
                  </li>
                  <li>
                    <span className="text-edda">-</span> Initial assessment within 5 business days
                  </li>
                  <li>
                    <span className="text-edda">-</span> Regular updates on remediation progress
                  </li>
                  <li>
                    <span className="text-edda">-</span> Credit in our security advisories (if
                    desired)
                  </li>
                </ul>
              </div>

              <div>
                <p className="text-text-primary mb-1">Scope</p>
                <p className="pl-4">
                  In scope: anvil CLI, web properties (*.eddacraft.ai), API endpoints. Out of scope:
                  third-party services, social engineering, physical attacks.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Safe Harbour</p>
                <p className="pl-4">
                  We will not pursue legal action against researchers who act in good faith and
                  follow responsible disclosure practices.
                </p>
              </div>
            </div>
          </section>

          {/* SEE ALSO */}
          <section>
            <h2 className="text-anvil font-bold mb-2">SEE ALSO</h2>
            <div className="text-text-muted pl-6">
              <p>
                <Link href="/" className="text-anvil hover:underline">
                  anvil(1)
                </Link>
                ,{' '}
                <Link href="/privacy" className="text-anvil hover:underline">
                  anvil-privacy(7)
                </Link>
                ,{' '}
                <span
                  className="text-text-muted cursor-default"
                  aria-label="anvil-terms(7) (coming soon)"
                >
                  anvil-terms(7)
                </span>
              </p>
            </div>
          </section>

          {/* AUTHOR */}
          <section>
            <h2 className="text-anvil font-bold mb-2">AUTHOR</h2>
            <div className="text-text-muted pl-6">
              <p>eddacraft, Inc.</p>
            </div>
          </section>
        </div>

        {/* Man Page Footer */}
        <div className="flex justify-between items-center text-text-muted text-xs sm:text-sm mt-12 border-t border-structure pt-4">
          <span>eddacraft</span>
          <span>January 2026</span>
          <span>ANVIL-SECURITY(7)</span>
        </div>
      </div>
    </main>
  );
}
