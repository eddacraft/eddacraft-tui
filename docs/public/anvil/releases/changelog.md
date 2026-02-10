---
id: changelog
title: Changelog
description: Release history for Anvil.
sidebar_position: 1
---

# Changelog

All notable changes to Anvil are documented here.

## [1.0.0] - 2024-01-15

### Added

- **Watch mode** — real-time validation on file save
- **Architecture boundaries** — define and enforce import rules
- **Anti-pattern detection** — AP-001 through AP-007
- **Secret detection** — pattern matching + entropy analysis
- **Evidence system** — immutable audit trail
- **GitHub Action** — CI integration with PR comments
- **VS Code extension** — in-editor diagnostics
- **TUI** — interactive terminal interface
- **MCP server** — Model Context Protocol integration

### Changed

- Initial stable release

### Security

- All evidence is cryptographically signed
- Secrets never appear in logs or output

---

## [0.9.0] - 2024-01-01

### Added

- Beta release for early adopters
- Core validation engine
- Basic CLI commands

### Known Issues

- Watch mode may miss rapid file changes
- VS Code extension requires manual reload on config change

---

## Versioning

Anvil follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes to config or CLI
- **MINOR** — new features, backward compatible
- **PATCH** — bug fixes

## Upgrading

See [Upgrade Notes](/anvil/releases/upgrade-notes) for migration guides.
