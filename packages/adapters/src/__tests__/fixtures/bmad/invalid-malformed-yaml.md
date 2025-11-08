---
name: Test Document
version: 1.0.0: invalid
author: [unclosed array
date: "2025-10-25
invalid: yaml: structure: here
---

# Document with Malformed YAML

This document has YAML front-matter that is syntactically invalid, which should
affect parsing but the BMAD adapter should handle gracefully.

FR-01: Some requirement that exists

The YAML is malformed but we still have requirement identifiers.
