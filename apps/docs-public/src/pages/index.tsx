import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import type { ReactNode } from 'react';

const sections = [
  {
    title: 'APS',
    description: 'Plan specification, examples, schemas, and validation guidance.',
    href: '/aps/overview',
  },
  {
    title: 'Kindling',
    description: 'Memory capture, retrieval concepts, adapters, and CLI reference.',
    href: '/kindling/overview',
  },
  {
    title: 'Memory Stack',
    description: 'Edda, Ember, and Kindling architecture for governed organisational memory.',
    href: '/edda-stack/overview',
  },
  {
    title: 'Anvil',
    description: 'Public product docs for save-time trust and AI-assisted development workflows.',
    href: 'https://docs.eddacraft.ai/anvil/overview',
  },
];

export default function Home(): ReactNode {
  return (
    <Layout
      title="eddacraft docs"
      description="Public eddacraft documentation for governed AI-assisted work"
    >
      <main className="container margin-vert--xl">
        <section className="hero hero--dark padding-vert--xl">
          <div className="container">
            <h1 className="hero__title">eddacraft docs</h1>
            <p className="hero__subtitle">
              The public knowledge base for governed AI-assisted work.
            </p>
          </div>
        </section>

        <section className="row margin-top--lg">
          {sections.map((section) => (
            <article className="col col--6 margin-bottom--lg" key={section.title}>
              <div className="card">
                <div className="card__body">
                  <h2>{section.title}</h2>
                  <p>{section.description}</p>
                </div>
                <div className="card__footer">
                  <Link className="button button--primary" to={section.href}>
                    Open {section.title}
                  </Link>
                </div>
              </div>
            </article>
          ))}
        </section>
      </main>
    </Layout>
  );
}
