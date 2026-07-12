import type { components } from '@/api/generated/openapi';
import { Badge } from '@/components/ui/badge';

type Entry = components['schemas']['PlanTimelineEntry'];

export function EvidenceTimeline({ entries }: { entries: Entry[] }) {
  return (
    <ol className="plan-timeline" aria-label="Plan readiness and evidence timeline">
      {entries.map((entry) => (
        <li className="panel" key={entry.id}>
          <div>
            <strong>
              {entry.id}: {entry.title}
            </strong>
            <Badge variant="outline">{entry.status}</Badge>
          </div>
          <p>{entry.readiness ? 'Ready for the next authorised step' : 'Lifecycle evidence'}</p>
          <code>{entry.evidence ?? 'No validation evidence recorded'}</code>
        </li>
      ))}
    </ol>
  );
}
