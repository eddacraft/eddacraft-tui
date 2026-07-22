import { useState } from 'react';

import type { components } from '@/api/generated/openapi';
import { EmptyState } from '@/components/primitives/empty-state';
import { Badge } from '@/components/ui/badge';

type Check = components['schemas']['GateCheckSummary'];

export function CheckTree({ checks }: { checks: readonly Check[] }) {
  const [expanded, setExpanded] = useState<string | null>(checks[0]?.name ?? null);

  if (checks.length === 0) {
    return (
      <EmptyState description="This gate run does not include a check tree." title="No checks" />
    );
  }

  return (
    <ul aria-label="Gate check tree" className="check-tree">
      {checks.map((check) => {
        const open = expanded === check.name;
        return (
          <li className="check-tree-item" key={check.name}>
            <button
              aria-expanded={open}
              className="check-tree-toggle"
              onClick={() => setExpanded(open ? null : check.name)}
              type="button"
            >
              <span>
                <strong>{check.name}</strong>
                <span className="muted-cell">{check.score ?? '—'}</span>
              </span>
              <Badge variant="outline">{check.status}</Badge>
            </button>
            {open ? <pre className="check-tree-detail">{check.message || 'No detail'}</pre> : null}
          </li>
        );
      })}
    </ul>
  );
}
