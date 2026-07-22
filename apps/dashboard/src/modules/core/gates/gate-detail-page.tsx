import type { components } from '@/api/generated/openapi';
import { EmptyState } from '@/components/primitives/empty-state';
import { CheckTree } from '@/modules/core/gates/check-tree';
import { GateDetailHeader } from '@/modules/core/gates/gate-detail-header';

type Overview = components['schemas']['ProtectionOverview'];

export function GateDetailPage({ overview, id }: { overview: Overview; id: string }) {
  const run =
    overview.recent_runs.find((item) => item.id === id) ??
    (overview.latest_run?.id === id ? overview.latest_run : undefined);

  if (!run) {
    return (
      <EmptyState
        description={`No local gate run with id “${id}” is present in the protection overview.`}
        title="Gate run not found"
      />
    );
  }

  return (
    <section className="gate-detail">
      <GateDetailHeader run={run} />
      <div className="panel">
        <header className="panel-header">
          <div>
            <h2>Check tree</h2>
            <p>Expand a check for detailed output from the latest gate artefact</p>
          </div>
          <span className="panel-count">{run.checks.length} checks</span>
        </header>
        <CheckTree checks={run.checks} />
      </div>
      <footer className="panel">
        <p className="muted-cell">
          Provenance: local `.anvil/gates.json` · source message: {overview.source_message}
        </p>
      </footer>
    </section>
  );
}
