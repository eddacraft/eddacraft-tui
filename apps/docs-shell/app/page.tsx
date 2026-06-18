export const dynamic = 'force-dynamic';

const COPYRIGHT_YEAR = new Date().getFullYear();

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

      <section className="hero">
        <h1>Documentation</h1>
        <p>The forge for governed AI-assisted work.</p>
      </section>

      <section className="cards">
        <a
          href="/anvil/overview"
          className="card"
          data-accent="anvil"
          aria-label="Anvil documentation"
        >
          <h3>Anvil</h3>
          <p>Governed code-gen pipelines for engineering teams.</p>
          <span className="card-link">Read docs &rarr;</span>
        </a>

        <a href="/aps/overview" className="card" data-accent="aps" aria-label="APS documentation">
          <h3>APS</h3>
          <p>Declarative implementation plans for AI-assisted work.</p>
          <span className="card-link">Read docs &rarr;</span>
        </a>

        <a
          href="/kindling/overview"
          className="card"
          data-accent="kindling"
          aria-label="Kindling documentation"
        >
          <h3>Kindling</h3>
          <p>Observation capture and memory substrate.</p>
          <span className="card-link">Read docs &rarr;</span>
        </a>
      </section>

      <footer className="footer">&copy; {COPYRIGHT_YEAR} eddacraft, Inc.</footer>
    </>
  );
}
