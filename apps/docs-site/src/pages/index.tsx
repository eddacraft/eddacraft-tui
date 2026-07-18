import React, { type ReactNode } from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './index.module.css';

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx(styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className={styles.heroTitle}>
          {siteConfig.title}
        </Heading>
        <p className={styles.heroSubtitle}>{siteConfig.tagline}</p>
        <p className={styles.heroDescription}>
          Start with a real result, then add protection at the pace your project needs.
        </p>
        <div className={styles.buttons}>
          <Link className="button button--primary button--lg" to="/anvil/overview">
            Understand anvil
          </Link>
          <Link className="button button--secondary button--lg" to="/anvil/quickstart">
            Install anvil
          </Link>
        </div>
      </div>
    </header>
  );
}

interface ProductTileProps {
  title: string;
  tagline: string;
  description: string;
  href: string;
  variant: 'anvil' | 'aps' | 'eddastack';
  status?: 'available' | 'coming-soon';
}

function ProductTile({
  title,
  tagline,
  description,
  href,
  variant,
  status = 'available',
}: ProductTileProps) {
  return (
    <div
      className={clsx(
        styles.productTile,
        styles[`productTile${variant.charAt(0).toUpperCase() + variant.slice(1).replace('-', '')}`]
      )}
    >
      <div className={styles.productTileHeader}>
        <h3 className={styles.productTileTitle}>{title}</h3>
        {status === 'coming-soon' && <span className={styles.badge}>Coming Soon</span>}
      </div>
      <p className={styles.productTileTagline}>{tagline}</p>
      <p className={styles.productTileDescription}>{description}</p>
      <Link className={styles.productTileLink} to={href}>
        Learn more &rarr;
      </Link>
    </div>
  );
}

function ProductTiles() {
  return (
    <section className={styles.products}>
      <div className="container">
        <div className={styles.productGrid}>
          <ProductTile
            title="anvil"
            tagline="Safer changes, earlier"
            description="Check AI-assisted and conventional code changes locally, then add save-time, pre-write, hook, or CI protection when you are ready."
            href="/anvil/overview"
            variant="anvil"
          />
          <ProductTile
            title="APS"
            tagline="A deterministic plan spec"
            description="Define what should be built with hash-stable, reproducible plans. Separate intent from implementation."
            href="/aps/overview"
            variant="aps"
          />
          <ProductTile
            title="Development Memory System"
            tagline="Capture, review, preserve"
            description="Capture high-signal observations, review candidates, and preserve trusted canonical memory for team reuse."
            href="/edda-stack/overview"
            variant="eddastack"
          />
        </div>
      </div>
    </section>
  );
}

function ValueProps() {
  return (
    <section className={styles.valueProps}>
      <div className="container">
        <div className={styles.valueGrid}>
          <div className={styles.valueProp}>
            <h3>Deterministic</h3>
            <p>
              Same inputs, same outputs. Hash-stable plans enable reproducible validation and
              reliable caching.
            </p>
          </div>
          <div className={styles.valueProp}>
            <h3>Save-Time Feedback</h3>
            <p>Issues surface immediately when you save, not hours later in code review or CI.</p>
          </div>
          <div className={styles.valueProp}>
            <h3>Provenance Tracked</h3>
            <p>
              Protected checks can retain their named inputs, results, and origin for later review.
            </p>
          </div>
          <div className={styles.valueProp}>
            <h3>Open Standards</h3>
            <p>APS and kindling are open source. Build on open foundations without lock-in.</p>
          </div>
          <div className={styles.valueProp}>
            <h3>Knowledge That Sticks</h3>
            <p>
              Capture what matters, review for quality, and keep canonical guidance queryable as
              teams and agents scale.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function QuickLinks() {
  return (
    <section className={styles.quickLinks}>
      <div className="container">
        <h2>Quick Links</h2>
        <div className={styles.linkGrid}>
          <div className={styles.linkGroup}>
            <h4>Getting Started</h4>
            <ul>
              <li>
                <Link to="/anvil/overview">What anvil does</Link>
              </li>
              <li>
                <Link to="/aps/overview">APS Overview</Link>
              </li>
              <li>
                <Link to="/anvil/quickstart">Install anvil</Link>
              </li>
              <li>
                <Link to="/edda-stack/overview">Memory System Overview</Link>
              </li>
            </ul>
          </div>
          <div className={styles.linkGroup}>
            <h4>Core Concepts</h4>
            <ul>
              <li>
                <Link to="/anvil/concepts/gates">Gates</Link>
              </li>
              <li>
                <Link to="/aps/spec/taxonomy">APS Taxonomy</Link>
              </li>
              <li>
                <Link to="/anvil/concepts/plans">Plans</Link>
              </li>
              <li>
                <Link to="/edda-stack/design-principles">Memory Design Principles</Link>
              </li>
            </ul>
          </div>
          <div className={styles.linkGroup}>
            <h4>Integrations</h4>
            <ul>
              <li>
                <Link to="/anvil/integrations/github">GitHub Actions</Link>
              </li>
              <li>
                <Link to="/anvil/integrations/vscode">VS Code</Link>
              </li>
              <li>
                <Link to="/anvil/integrations/mcp">AI client integration</Link>
              </li>
            </ul>
          </div>
          <div className={styles.linkGroup}>
            <h4>Reference</h4>
            <ul>
              <li>
                <Link to="/anvil/operations/config">Configuration</Link>
              </li>
              <li>
                <Link to="/aps/schemas/json-schema">APS Schema</Link>
              </li>
              <li>
                <Link to="/aps/examples/minimal-plan">APS Examples</Link>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}

export default function Home(): ReactNode {
  return (
    <Layout
      title="The forge for governed AI-assisted work"
      description="eddacraft builds tools for safer AI-assisted software work. anvil checks changes locally; APS makes plans reproducible."
    >
      <HomepageHeader />
      <main>
        <ProductTiles />
        <ValueProps />
        <QuickLinks />
      </main>
    </Layout>
  );
}
