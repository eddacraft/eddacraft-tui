# Git Worktree Workflow for Anvil Plans

## Overview

Git worktrees enable you to work on multiple Anvil plans simultaneously without
switching branches or affecting your main working directory. This is perfect for
the Anvil workflow where you might be:

- Developing a feature in one worktree
- Validating a different plan in another
- Reviewing a PR in a third

## What are Git Worktrees?

Git worktrees allow you to have **multiple working directories** attached to the
same repository, each checked out to a different branch.

**Traditional Git:**

```
repo/               ← One working directory
  ├── .git/
  └── src/
```

**With Worktrees:**

```
repo/               ← Main working directory (main branch)
  ├── .git/
  └── src/

repo-worktrees/
  ├── feature-auth/     ← Worktree for auth feature (feature-auth branch)
  │   └── src/
  ├── feature-api/      ← Worktree for API feature (feature-api branch)
  │   └── src/
  └── bugfix-login/     ← Worktree for bug fix (bugfix-login branch)
      └── src/
```

## Benefits for Anvil Planning

### 1. **Parallel Plan Development**

Work on multiple features simultaneously without branch switching:

```bash
# Terminal 1: Develop auth feature
cd ~/projects/anvil-worktrees/feature-auth
anvil gate docs/specs/auth-spec.md

# Terminal 2: Develop API feature (simultaneously!)
cd ~/projects/anvil-worktrees/feature-api
anvil gate docs/specs/api-spec.md

# Terminal 3: Main branch for reviews
cd ~/projects/anvil
git pull  # Stays clean, no conflicts with other work
```

### 2. **Isolated Validation**

Each plan runs in complete isolation:

```bash
# Worktree 1: Run gate on auth plan
cd feature-auth-worktree
anvil gate specs/auth.md     # Evidence stored here

# Worktree 2: Run gate on payment plan (parallel!)
cd feature-payment-worktree
anvil gate specs/payment.md  # Evidence stored separately
```

### 3. **No Context Switching**

Keep your IDE/editor open in each worktree:

```
VS Code Window 1:  feature-auth worktree
VS Code Window 2:  feature-api worktree
VS Code Window 3:  main worktree (for reviews)
```

### 4. **PR Review Without Disruption**

Review PRs without affecting your current work:

```bash
# You're working on feature-auth
cd ~/projects/anvil-worktrees/feature-auth
# ...coding...

# PR review request comes in
cd ~/projects/anvil  # Main worktree
git worktree add ../anvil-worktrees/pr-review-123 pr-123
cd ../anvil-worktrees/pr-review-123

# Review the PR, test it
anvil gate specs/their-feature.md

# Done? Remove worktree
cd ~/projects/anvil
git worktree remove ../anvil-worktrees/pr-review-123

# Your feature-auth work is untouched!
```

## Setup Guide

### Initial Setup

```bash
# 1. Your main repository
cd ~/projects/anvil
git clone https://github.com/your-org/anvil.git

# 2. Create worktrees directory
mkdir ../anvil-worktrees
cd anvil

# 3. Add a worktree for a new feature
git worktree add ../anvil-worktrees/feature-auth -b feature-auth

# 4. The new worktree is ready
cd ../anvil-worktrees/feature-auth
ls  # Full repository checkout on feature-auth branch
```

### Directory Structure

**Recommended structure:**

```
~/projects/
├── anvil/                      ← Main worktree (main branch)
│   ├── .git/                   ← Central Git database
│   ├── specs/
│   └── src/
└── anvil-worktrees/           ← All worktrees here
    ├── feature-auth/          ← Worktree 1
    │   ├── specs/auth.md
    │   └── src/
    ├── feature-api/           ← Worktree 2
    │   ├── specs/api.md
    │   └── src/
    └── feature-payments/      ← Worktree 3
        ├── specs/payments.md
        └── src/
```

## Common Workflows

### Workflow 1: Create Plan in New Worktree

