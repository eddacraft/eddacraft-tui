import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import type { ReactNode } from 'react';

type SectionAccent = 'structure' | 'anvil';

const sections: Array<{
  title: string;
  bracket: string;
  description: string;
  href: string;
  accent: SectionAccent;
}> = [
  {
    title: 'APS',
    bracket: '[ ]',
    description: 'Plan specification, schemas, validation, and implementation guidance.',
    href: '/aps/overview',
    accent: 'structure',
  },
  {
    title: 'Kindling',
    bracket: '[ ]',
    description: 'Memory capture, retrieval, adapters, and CLI reference.',
    href: '/kindling/overview',
    accent: 'anvil',
  },
  {
    title: 'Anvil',
    bracket: '[ = ]',
    description: 'Save-time trust and AI-assisted development workflows.',
    href: 'https://docs.eddacraft.ai/anvil/overview',
    accent: 'anvil',
  },
];

export default function Home(): ReactNode {
  return (
    <Layout
      title="eddacraft docs"
      description="Governed specification, memory, and product documentation."
    >
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
            <article
              className={`ec-landing-card ec-landing-card--${section.accent}`}
              key={section.title}
            >
              <p className="ec-landing-card__bracket">{section.bracket}</p>
              <h2 className="ec-landing-card__title">{section.title}</h2>
              <p className="ec-landing-card__description">{section.description}</p>
              <Link className="ec-landing-card__cta" to={section.href}>
                OPEN {section.title.toUpperCase()}
              </Link>
            </article>
          ))}
        </section>
      </main>
    </Layout>
  );
}
