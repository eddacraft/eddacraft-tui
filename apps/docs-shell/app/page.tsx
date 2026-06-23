export const dynamic = 'force-dynamic';

const COPYRIGHT_YEAR = new Date().getFullYear();

type SectionAccent = 'structure' | 'anvil';

const sections: Array<{
  title: string;
  bracket: string;
  description: string;
  href: string;
  accent: SectionAccent;
  ariaLabel: string;
}> = [
  {
    title: 'APS',
    bracket: '[ ]',
    description: 'Plan specification, schemas, validation, and implementation guidance.',
    href: '/aps/overview',
    accent: 'structure',
    ariaLabel: 'APS documentation',
  },
  {
    title: 'Kindling',
    bracket: '[ ]',
    description: 'Memory capture, retrieval, adapters, and CLI reference.',
    href: '/kindling/overview',
    accent: 'anvil',
    ariaLabel: 'Kindling documentation',
  },
  {
    title: 'Anvil',
    bracket: '[ = ]',
    description: 'Save-time trust and AI-assisted development workflows.',
    href: '/anvil/overview',
    accent: 'anvil',
    ariaLabel: 'Anvil documentation',
  },
];

export default function HomePage() {
  return (
    <>
      <header className="header">
        <a href="https://eddacraft.ai" className="wordmark">
          eddacraft
        </a>
        <nav className="header-nav">
          <a href="/blog" className="header-link">
            Blog
          </a>
          <a href="https://eddacraft.ai" className="header-link">
            eddacraft.ai &rarr;
          </a>
        </nav>
      </header>

      <main className="ec-landing">
        <section className="ec-landing-hero">
          <p className="ec-landing-eyebrow">PUBLIC_KNOWLEDGE_BASE</p>
          <h1 className="ec-landing-hero__title">
            EDDACRAFT <span className="ec-landing-hero__accent">DOCS</span>
          </h1>
          <p className="ec-landing-hero__subtitle">
            Governed specification, memory, and product documentation.
          </p>
        </section>

        <section className="ec-landing-grid">
          {sections.map((section) => (
            <a
              key={section.title}
              href={section.href}
              className={`ec-landing-card ec-landing-card--${section.accent}`}
              aria-label={section.ariaLabel}
            >
              <p className="ec-landing-card__bracket">{section.bracket}</p>
              <h2 className="ec-landing-card__title">{section.title}</h2>
              <p className="ec-landing-card__description">{section.description}</p>
              <span className="ec-landing-card__cta">OPEN {section.title.toUpperCase()}</span>
            </a>
          ))}
        </section>
      </main>

      <footer className="footer">&copy; {COPYRIGHT_YEAR} eddacraft, Inc.</footer>
    </>
  );
}
