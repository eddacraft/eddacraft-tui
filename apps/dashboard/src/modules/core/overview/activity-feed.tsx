import { Link } from '@tanstack/react-router';

import type { components } from '@/api/generated/openapi';
import { EmptyState } from '@/components/primitives/empty-state';
import { Badge } from '@/components/ui/badge';

type Overview = components['schemas']['ProtectionOverview'];

export interface ActivityEvent {
  id: string;
  kind: 'gate' | 'warning';
  timestamp: string;
  summary: string;
  badge: string;
  targetId: string;
}

export function deriveActivityEvents(overview: Overview, limit = 20): ActivityEvent[] {
  const events: ActivityEvent[] = [];

  for (const run of overview.recent_runs) {
    events.push({
      id: `gate:${run.id}`,
      kind: 'gate',
      timestamp: run.started_at ?? 'Latest gate',
      summary: `${run.label} · ${run.warning_count} warnings`,
      badge: run.result,
      targetId: run.id,
    });
  }

  for (const warning of overview.warnings) {
    events.push({
      id: `warning:${warning.id}`,
      kind: 'warning',
      timestamp: warning.age_label,
      summary: warning.message,
      badge: warning.severity,
      targetId: warning.evidence_id || warning.id,
    });
  }

  return events.slice(0, limit);
}

export function ActivityFeed({ overview }: { overview: Overview }) {
  const events = deriveActivityEvents(overview);

  if (events.length === 0) {
    return (
      <EmptyState
        description="No latest-gate or warning events are available yet."
        title="No recent activity"
      />
    );
  }

  return (
    <section aria-labelledby="activity-feed-title" className="panel activity-feed">
      <header className="panel-header">
        <div>
          <h2 id="activity-feed-title">Recent activity</h2>
          <p>Latest gate and warning signals from local evidence</p>
        </div>
        <span className="panel-count">{events.length} events</span>
      </header>
      <ul className="activity-feed-list">
        {events.map((event) => (
          <li key={event.id}>
            {event.kind === 'gate' ? (
              <Link className="activity-feed-item" params={{ id: event.targetId }} to="/gates/$id">
                <ActivityEventContent event={event} />
              </Link>
            ) : (
              <Link
                className="activity-feed-item"
                search={{ evidence: event.targetId, severity: 'all', view: 'warnings' }}
                to="/warnings"
              >
                <ActivityEventContent event={event} />
              </Link>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function ActivityEventContent({ event }: { event: ActivityEvent }) {
  return (
    <>
      <div>
        <strong>{event.summary}</strong>
        <span className="muted-cell">{event.timestamp}</span>
      </div>
      <Badge variant="outline">{event.badge}</Badge>
    </>
  );
}
