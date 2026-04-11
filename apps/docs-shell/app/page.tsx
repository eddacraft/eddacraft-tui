export default function HomePage() {
  return (
    <>
      <section className="hero">
        <h1>EddaCraft</h1>
        <p className="tagline">The forge for governed AI-assisted work.</p>
        <a className="cta" href="/anvil/overview">
          Anvil docs
        </a>
        <a className="cta secondary" href="/aps/overview">
          APS spec
        </a>
      </section>

      <section className="sections">
        <div className="section">
          <h3>Anvil</h3>
          <p>Commercial beta: governed code-gen pipelines for engineering teams.</p>
          <a href="/anvil/overview">Read the Anvil docs →</a>
        </div>
        <div className="section">
          <h3>APS</h3>
          <p>Open-source Anvil Plan Spec: declarative implementation plans.</p>
          <a href="/aps/overview">Read the APS spec →</a>
        </div>
        <div className="section">
          <h3>Kindling</h3>
          <p>Open-source observation capture and memory substrate.</p>
          <a href="/kindling/overview">Read the Kindling docs →</a>
        </div>
        <div className="section">
          <h3>edda-stack</h3>
          <p>Open-source integration layer between Anvil, APS, and Kindling.</p>
          <a href="/edda-stack/overview">Read the edda-stack docs →</a>
        </div>
      </section>
    </>
  );
}
