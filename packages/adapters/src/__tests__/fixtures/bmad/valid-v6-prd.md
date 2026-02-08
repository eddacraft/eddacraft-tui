---
name: 'Product Requirements Document'
version: '6.0.0'
description: 'BMAD v6 PRD with hyphenated variables'
output_file: '{project-root}/docs/PRD.md'
author: 'v6 Author'
date: '2026-01-15'
---

# Product Requirements Document

## Change Log

| Date       | Version | Description         | Author    |
| :--------- | :------ | :------------------ | :-------- |
| 2026-01-15 | 6.0.0   | v6 format migration | v6 Author |

## Executive Summary

This PRD describes the migration to BMAD v6 format with updated folder structure
and variable syntax. The project uses `_bmad/_config` for configuration and
`{project-root}` for path references.

## Functional Requirements

<!-- prettier-ignore -->
FR-01: Support `_bmad` folder structure
FR-02: Support `_config` folder for modules
FR-03: Expand `{project-root}` variables

## Non-Functional Requirements

<!-- prettier-ignore -->
NFR-01: Backward compat with `.bmad` and `_cfg`
NFR-02: Support `{project_root}` and `{project-root}`
