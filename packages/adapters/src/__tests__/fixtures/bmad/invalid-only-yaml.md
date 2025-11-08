---
name: 'Document with Only YAML'
version: '1.0.0'
author: 'Test Author'
date: '2025-10-25'
description: 'This document only has YAML front-matter'
---

# Document Title

This document has proper YAML front-matter but absolutely no requirements or
user stories. It's just freeform content without any of the structured elements
that make it a BMAD document.

This should result in low confidence detection and validation warnings about
missing requirements.
