# Changesets

This directory is used by [changesets](https://github.com/changesets/changesets)
to manage versioning and changelogs.

## Usage

When making changes that should be released:

```bash
pnpm changeset
```

This will prompt you to:

1. Select which packages have changed
2. Choose the type of version bump (major/minor/patch)
3. Write a summary of the changes

## Workflow

1. Create a changeset when making notable changes
2. Changesets are committed with your PR
3. On merge, changesets are consumed to update versions and CHANGELOG

## Configuration

See `.changeset/config.json` for changeset configuration.
