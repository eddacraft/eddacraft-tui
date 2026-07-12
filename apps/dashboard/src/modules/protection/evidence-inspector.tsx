import { Check, Clipboard, ExternalLink } from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import type { ProtectionWarning } from '@/modules/protection/fixture';

interface EvidenceInspectorProps {
  warning: ProtectionWarning;
}

export function EvidenceInspector({ warning }: EvidenceInspectorProps) {
  const [copied, setCopied] = useState<'path' | 'evidence' | null>(null);

  const copyText = async (value: string, target: 'path' | 'evidence') => {
    try {
      await navigator.clipboard?.writeText(value);
      setCopied(target);
      window.setTimeout(() => setCopied(null), 1600);
    } catch {
      setCopied(null);
    }
  };

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
        <span aria-hidden="true" className="inspector-live-dot" />
      </header>

      <div className="evidence-body">
        <div className="evidence-rule-heading">
          <div>
            <strong>{warning.rule}</strong>
            <span>{warning.category}</span>
          </div>
          <Badge
            className={`severity-badge severity-badge-${warning.severity.toLowerCase()}`}
            variant="outline"
          >
            {warning.severity}
          </Badge>
        </div>

        <div className="evidence-file">
          <span>File</span>
          <code>
            {warning.file}:{warning.line}
          </code>
          <Button
            aria-label="Copy file path"
            className="copy-button"
            onClick={() => void copyText(`${warning.file}:${warning.line}`, 'path')}
            size="xs"
            type="button"
            variant="ghost"
          >
            {copied === 'path' ? <Check aria-hidden="true" /> : <Clipboard aria-hidden="true" />}
            {copied === 'path' ? 'Copied' : 'Copy path'}
          </Button>
        </div>

        <Separator />

        <section aria-labelledby="why-flagged-title" className="evidence-section">
          <h3 id="why-flagged-title">Why this was flagged</h3>
          <p>{warning.explanation}</p>
        </section>

        {warning.evidence ? (
          <div aria-label={`Evidence excerpt from ${warning.file}`} className="code-evidence">
            {warning.code.map((line) => (
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
          <p className="evidence-unavailable">
            No deterministic evidence was captured for this warning.
          </p>
        )}

        <dl className="evidence-metadata">
          <div>
            <dt>Matched pattern</dt>
            <dd>
              <code>{warning.matchedPattern}</code>
            </dd>
          </div>
          <div>
            <dt>Evidence ID</dt>
            <dd>
              <code>{warning.evidenceId}</code>
            </dd>
          </div>
          <div>
            <dt>Run</dt>
            <dd>2025-05-28 14:32:07 (18.4s)</dd>
          </div>
        </dl>
      </div>

      <footer className="evidence-actions">
        <Button disabled={!warning.evidence} size="sm" type="button" variant="outline">
          <ExternalLink aria-hidden="true" /> View evidence
        </Button>
        <Button
          disabled={!warning.evidence}
          onClick={() => void copyText(warning.evidenceId, 'evidence')}
          size="sm"
          type="button"
          variant="ghost"
        >
          {copied === 'evidence' ? <Check aria-hidden="true" /> : <Clipboard aria-hidden="true" />}
          {copied === 'evidence' ? 'Copied' : 'Copy evidence ID'}
        </Button>
      </footer>
    </aside>
  );
}
