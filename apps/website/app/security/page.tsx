import Link from 'next/link';

export const metadata = {
  title: 'Security — anvil by eddacraft',
  description:
    'What anvil checks in AI-assisted code, how eddacraft builds and ships anvil, and how to report a vulnerability.',
};

const DOCS_SECURITY = 'https://docs.eddacraft.ai/anvil/operations/security';
const DOCS_TELEMETRY = 'https://docs.eddacraft.ai/anvil/operations/telemetry';

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
              anvil-security - what anvil checks, how anvil is built and shipped, and how to report
              a vulnerability
            </p>
          </section>

          {/* SYNOPSIS */}
          <section>
            <h2 className="text-anvil font-bold mb-2">SYNOPSIS</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                This page describes what anvil checks in your code, how anvil itself is built and
                shipped, and how to report a security issue.
              </p>
              <p>
                Product behaviour is documented in detail at{' '}
                <a href={DOCS_SECURITY} className="text-anvil hover:underline">
                  docs.eddacraft.ai
                </a>
                . Where this page and the documentation differ, the documentation is current.
              </p>
            </div>
          </section>

          {/* WHAT ANVIL CHECKS */}
          <section>
            <h2 className="text-anvil font-bold mb-2">WHAT ANVIL CHECKS</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                anvil evaluates a proposed change on your machine, before it lands and again at the
                gate. The checks below ship in the current release.
              </p>

              <div>
                <p className="text-text-primary mb-1">Secret Detection</p>
                <p className="pl-4">
                  Scans a proposed write for credential-like tokens before it reaches the diff, and
                  scans the change set again in the gate. Lines that could not be scanned are
                  reported as failures, never assumed clean.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Command Safety</p>
                <p className="pl-4">
                  Destructive or unexpected shell commands issued by an agent are stopped at the
                  intercept point when enforcement is enabled.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Anti-Pattern Scan</p>
                <p className="pl-4">
                  Known unsafe shapes in generated code are reported against a baseline of the
                  existing repository, so new violations are surfaced and old ones are not
                  re-litigated.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Architecture Boundaries</p>
                <p className="pl-4">
                  Layer and package import rules you define are held at save-time.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Policy</p>
                <p className="pl-4">
                  Write your own rules, including Rego policies evaluated with{' '}
                  <span className="text-anvil">anvil policy eval</span>. Evaluation is
                  deterministic: the same input produces the same result every time.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Default Posture</p>
                <p className="pl-4">
                  By default anvil warns and exits zero. Blocking a write, or failing a check on
                  warnings, is a choice the operator turns on.
                </p>
              </div>
            </div>
          </section>

          {/* HOW WE SECURE ANVIL */}
          <section>
            <h2 className="text-anvil font-bold mb-2">HOW WE SECURE ANVIL</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <div>
                <p className="text-text-primary mb-1">Local-First</p>
                <p className="pl-4">
                  Checks run on your machine. Source code is not uploaded to run them. Network
                  access is used for installation, sign-in, licence refresh, updates, feedback you
                  choose to send, and the anonymous usage beacon described below.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">AI-Client Egress</p>
                <p className="pl-4">
                  Graph context shared with a connected AI client is identity-only by default:
                  symbol names, locations and relationships, not source. Sending a source snippet
                  requires an explicit request from the client and per-workspace consent, and{' '}
                  <span className="text-anvil">ANVIL_GCTX_EGRESS=0</span> keeps it off regardless.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Deterministic Core</p>
                <p className="pl-4">
                  The policy engine contains no model. AI may explain a finding or propose a fix; it
                  does not decide whether a change passes.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Signed Releases</p>
                <p className="pl-4">
                  Installer artefacts for each tagged release are signed with minisign, and the
                  detached signatures are published alongside the release on GitHub.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Anonymous Telemetry</p>
                <p className="pl-4">
                  anvil sends a narrow anonymous usage beacon, at most once per installation per
                  day, and only after an interactive first run has shown its notice. It never
                  includes source, paths, command arguments, findings or free text. Turn it off with{' '}
                  <span className="text-anvil">anvil telemetry off</span>,{' '}
                  <span className="text-anvil">ANVIL_TELEMETRY=off</span> or{' '}
                  <span className="text-anvil">DO_NOT_TRACK=1</span>. The complete payload is
                  documented{' '}
                  <a href={DOCS_TELEMETRY} className="text-anvil hover:underline">
                    here
                  </a>
                  .
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
                  The website, the documentation site and the early-access API run on Vercel.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Data</p>
                <p className="pl-4">
                  Waitlist and account records are stored in a managed Postgres database. Data in
                  transit uses TLS; data at rest is encrypted by the database provider.
                </p>
              </div>

              <div>
                <p className="text-text-primary mb-1">Access</p>
                <p className="pl-4">Access to production systems follows least privilege.</p>
              </div>
            </div>
          </section>

          {/* RESPONSIBLE DISCLOSURE */}
          <section>
            <h2 className="text-anvil font-bold mb-2">RESPONSIBLE DISCLOSURE</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                If you find a security issue in anvil or in an eddacraft service, please report it
                to us before disclosing it publicly.
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
                    <span className="text-edda">-</span> Updates as remediation progresses
                  </li>
                  <li>
                    <span className="text-edda">-</span> Credit in the release notes, if you want it
                  </li>
                </ul>
              </div>

              <div>
                <p className="text-text-primary mb-1">Scope</p>
                <p className="pl-4">
                  In scope: the anvil CLI and daemon, web properties under *.eddacraft.ai, and the
                  early-access API. Out of scope: third-party services, social engineering, physical
                  attacks.
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
                <a href={DOCS_SECURITY} className="text-anvil hover:underline">
                  local data and security
                </a>
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
              <p>eddacraft</p>
            </div>
          </section>
        </div>

        {/* Man Page Footer */}
        <div className="flex justify-between items-center text-text-muted text-xs sm:text-sm mt-12 border-t border-structure pt-4">
          <span>eddacraft</span>
          <span>September 2026</span>
          <span>ANVIL-SECURITY(7)</span>
        </div>
      </div>
    </main>
  );
}
