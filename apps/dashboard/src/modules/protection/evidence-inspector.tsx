import type { components } from '@/api/generated/openapi';
import { SyntaxGlyph } from '@/components/brand/syntax-glyph';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';

type Warning = components['schemas']['WarningSummary'];

export function EvidenceInspector({ warning }: { warning?: Warning }) {
  if (!warning) {
    return (
      <aside
        aria-labelledby="evidence-inspector-title"
        className="panel evidence-inspector"
        id="evidence-inspector"
      >
        <header className="panel-header evidence-header">
          <div>
            <p className="eyebrow">Selected warning</p>
            <h2 id="evidence-inspector-title">Evidence inspector</h2>
          </div>
        </header>
        <div className="evidence-body">
          <p>No evidence is selected.</p>
        </div>
      </aside>
    );
  }
  const file = warning.file_path ?? 'Workspace';
  return (
    <aside
      aria-labelledby="evidence-inspector-title"
      className="panel evidence-inspector"
      id="evidence-inspector"
      tabIndex={-1}
    >
      <header className="panel-header evidence-header">
        <div>
          <p className="eyebrow">Selected warning</p>
          <h2 id="evidence-inspector-title">Evidence inspector</h2>
        </div>
      </header>
      <div className="evidence-body">
        <div className="evidence-rule-heading">
          <div>
            <strong>{warning.rule}</strong>
            <span>{warning.category}</span>
          </div>
          <Badge variant="outline">{warning.severity.toUpperCase()}</Badge>
        </div>
        <div className="evidence-file">
          <span>File</span>
          <code>
            {file}:{warning.line ?? '—'}
          </code>
        </div>
        <Separator />
        <section aria-labelledby="why-flagged-title" className="evidence-section">
          <h3 id="why-flagged-title">Why this was flagged</h3>
          <p>{warning.explanation}</p>
        </section>
        {warning.evidence_excerpt.length ? (
          <div aria-label={`Evidence excerpt from ${file}`} className="code-evidence">
            {warning.evidence_excerpt.map((line) => (
              <div
                className={line.highlighted ? 'code-line code-line-highlighted' : 'code-line'}
                key={line.number}
              >
                <span aria-hidden="true" className="line-number">
                  {line.number}
                </span>
                <code>{line.text || ' '}</code>
              </div>
            ))}
          </div>
        ) : (
          <p className="evidence-unavailable">No deterministic evidence excerpt was captured.</p>
        )}
        <dl className="evidence-metadata">
          <div>
            <dt>Matched pattern</dt>
            <dd>
              <code>{warning.matched_pattern}</code>
            </dd>
          </div>
          <div>
            <dt>Evidence ID</dt>
            <dd>
              <code>{warning.evidence_id}</code>
            </dd>
          </div>
        </dl>
      </div>
      <footer className="evidence-actions">
        <Button disabled size="sm" type="button" variant="outline">
          <SyntaxGlyph kind="context" /> View evidence
        </Button>
        <Button disabled size="sm" type="button" variant="ghost">
          <SyntaxGlyph kind="action" /> Copy evidence ID
        </Button>
      </footer>
    </aside>
  );
}
