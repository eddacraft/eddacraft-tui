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
  href: string;
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
      href: `/gates/${encodeURIComponent(run.id)}`,
    });
  }

  for (const warning of overview.warnings) {
    events.push({
      id: `warning:${warning.id}`,
      kind: 'warning',
      timestamp: warning.age_label,
      summary: warning.message,
      badge: warning.severity,
      href: `/warnings?evidence=${encodeURIComponent(warning.id)}&severity=all&view=warnings`,
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
            <a className="activity-feed-item" href={event.href}>
              <div>
                <strong>{event.summary}</strong>
                <span className="muted-cell">{event.timestamp}</span>
              </div>
              <Badge variant="outline">{event.badge}</Badge>
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
