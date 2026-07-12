export interface WorkspaceSwitcherProps {
  root: string;
  refreshedAt: string;
}

export function WorkspaceSwitcher({ root, refreshedAt }: WorkspaceSwitcherProps) {
  return (
    <dl className="topbar-context">
      <div>
        <dt>Workspace root</dt>
        <dd>{root}</dd>
      </div>
      <div>
        <dt>Last refreshed</dt>
        <dd>{refreshedAt}</dd>
      </div>
    </dl>
  );
}
