import type { components } from '@/api/generated/openapi';
import { CodeBlock } from '@/components/primitives/code-block';
import { SeverityBadge } from '@/components/primitives/severity-badge';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import type { DashboardSeverity } from '@/lib/theme';

type Warning = components['schemas']['WarningSummary'];

function asSeverity(value: string): DashboardSeverity {
  if (value === 'critical' || value === 'high' || value === 'medium' || value === 'low') {
    return value;
  }
  return 'medium';
}

export function WarningDetailPanel({
  warning,
  open,
  onOpenChange,
}: {
  warning?: Warning;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Sheet onOpenChange={onOpenChange} open={open}>
      <SheetContent className="warning-detail-sheet" side="right">
        <SheetHeader>
          <SheetTitle>{warning?.rule ?? 'Warning detail'}</SheetTitle>
          <SheetDescription>
            {warning
              ? `${warning.file_path ?? 'Workspace'}:${warning.line ?? '—'}`
              : 'No warning selected'}
          </SheetDescription>
        </SheetHeader>
        {warning ? (
          <div className="warning-detail-body">
            <SeverityBadge severity={asSeverity(warning.severity)} />
            <p>{warning.explanation || warning.message}</p>
            {warning.matched_pattern ? (
              <p className="muted-cell">Pattern: {warning.matched_pattern}</p>
            ) : null}
            {warning.evidence_excerpt.length > 0 ? (
              <CodeBlock
                code={warning.evidence_excerpt
                  .map((line) => `${line.number}| ${line.text}`)
                  .join('\n')}
                label={warning.file_path ?? 'Evidence'}
              />
            ) : (
              <p className="evidence-unavailable">
                No deterministic evidence excerpt was captured.
              </p>
            )}
            <p className="muted-cell">Age: {warning.age_label}</p>
          </div>
        ) : null}
      </SheetContent>
    </Sheet>
  );
}
