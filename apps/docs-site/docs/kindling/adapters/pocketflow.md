---
id: pocketflow
title: PocketFlow Adapter
description: Using Kindling with PocketFlow workflows.
sidebar_position: 2
---

# PocketFlow Adapter

Integrate Kindling as nodes in PocketFlow workflows.

## What is PocketFlow?

PocketFlow is a workflow automation tool. The Kindling adapter provides nodes for capturing and retrieving observations within workflows.

## Setup

### Install the Adapter

```bash
kindling adapter install pocketflow
```

### Register Nodes

```bash
kindling adapter pocketflow register
```

This adds Kindling nodes to your PocketFlow node palette.

## Available Nodes

### Observe Node

Captures an observation:

```yaml
- node: kindling.observe
  inputs:
    content: "{{ workflow.result }}"
    kind: discovery
    tags:
      - automated
      - workflow
  outputs:
    observation_id: obs_id
```

### Search Node

Queries observations:

```yaml
- node: kindling.search
  inputs:
    query: "API rate limit"
    capsule: my-project
    limit: 5
  outputs:
    results: observations
```

### Export Node

Exports observations:

```yaml
- node: kindling.export
  inputs:
    capsule: my-project
    format: markdown
    since: 7d
  outputs:
    content: exported_docs
```

## Example Workflow

Capture learnings from a code review:

```yaml
name: code-review-capture
trigger:
  event: pull_request.reviewed

steps:
  - node: github.get_review
    inputs:
      pr: "{{ trigger.pr_number }}"
    outputs:
      review: review_data

  - node: kindling.observe
    inputs:
      content: |
        PR #{{ trigger.pr_number }}: {{ review_data.summary }}
        Key feedback: {{ review_data.comments | join(', ') }}
      kind: discovery
      tags:
        - code-review
        - pr-{{ trigger.pr_number }}
    outputs:
      observation_id: captured_id

  - node: slack.notify
    inputs:
      message: "Captured review insights: {{ captured_id }}"
```

## Context Injection

Use Kindling to inject context into workflows:

```yaml
- node: kindling.search
  inputs:
    query: "{{ task.description }}"
    capsule: team-knowledge
    limit: 3
  outputs:
    context: relevant_observations

- node: llm.generate
  inputs:
    prompt: |
      Task: {{ task.description }}

      Relevant context from past work:
      {{ relevant_observations | format }}

      Generate a solution...
```

## Configuration

```json
{
  "pocketflow": {
    "defaultCapsule": "workflow-observations",
    "autoTag": true,
    "tagPrefix": "pf-"
  }
}
```

---

**Next:** [Custom adapters →](/docs/kindling/adapters/custom)