```bash
# 1. Create worktree for new feature
cd ~/projects/anvil
git worktree add ../anvil-worktrees/feature-notifications -b feature-notifications

# 2. Work in the new worktree
cd ../anvil-worktrees/feature-notifications

# 3. Install dependencies (if needed)
pnpm install

# 4. Create plan
anvil plan "Add push notifications" --format speckit
# → Generates specs/notifications-spec.md

# 5. Develop and validate
# ...edit code...
anvil gate specs/notifications-spec.md

# 6. Commit and push
git add .
git commit -m "feat: add push notifications"
git push -u origin feature-notifications

# 7. Create PR
gh pr create --title "Add push notifications" --body "$(cat specs/notifications-spec.md)"
```

### Workflow 2: Work on Multiple Plans Simultaneously

```bash
# Morning: Start feature A
cd ~/projects/anvil
git worktree add ../anvil-worktrees/feature-a -b feature-a
cd ../anvil-worktrees/feature-a
anvil plan "Feature A" --format bmad
# ...start coding...

# Afternoon: Switch to feature B (without losing feature A work!)
cd ~/projects/anvil
git worktree add ../anvil-worktrees/feature-b -b feature-b
cd ../anvil-worktrees/feature-b
anvil plan "Feature B" --format speckit
# ...code feature B...

# Later: Back to feature A
cd ~/projects/anvil-worktrees/feature-a
# All your work is exactly as you left it!
# No git stash, no branch switching!
```

### Workflow 3: Review PR Without Disrupting Current Work

```bash
# You're deep in feature-auth work
cd ~/projects/anvil-worktrees/feature-auth
# ...coding...

# Teammate asks you to review PR #42
cd ~/projects/anvil
git fetch origin pull/42/head:pr-42
git worktree add ../anvil-worktrees/pr-42-review pr-42

# Review in isolation
cd ../anvil-worktrees/pr-42-review
anvil gate specs/their-feature.md
npm test
# ...test changes...

# Approve and clean up
gh pr review 42 --approve
cd ~/projects/anvil
git worktree remove ../anvil-worktrees/pr-42-review

# Back to your feature (unchanged!)
cd ../anvil-worktrees/feature-auth
```

### Workflow 4: Hotfix on Production While Developing

```bash
# You're developing a new feature
cd ~/projects/anvil-worktrees/feature-complex

# Production bug reported!
cd ~/projects/anvil
git worktree add ../anvil-worktrees/hotfix-prod -b hotfix-prod origin/main

cd ../anvil-worktrees/hotfix-prod
# Fix bug
anvil gate specs/hotfix-spec.md
git commit -am "fix: production issue"
git push origin hotfix-prod

# Deploy hotfix
# ...

# Remove hotfix worktree
cd ~/projects/anvil
git worktree remove ../anvil-worktrees/hotfix-prod

# Continue feature work (never interrupted)
cd ../anvil-worktrees/feature-complex
```

## Anvil-Specific Considerations

### Evidence Storage

Each worktree has its own `.anvil/` directory:

```bash
# Worktree 1
feature-auth/
  ├── .anvil/
  │   ├── plans/
  │   │   └── aps-auth-001.json
  │   └── evidence/
  │       └── aps-auth-001/
  └── specs/auth-spec.md

# Worktree 2 (independent!)
feature-api/
  ├── .anvil/
  │   ├── plans/
  │   │   └── aps-api-001.json
  │   └── evidence/
  │       └── aps-api-001/
  └── specs/api-spec.md
```

**Note**: `.anvil/` is typically in `.gitignore`, so each worktree maintains its
own local evidence.

### Shared Configuration

Some files are shared across all worktrees (via the shared `.git/`):

- `.anvilrc` (configuration)
- `.anvil/config.json` (if committed)
- Git hooks

### Plan Hashing

Plan hashes are deterministic and based on content, not worktree location:

- Same plan in different worktrees → same hash ✅
- Different plans in different worktrees → different hashes ✅

## Best Practices

### 1. Naming Convention

Use descriptive worktree names that match branch names:

```bash
# Good
git worktree add ../anvil-worktrees/feature-auth -b feature-auth
git worktree add ../anvil-worktrees/bugfix-login -b bugfix-login
git worktree add ../anvil-worktrees/refactor-api -b refactor-api

# Less clear
git worktree add ../anvil-worktrees/temp1 -b my-branch
git worktree add ../anvil-worktrees/test -b testing
```

