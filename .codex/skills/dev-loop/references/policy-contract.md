# Repository Policy Contract

Use repository-native configuration when present. The exact file format may evolve; preserve these semantics.

```yaml
devLoop:
  defaultMode: interactive # or autonomous
  integrationBranch: main
  isolation:
    provider: worktrunk
    requireFor: [module, autonomous, parallel]
  pullRequests:
    defaultBoundary: invocation-target
    autonomousMerge: true
  claims:
    provider: git-ref # or anvil or manual
    leaseMinutes: 30
    heartbeatMinutes: 10
  repair:
    maxCycles: 5
    stopOnNoProgress: true
  risk:
    default: standard
    pathRules: []
    mandatoryDifferentialDesign: [architectural, security-sensitive, irreversible, materially-ambiguous]
    crossModelVerification: [high, critical, disputed]
  gates:
    required: []
    postMerge: []
```

## Resolution

1. Load repository policy.
2. Overlay APS-declared requirements.
3. Raise the effective risk or gates when project truth demands it.
4. Apply a human override only when it is session-scoped, explicit, recorded, and permitted by external controls.

Mode controls checkpoints and terminal authority, never target scope. Invocation target controls scope.

## Override record

Record:

- operator identity;
- session and target;
- policy being varied;
- authority granted or restriction relaxed;
- rationale;
- issued and expiry timestamps;
- resulting actions.

Never treat an override as permission to evade branch protection, unavailable credentials, platform policy, or destructive-action safeguards outside its explicit scope.
