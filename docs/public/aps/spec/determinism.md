---
id: determinism
title: Determinism Rules
description: How APS achieves hash stability and reproducible validation.
sidebar_position: 3
---

# Determinism Rules

APS is designed for deterministic, reproducible validation. This page explains
how.

## What is Determinism?

**Determinism** means: same inputs → same outputs.

For APS:

- Same plan content → same hash
- Same plan → same validation result
- Same task → same success criteria

## Hash Stability

### How Hashes Work

Each APS document has a content hash:

```markdown
---
format: aps
version: 1.0
hash: sha256:a1b2c3d4e5f6...
---
```

The hash is computed from the **semantic content**, not the raw text.

### What Changes the Hash

**Content changes:**

```markdown
# Before

Intent: Users can log in

# After (hash changes)

Intent: Users can log in with SSO
```

### What Doesn't Change the Hash

**Whitespace normalisation:**

```markdown
# These produce the same hash:

Intent: Users can log in

Intent: Users can log in

Intent: Users can log in
```

**Comment changes:**

```markdown
# These produce the same hash:

Intent: Users can log in

<!-- This is a comment -->

Intent: Users can log in

<!-- Different comment -->
```

### Computing Hashes

```bash
# Compute hash for a plan
anvil plan hash plans/index.aps.md

# Output:
# sha256:a1b2c3d4e5f6...
```

## Normalisation Rules

### Text Normalisation

1. Trim leading/trailing whitespace
2. Collapse multiple spaces to single space
3. Normalise line endings to LF
4. Remove trailing whitespace from lines

### Structural Normalisation

1. Order frontmatter fields alphabetically
2. Order list items by appearance
3. Preserve heading hierarchy

### What's Preserved

- Heading text and level
- Task IDs and titles
- Intent text
- Validation commands
- Step text and order
- Dependencies

### What's Ignored

- Comments (HTML-style)
- Extra blank lines
- Trailing whitespace
- Frontmatter order

## Reproducibility

### Validation Reproducibility

Given:

- Same plan (by hash)
- Same configuration
- Same codebase state

Result:

- Same validation outcome
- Same evidence produced

### Caching

Hash stability enables caching:

```
Plan hash: abc123
Config hash: def456
Files hash: ghi789
Combined: xyz...

Cache hit? Use cached result.
Cache miss? Run validation.
```

### Audit Trail

Reproducibility enables auditing:

```
Q: "Was this code validated?"
A: Evidence shows plan abc123 passed at timestamp T
   Same plan today produces same hash
   Therefore: same validation criteria applied
```

## Implementation Details

### Hash Algorithm

APS uses SHA-256:

- Widely supported
- Collision resistant
- Fast computation

### Content Extraction

Before hashing:

1. Parse Markdown to AST
2. Extract semantic nodes
3. Normalise text content
4. Serialise to canonical form
5. Compute SHA-256

### Canonical Form

The canonical form is JSON:

```json
{
  "format": "aps",
  "version": "1.0",
  "title": "My Project",
  "modules": [
    {
      "id": "auth",
      "tasks": [
        {
          "id": "AUTH-001",
          "title": "Login endpoint",
          "intent": "Users can log in"
        }
      ]
    }
  ]
}
```

This JSON is then hashed.

## Edge Cases

### Empty Content

Empty or whitespace-only content has a defined hash:

```
sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

(This is the SHA-256 of empty string.)

### Unicode

Unicode is normalised (NFC) before hashing:

```markdown
# These produce the same hash:

café (composed) café (decomposed: e + combining accent)
```

### Ordering

Task order matters (affects hash). Step order matters.

```markdown
# Different hashes:

## Tasks

- AUTH-001
- AUTH-002

## Tasks

- AUTH-002
- AUTH-001
```

## Verification

### Manual Verification

```bash
# Compute expected hash
anvil plan hash plans/index.aps.md

# Compare with stored hash
cat plans/index.aps.md | grep "hash:"
```

### Automated Verification

```bash
# Verify all plans
anvil plan verify

# Output:
# plans/index.aps.md: ✓ valid
# plans/modules/auth.aps.md: ✗ hash mismatch
```

---

**Next:** [JSON Schema →](/aps/schemas/json-schema)
