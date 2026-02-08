# Explain Command — Execution Steps

## EXPLAIN-001: Warning ID System

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ warning-id.ts created with ID generation     |
| 2    | ✅ Parse warning ID into rule, file, line       |
| 3    | ✅ Find warning by ID in warning list           |
| 4    | ✅ Short ID support for display                 |
| 5    | ✅ Tests pass: 26 tests in warning-id.test.ts   |

## EXPLAIN-002: Explanation Templates

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ Types defined: WarningExplanation, Context   |
| 2    | ✅ Template registry with register/get/render   |
| 3    | ✅ Generic fallback for unknown rules           |
| 4    | ✅ Tests pass: 12 tests in template-loader.test |

## EXPLAIN-003: Architecture Explanations

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ ARCH-001 through ARCH-004 templates          |
| 2    | ✅ BOUND-001 template for boundary violations   |
| 3    | ✅ Layer context in explanations                |
| 4    | ✅ Tests pass: 15 tests in boundary-explainer   |

## EXPLAIN-004: Anti-pattern Explanations

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ AP-001 through AP-007 templates              |
| 2    | ✅ Detailed why/how/when sections per pattern   |
| 3    | ✅ Suppression syntax examples                  |
| 4    | ✅ Tests pass: 10 tests in antipattern-explainer|

## EXPLAIN-005: ExplainService

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ Template initialisation on first use         |
| 2    | ✅ explainWarning for Warning objects           |
| 3    | ✅ explainById for warning ID lookup            |
| 4    | ✅ explainByRule for rule-only explanation      |
| 5    | ✅ listWarnings for warning enumeration         |
| 6    | ✅ Tests pass: 19 tests in explain-service      |

## EXPLAIN-006: CLI Explain Command

| Step | State                                            |
| ---- | ------------------------------------------------ |
| 1    | ✅ `anvil explain <warning-id>` command         |
| 2    | ✅ `anvil explain --list` shows available rules |
| 3    | ✅ `anvil explain --json` JSON output           |
| 4    | ✅ Formatted terminal output with sections      |
| 5    | ✅ Command registered and help working          |

## Verification

| Check      | Result                                          |
| ---------- | ----------------------------------------------- |
| Build      | ✅ All packages build successfully              |
| Tests      | ✅ 1627 tests pass (82 new explain tests)       |
| Typecheck  | ✅ No type errors                               |
| Lint       | ✅ Only pre-existing vscode warnings            |
| CLI        | ✅ `anvil explain --help` works                 |
