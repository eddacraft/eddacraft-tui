import type { components } from '@/api/generated/openapi';
import { Badge } from '@/components/ui/badge';

type GateRun = components['schemas']['GateRunSummary'];

export function GateDetailHeader({ run }: { run: GateRun }) {
  return (
    <header className="protection-heading">
      <p className="eyebrow">GATE_RUN</p>
      <h1>{run.label}</h1>
      <div className="data-state-row">
        <Badge variant="outline">{run.result}</Badge>
        <Badge variant="outline">Score {run.score == null ? '—' : run.score}</Badge>
        <Badge variant="outline">{run.started_at ?? 'Latest gate'}</Badge>
        <Badge variant="outline">
          {run.duration_seconds == null ? 'Duration —' : `${run.duration_seconds}s`}
        </Badge>
      </div>
    </header>
  );
}
