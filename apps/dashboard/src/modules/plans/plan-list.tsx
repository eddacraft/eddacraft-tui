import { Link } from '@tanstack/react-router';

import type { components } from '@/api/generated/openapi';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

type PlanSummary = components['schemas']['PlanSummary'];

export function PlanList({ plans }: { plans: PlanSummary[] }) {
  return (
    <section className="panel plan-list" aria-labelledby="plan-list-title">
      <header className="panel-header">
        <div>
          <h2 id="plan-list-title">Indexed plans</h2>
          <p>{plans.length} read-only APS modules</p>
        </div>
      </header>
      <Table className="operations-table">
        <TableHeader>
          <TableRow>
            <TableHead>Scope</TableHead>
            <TableHead>Plan</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Progress</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {plans.map((plan) => (
            <TableRow key={plan.id}>
              <TableCell>
                <Badge variant="outline">{plan.scope}</Badge>
              </TableCell>
              <TableCell>
                <Link
                  className="table-select-button table-rule"
                  params={{ id: plan.id }}
                  to="/plans/$id"
                >
                  {plan.title}
                </Link>
              </TableCell>
              <TableCell>{plan.status}</TableCell>
              <TableCell>{plan.progress}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </section>
  );
}
