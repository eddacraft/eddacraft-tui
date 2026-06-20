---
id: policies
title: Custom Policies
sidebar_position: 2
---

# Custom Policies

anvil evaluates custom rules written in OPA/Rego. This tutorial walks through
creating, checking, and running a policy that enforces a service-file naming
convention.

## Prerequisites

- **anvil** initialised in your project (`anvil init`)
- **OPA** binary installed
  ([install guide](https://www.openpolicyagent.org/docs/latest/#running-opa))

Verify OPA is available:

```bash
opa version
```

## 1. Create the Policies Directory

```bash
mkdir -p .anvil/policies
```

anvil loads every `.rego` file in this directory automatically.

## 2. Write the Policy

Create `.anvil/policies/service_names.rego`:

```rego
package anvil.policies.service_names

import rego.v1

warn contains finding if {
  some file in input.files
  startswith(file, "src/services/")
  not endswith(file, ".service.ts")
  not endswith(file, ".service.tsx")

  finding := {
    "message": sprintf("service file should use .service.ts or .service.tsx suffix: %s", [file]),
    "path": file,
    "severity": "warning",
  }
}
```

How it works:

- `input.files` is the list of policy-relevant workspace-relative paths anvil
  passes to OPA
- `warn` emits advisory findings; use `violation` or `deny` for error-severity
  findings
- each object can include `message`, `path`, and `severity` fields so anvil can
  display a useful finding

## 3. Check Policy Tests

Ask anvil to discover policy test files:

```bash
anvil policy test
```

```
Found 1 test file(s) in '.anvil/policies' but policy test execution is not yet implemented
```

To execute Rego tests today, use OPA directly:

```bash
opa test .anvil/policies
```

:::tip Add your own test cases in `.anvil/policies/service_names_test.rego`
using standard OPA test conventions. :::

## 4. Run the Policy

```bash
anvil gate --only-checks policy
```

```
Checking policies...
  [POLICY] service_names
    service file should use .service.ts or .service.tsx suffix: src/services/legacy-handler.ts

1 policy warning found.
```

## 5. Available Input

Custom policies receive a compact project snapshot:

- `input.workspace` — absolute workspace root
- `input.files` — policy-relevant workspace-relative files
- `input.changed_files` — files changed according to Git, when available
- `input.profile` — active gate profile, such as `default`, `dev`, or `ci`

Policies do not receive file contents by default. For content-sensitive checks,
prefer built-in gates or run OPA directly with an explicit input document.

## Ideas for More Policies

| Policy             | What it enforces                                           |
| ------------------ | ---------------------------------------------------------- |
| Naming conventions | Require files in `src/services/` to end with `.service.ts` |
| Import depth       | Flag import chains deeper than N levels                    |
| Test ratio         | Require at least one test file per source file             |
| Export count       | Warn when a module exports more than a threshold           |

Each policy is a standalone `.rego` file. Drop it into `.anvil/policies/` and
anvil picks it up on the next run.

---

**Next:** [Architecture Boundaries](/anvil/tutorials/architecture)
