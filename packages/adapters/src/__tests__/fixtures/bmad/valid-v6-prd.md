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
FR-01: Support `_bmad` folder structure for project configuration
FR-02: Support `_config` folder for module configuration
FR-03: Expand `{project-root}` hyphenated variable syntax

## Non-Functional Requirements

<!-- prettier-ignore -->
NFR-01: Maintain backward compatibility with `.bmad` and `_cfg` folders
NFR-02: Support both `{project_root}` and `{project-root}` variable formats
