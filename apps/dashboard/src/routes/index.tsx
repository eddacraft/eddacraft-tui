export function DashboardIndexRoute() {
  return (
    <div className="dashboard-home">
      <header className="dashboard-header">
        <div>
          <h1>Protection overview</h1>
          <p>Local workspace protection state and deterministic evidence</p>
        </div>
        <span className="dashboard-status">Local data disconnected</span>
      </header>
      <section className="dashboard-empty" aria-labelledby="empty-state-title">
        <span aria-hidden="true" className="dashboard-empty-mark" />
        <div>
          <h2 id="empty-state-title">No protection data connected</h2>
          <p>
            Protection status, recent runs, warnings, affected files, and evidence will appear after
            the local dashboard server is available.
          </p>
        </div>
      </section>
    </div>
  );
}