### 2. Keep Main Worktree Clean

Reserve your main repository for stable work:

```bash
# Main worktree = main branch (for reviews, merging)
~/projects/anvil/  → main branch

# All feature work in worktrees
~/projects/anvil-worktrees/*  → feature branches
```

### 3. Clean Up Old Worktrees

Remove worktrees after merging:

```bash
# List all worktrees
git worktree list

# Remove a worktree (after PR merged)
git worktree remove ../anvil-worktrees/feature-auth

# Also delete the branch
git branch -d feature-auth

# Prune stale worktree references
git worktree prune
```

### 4. Dependency Management

Each worktree can have different `node_modules`:

```bash
# Option A: Shared node_modules (symlink)
# In main repo
pnpm install

# In each worktree
ln -s ~/projects/anvil/node_modules node_modules

# Option B: Independent node_modules (more disk, full isolation)
# In each worktree
pnpm install  # Separate installation
```

**Recommendation for Anvil**: Use **Option B** (independent) to ensure complete
isolation for testing.

### 5. VS Code Integration

VS Code works great with worktrees:

```bash
# Open each worktree in separate window
code ~/projects/anvil-worktrees/feature-auth
code ~/projects/anvil-worktrees/feature-api
code ~/projects/anvil  # Main worktree
```

**Settings**: Each worktree can have `.vscode/settings.json` for
workspace-specific config.

### 6. Anvil Evidence Management

**Local evidence** (default): Each worktree has independent `.anvil/` directory

```bash
# Worktree 1
feature-auth/.anvil/evidence/...

# Worktree 2
feature-api/.anvil/evidence/...
```

**Shared evidence** (if needed): Configure Anvil to use shared evidence storage

```bash
# In .anvilrc
{
  "evidence_path": "~/projects/anvil-shared-evidence/"
}
```

## Common Commands Reference

### Creating Worktrees

```bash
# Create worktree with new branch
git worktree add <path> -b <branch-name>

# Create worktree from existing branch
git worktree add <path> <existing-branch>

# Create worktree from remote branch
git worktree add <path> <remote>/<branch>

# Create worktree from specific commit
git worktree add <path> <commit-hash>
```

### Managing Worktrees

```bash
# List all worktrees
git worktree list

# Get detailed info
git worktree list --porcelain

# Remove a worktree
git worktree remove <path>

# Remove a worktree (force, even with uncommitted changes)
git worktree remove <path> --force

# Clean up stale worktree references
git worktree prune

# Move a worktree to a different location
git worktree move <old-path> <new-path>

# Lock a worktree (prevent accidental removal)
git worktree lock <path>

# Unlock a worktree
git worktree unlock <path>
```

### Checking Worktree Status

```bash
# From any worktree, see all worktrees
git worktree list

# Output example:
# /home/user/projects/anvil              abc123 [main]
# /home/user/projects/anvil-worktrees/feature-auth  def456 [feature-auth]
# /home/user/projects/anvil-worktrees/feature-api   ghi789 [feature-api]
```

## Troubleshooting

### Issue: "Cannot add worktree, branch already exists"

```bash
# Error
$ git worktree add ../anvil-worktrees/feature-auth -b feature-auth
fatal: 'feature-auth' is already checked out at '...'

# Solution 1: Use existing worktree
cd ../anvil-worktrees/feature-auth

# Solution 2: Remove old worktree first
git worktree remove ../anvil-worktrees/feature-auth
git worktree add ../anvil-worktrees/feature-auth -b feature-auth

# Solution 3: Use existing branch without creating new one
git worktree add ../anvil-worktrees/feature-auth feature-auth
```

### Issue: "Worktree directory not empty"

```bash
# Error
$ git worktree add ../anvil-worktrees/feature-auth -b feature-auth
fatal: '../anvil-worktrees/feature-auth' already exists

# Solution: Remove directory first
rm -rf ../anvil-worktrees/feature-auth
git worktree add ../anvil-worktrees/feature-auth -b feature-auth
```

### Issue: "Stale worktree references"

