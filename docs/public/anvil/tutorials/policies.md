---
id: policies
title: Custom Policies
sidebar_position: 2
---

# Custom Policies

anvil evaluates custom rules written in OPA/Rego. This tutorial walks through
creating, testing, and running a policy that limits file length.

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

Create `.anvil/policies/max_file_length.rego`:

```rego
package anvil.policies.max_file_length

import future.keywords.if

default max_lines := 300

max_lines := input.config.max_lines if {
  input.config.max_lines
}

violation[msg] {
  count(input.file.lines) > max_lines
  msg := sprintf("%s exceeds %d lines (%d)",
    [input.file.path, max_lines,
     count(input.file.lines)])
}
```

How it works:

- `max_lines` defaults to 300 but can be overridden via `input.config`
- The `violation` rule fires when a file exceeds the threshold
- anvil treats every string in the `violation` set as a warning

## 3. Test the Policy

Run the built-in policy test harness:

```bash
anvil policy test
```

```
Testing policies...
  .anvil/policies/max_file_length.rego
    PASS  violation fires when file exceeds max_lines
    PASS  no violation when file is within limit
    PASS  respects config override

All policy tests passed.
```

:::tip Add your own test cases in `.anvil/policies/max_file_length_test.rego`
using standard OPA test conventions. :::

## 4. Run the Policy

```bash
anvil gate --only-checks policy
```

```
Checking policies...
  [POLICY] max_file_length
    src/services/legacy-handler.ts exceeds 300 lines (487)

1 policy warning found.
```

## 5. Customise the Threshold

Override the default in `.anvilrc`:

```json
{
  "gates": {
    "policies": {
      "enabled": true,
      "config": {
        "max_file_length": {
          "max_lines": 500
        }
      }
    }
  }
}
```

You can also set per-directory thresholds using gate-config.json files placed
alongside the code they govern.

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
