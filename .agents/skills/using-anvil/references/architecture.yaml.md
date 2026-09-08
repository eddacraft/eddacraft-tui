# `.anvil/architecture.yaml`

Import-boundary rules (layers and deps). For domain invariants that need custom
Rego or pack admission, stop `using-anvil` and follow current product docs.

A layer declares the files it owns and which layers it may depend on:

```yaml
schema_version: "0.1.0"
template: custom
layers:
  api-layer:
    patterns:
      - "src/api/**"
    depends_on:
      - service-layer
      - utils

  service-layer:
    patterns:
      - "src/services/**"
    depends_on:
      - repository-layer
      - utils

  repository-layer:
    patterns:
      - "src/repositories/**"
    depends_on:
      - utils

  utils:
    patterns:
      - "src/utils/**"
    depends_on: []
```

Validate before relying on it:

```bash
anvil architecture validate
```
