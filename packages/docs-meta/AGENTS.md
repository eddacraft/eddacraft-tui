# docs-meta Package (@eddacraft/anvil-docs-meta)

> Parser for the DOCGOV-002 documentation governance metadata convention.

**Parent**: See root `AGENTS.md` for project-wide conventions.

## Structure

```
packages/docs-meta/src/
├── parser/                  # Markdown AST parsing (remark + remark-gfm)
│   ├── parse-metadata.ts    # parseDocGovernance() — main entry
│   └── index.ts             # Subpath barrel
├── types/                   # Zod schemas and inferred TypeScript types
│   └── index.ts             # DocMetadataSchema, DocRelationsSchema, ParseError, …
└── index.ts                 # Root barrel
```

## Where to Add Things

| Task                                 | Location                        |
| ------------------------------------ | ------------------------------- |
| New enum value (Type, Authority, …)  | `types/index.ts`                |
| New parsed field on the metadata row | `types/index.ts` and parser     |
| New table shape (e.g. tags table)    | New file under `parser/`        |
| Fixture-driven test for a doc shape  | `parser/parse-metadata.test.ts` |

## Conventions

- The canonical convention lives in `docs/guides/documentation-governance.md`.
  Treat that file as the upstream source; schemas in this package are derived
  from it.
- All public types are inferred from Zod schemas. Export the schema and the
  inferred type together so callers can choose runtime validation or
  compile-time typing.
- Errors are raised as `ParseError` with `sourcePath` and `lineNumber` populated
  whenever the AST provides position info.

## Anti-Patterns

- Do not reach into other packages (`@eddacraft/anvil-aps`, anvil core, etc.)
  from here. This package must stay a pure leaf consumer of `unified` /
  `remark-gfm` / `zod`.
- Do not accept unparsed strings as input to consumer-facing functions. The
  parser receives Markdown text; consumers receive parsed and validated
  structures.
- Do not introduce filesystem access. The parser is pure; reading files is the
  caller's job (typically a `scripts/docs/*.mjs` validator).
- Do not silently coerce unknown enum values. The convention is small; new
  values belong in `types/index.ts` and the governance guide together.
