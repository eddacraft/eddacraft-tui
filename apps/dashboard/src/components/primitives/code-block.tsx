export interface CodeBlockProps {
  code: string;
  language?: string;
  label?: string;
}

export function CodeBlock({ code, language = 'text', label = 'Code sample' }: CodeBlockProps) {
  return (
    <figure className="overflow-hidden rounded-none border bg-muted">
      <figcaption className="border-b px-3 py-2 text-xs text-muted-foreground">
        {label} · {language}
      </figcaption>
      <pre className="overflow-x-auto p-3 text-xs" tabIndex={0}>
        <code>{code}</code>
      </pre>
    </figure>
  );
}