```bash
# After manually deleting worktree directory
$ git worktree list
# Shows deleted worktrees with (prunable) flag

# Solution: Prune stale references
git worktree prune

# Verify
git worktree list
```

### Issue: "Different package versions in worktrees"

```bash
# Symptom: Tests pass in one worktree but fail in another

# Solution: Ensure consistent dependencies
cd worktree-1
pnpm install  # Install from lockfile

cd worktree-2
pnpm install  # Install from lockfile

# Verify versions match
pnpm list package-name  # Check in both worktrees
```

## Advanced: Scripting Worktree Creation

### Helper Script: Create Anvil Feature Worktree

```bash
#!/bin/bash
# File: scripts/new-feature-worktree.sh

FEATURE_NAME=$1
WORKTREE_BASE=../anvil-worktrees

if [ -z "$FEATURE_NAME" ]; then
  echo "Usage: ./new-feature-worktree.sh <feature-name>"
  exit 1
fi

BRANCH_NAME="feature-${FEATURE_NAME}"
WORKTREE_PATH="${WORKTREE_BASE}/${BRANCH_NAME}"

echo "Creating worktree for ${BRANCH_NAME}..."

# Create worktree
git worktree add "$WORKTREE_PATH" -b "$BRANCH_NAME"

# Navigate to worktree
cd "$WORKTREE_PATH"

# Install dependencies
echo "Installing dependencies..."
pnpm install

# Create spec directory if needed
mkdir -p specs

echo "✅ Worktree created: $WORKTREE_PATH"
echo "Next steps:"
echo "  cd $WORKTREE_PATH"
echo "  anvil plan \"Your feature description\" --format speckit"
```

**Usage:**

```bash
chmod +x scripts/new-feature-worktree.sh
./scripts/new-feature-worktree.sh auth
# → Creates worktree at ../anvil-worktrees/feature-auth
```

### Helper Script: Clean Merged Worktrees

```bash
#!/bin/bash
# File: scripts/clean-merged-worktrees.sh

WORKTREE_BASE=../anvil-worktrees

echo "Finding merged branches with worktrees..."

# Get list of merged branches
git branch --merged main | grep -v "main" | while read branch; do
  # Check if worktree exists for this branch
  WORKTREE_PATH="${WORKTREE_BASE}/${branch}"

  if [ -d "$WORKTREE_PATH" ]; then
    echo "Removing worktree for merged branch: $branch"
    git worktree remove "$WORKTREE_PATH" 2>/dev/null || echo "  (already removed)"
    git branch -d "$branch" 2>/dev/null || echo "  (branch already deleted)"
  fi
done

# Prune stale references
git worktree prune

echo "✅ Cleanup complete"
```

## Integration with Anvil CLI

### Future Enhancement: Native Worktree Support

Potential Anvil CLI commands for worktree management:

```bash
# Create feature with worktree
anvil feature create auth --worktree
# → Creates worktree, branch, and initial spec

# List all Anvil plans across worktrees
anvil plan list --all-worktrees
# → Shows plans from all worktrees

# Gate all plans in all worktrees
anvil gate --all-worktrees
# → Validates all plans across all worktrees

# Clean merged worktrees
anvil worktree cleanup
# → Removes worktrees for merged branches
```

## Summary

### ✅ Use Worktrees When:

- Working on multiple features simultaneously
- Need to review PRs without disrupting current work
- Running long-running tests on one plan while developing another
- Need complete isolation between different plans
- Want to keep multiple validation contexts active

### ❌ Don't Use Worktrees When:

- Rapidly switching between branches for small changes (use `git switch`)
- Working on a single feature (unnecessary overhead)
- Very limited disk space (worktrees duplicate working directory)

### Key Takeaway

Git worktrees are **perfect for Anvil's plan-based workflow** because they
enable:

- ✅ Parallel plan development
- ✅ Isolated validation contexts
- ✅ Zero context switching overhead
- ✅ Independent `.anvil/` evidence per plan
- ✅ PR reviews without disrupting active work

**Recommended workflow**: Main repo on `main` branch, all feature work in
dedicated worktrees under `../anvil-worktrees/`.
