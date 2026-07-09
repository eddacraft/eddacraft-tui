const protectionSignals = [
  { label: 'Save-time status', value: 'Ready' },
  { label: 'Latest run', value: 'Pending API' },
  { label: 'Evidence', value: 'Read-only' },
];

export function DashboardIndexRoute() {
  return (
    <div className="dashboard-home">
      <header className="dashboard-header">
        <div>
          <h1>Protection overview</h1>
          <p>Local workspace protection state</p>
        </div>
        <span className="dashboard-status">Read-only Wave 1</span>
      </header>
      <section className="dashboard-metrics" aria-label="Protection signals">
        {protectionSignals.map((signal) => (
          <article className="dashboard-metric" key={signal.label}>
            <span>{signal.label}</span>
            <strong>{signal.value}</strong>
          </article>
        ))}
      </section>
    </div>
  );
}
