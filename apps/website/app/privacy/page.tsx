import Link from 'next/link';

export const metadata = {
  title: 'Privacy Policy — anvil by eddacraft',
  description: 'Privacy policy for anvil, the AI governance tool by eddacraft.',
};

export default function PrivacyPage() {
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
          <span>ANVIL-PRIVACY(7)</span>
          <span>eddacraft Manual</span>
          <span>ANVIL-PRIVACY(7)</span>
        </div>

        <div className="space-y-8 text-sm leading-relaxed">
          {/* NAME */}
          <section>
            <h2 className="text-anvil font-bold mb-2">NAME</h2>
            <p className="text-text-muted pl-6">
              anvil-privacy - privacy policy for anvil services
            </p>
          </section>

          {/* SYNOPSIS */}
          <section>
            <h2 className="text-anvil font-bold mb-2">SYNOPSIS</h2>
            <p className="text-text-muted pl-6">
              This document describes what data eddacraft collects, how we use it, and your rights
              regarding that data when using anvil.
            </p>
          </section>

          {/* DESCRIPTION */}
          <section>
            <h2 className="text-anvil font-bold mb-2">DESCRIPTION</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                anvil is a developer tool for AI governance. We are committed to protecting your
                privacy and being transparent about the data we handle.
              </p>
              <p>
                We operate under a principle of{' '}
                <span className="text-text-primary">data minimisation</span>: we only collect what
                is necessary to provide the service.
              </p>
            </div>
          </section>

          {/* DATA COLLECTION */}
          <section>
            <h2 className="text-anvil font-bold mb-2">DATA COLLECTION</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <div>
                <p className="text-text-primary mb-1">Waitlist Registration</p>
                <p className="pl-4">
                  When you request access, we collect your email address. This is stored securely
                  and used solely to communicate about your access status and product updates.
                </p>
              </div>
              <div>
                <p className="text-text-primary mb-1">Usage Analytics</p>
                <p className="pl-4">
                  We collect anonymised, aggregated usage data to improve the product. This does not
                  include your source code, prompts, or any content you process through anvil.
                </p>
              </div>
              <div>
                <p className="text-text-primary mb-1">CLI Telemetry</p>
                <p className="pl-4">
                  The anvil CLI may collect anonymous performance metrics. Telemetry can be disabled
                  via <span className="text-anvil">anvil config set telemetry false</span>.
                </p>
              </div>
            </div>
          </section>

          {/* WHAT WE DO NOT COLLECT */}
          <section>
            <h2 className="text-anvil font-bold mb-2">WHAT WE DO NOT COLLECT</h2>
            <div className="text-text-muted pl-6">
              <ul className="space-y-2">
                <li>
                  <span className="text-edda">-</span> Your source code or generated code
                </li>
                <li>
                  <span className="text-edda">-</span> AI prompts or responses
                </li>
                <li>
                  <span className="text-edda">-</span> Policy rule definitions
                </li>
                <li>
                  <span className="text-edda">-</span> Repository contents or git history
                </li>
                <li>
                  <span className="text-edda">-</span> Personal information beyond email for
                  waitlist
                </li>
              </ul>
            </div>
          </section>

          {/* DATA STORAGE */}
          <section>
            <h2 className="text-anvil font-bold mb-2">DATA STORAGE</h2>
            <div className="text-text-muted pl-6 space-y-4">
              <p>
                Data is stored on servers located in the European Union and the United States. We
                use industry-standard encryption for data at rest and in transit.
              </p>
            </div>
          </section>

          {/* YOUR RIGHTS */}
          <section>
            <h2 className="text-anvil font-bold mb-2">YOUR RIGHTS</h2>
            <div className="text-text-muted pl-6 space-y-2">
              <p>You have the right to:</p>
              <ul className="space-y-2 pl-4">
                <li>
                  <span className="text-edda">-</span> Request access to your personal data
                </li>
                <li>
                  <span className="text-edda">-</span> Request deletion of your data
                </li>
                <li>
                  <span className="text-edda">-</span> Withdraw consent at any time
                </li>
                <li>
                  <span className="text-edda">-</span> Lodge a complaint with a supervisory
                  authority
                </li>
              </ul>
              <p className="mt-4">
                To exercise these rights, contact us at{' '}
                <a href="mailto:privacy@eddacraft.ai" className="text-anvil hover:underline">
                  privacy@eddacraft.ai
                </a>
                .
              </p>
            </div>
          </section>

          {/* COOKIES */}
          <section>
            <h2 className="text-anvil font-bold mb-2">COOKIES</h2>
            <div className="text-text-muted pl-6">
              <p>
                We use essential cookies only. No third-party tracking or advertising cookies are
                used on this site.
              </p>
            </div>
          </section>

          {/* CHANGES */}
          <section>
            <h2 className="text-anvil font-bold mb-2">CHANGES</h2>
            <div className="text-text-muted pl-6">
              <p>
                We may update this policy from time to time. Significant changes will be
                communicated via email to registered users.
              </p>
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
                <Link href="/security" className="text-anvil hover:underline">
                  anvil-security(7)
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
          <span>ANVIL-PRIVACY(7)</span>
        </div>
      </div>
    </main>
  );
}
