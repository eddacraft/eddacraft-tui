---
id: write-observations
title: Write Observations
description: Capturing observations with Kindling.
sidebar_position: 3
---

# Write Observations

Observations are the core of Kindling. Here's how to capture them.

## Basic Observation

```bash
kindling observe "The API requires Content-Type: application/json"
```

**Output:**

```
Observation recorded.
  Capsule: payment-integration
  ID: obs_abc123
```

## With Tags

Add tags for better searchability:

```bash
kindling observe "Rate limit is 100 requests per minute" --tag api --tag limits
```

Or inline:

```bash
kindling observe "Rate limit is 100 requests per minute #api #limits"
```

## Observation Kinds

Specify the type of observation:

```bash
# A discovery (default)
kindling observe "Found the root cause" --kind discovery

# A decision made
kindling observe "Using Redis for caching" --kind decision

# A gotcha/warning
kindling observe "Don't use sync writes in production" --kind gotcha

# A reference/link
kindling observe "See RFC 7231 for status codes" --kind reference
```

### Available Kinds

| Kind | Use for |
|------|---------|
| `discovery` | Things you learned |
| `decision` | Choices made and why |
| `gotcha` | Pitfalls and warnings |
| `reference` | Links and citations |
| `question` | Open questions |
| `todo` | Things to follow up on |

## Multi-line Observations

For longer content, use a heredoc:

```bash
kindling observe <<EOF
The authentication flow works like this:
1. Client sends credentials to /auth/login
2. Server validates and returns JWT
3. Client includes JWT in Authorization header
4. Server validates JWT on each request
EOF
```

Or from a file:

```bash
kindling observe --file notes.md
```

## With Context

Add source context:

```bash
kindling observe "This error means the token expired" \
  --source "debugging" \
  --context "src/auth/middleware.ts:42"
```

## Interactive Mode

For session capture:

```bash
kindling observe --interactive
```

```
Enter observations (Ctrl+D to finish, empty line to save current):

> API uses OAuth2 with PKCE flow
Saved.

> Client secret is not used in PKCE
Saved.

> ^D
2 observations recorded.
```

## Automatic Capture

### From Clipboard

```bash
# Capture clipboard contents as observation
kindling observe --clipboard
```

### From Command Output

```bash
# Capture command output
kindling observe --exec "curl -I https://api.example.com"
```

### With Timestamp

```bash
# Backdate an observation
kindling observe "Deployed v1.2.0" --time "2024-01-14T15:00:00Z"
```

## Best Practices

### Be Specific

```bash
# ❌ Vague
kindling observe "API issue"

# ✓ Specific
kindling observe "API returns 500 when payload exceeds 1MB"
```

### Include Context

```bash
# ❌ Missing context
kindling observe "Use the new method"

# ✓ With context
kindling observe "Use processPaymentV2() - the old method has a race condition"
```

### Tag Consistently

Establish conventions:
- `#bug` for bugs found
- `#perf` for performance insights
- `#security` for security notes
- `#api` for API knowledge

### Capture Promptly

Record observations while context is fresh. Waiting leads to:
- Lost details
- Forgotten reasoning
- Incomplete knowledge

---

**Next:** [Search and export →](/docs/kindling/quickstart/search-export)
