---
name: 'Code Review Agent'
version: '1.0.0'
author: 'BMAD Team'
hasSidecar: true
---

# Code Review Agent

## Purpose

An automated code review agent that analyzes pull requests for quality,
security, and best practice adherence using BMAD v6 agent framework.

## Role

Perform thorough code reviews on incoming pull requests, checking for:

- Coding standards compliance
- Security vulnerabilities
- Performance issues
- Test coverage gaps

## Configuration

The agent uses `_bmad/_config/module.yaml` for its base configuration and stores
review history in `_bmad/_memory`.
