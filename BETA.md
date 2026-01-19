# Anvil Beta Release

> **Version:** 0.1.0-beta.1 **Status:** Pre-release for testing

Thank you for trying Anvil! This is an early beta release intended for testing
and gathering feedback. Your input is invaluable in shaping the future of this
project.

## What is Anvil?

Anvil is a deterministic development automation platform that makes AI-generated
code changes safe for production. It validates plans through quality gates
before execution, ensuring changes meet your team's standards.

## Known Limitations

This beta has the following known limitations:

### Functionality

- **Gate checks**: Some gate checks (policy, OPA/Rego) require external tools to
  be installed
- **Adapters**: SpecKit and BMAD adapters are fully implemented; others are in
  development
- **VS Code extension**: Basic functionality only; advanced features coming soon

### Performance

- First-run validation may be slower as caches are built
- Large monorepos may experience slower gate execution

### Platform Support

- Tested on: Linux (Ubuntu 22.04+), macOS 13+, Windows 11
- Node.js 20+ required

## How to Report Issues

We welcome bug reports and feedback! Please use our issue templates:

- **Bugs**:
  [Report a bug](https://github.com/EddaCraft/anvil-001/issues/new?template=bug_report.md)
- **Feature requests**:
  [Request a feature](https://github.com/EddaCraft/anvil-001/issues/new?template=feature_request.md)
- **General feedback**:
  [Share feedback](https://github.com/EddaCraft/anvil-001/issues/new?template=feedback.md)

### What Makes Good Feedback

- **Specific examples**: Include the commands you ran and what happened
- **Environment details**: OS, Node version, pnpm version
- **Expected vs actual**: What did you expect, and what happened instead?
- **Reproduction steps**: Can you consistently reproduce the issue?

## What Feedback is Helpful

We're especially interested in:

1. **Installation experience**: Was setup straightforward?
2. **Documentation clarity**: Were the docs helpful? What was confusing?
3. **Gate results**: Are the validation messages clear and actionable?
4. **Performance**: Is validation fast enough for your workflow?
5. **Missing features**: What would make Anvil more useful for you?
6. **Integration**: How well does Anvil fit into your existing workflow?

## Getting Help

- **Documentation**: See [README.md](./README.md) for usage instructions
- **Contributing**: See [CONTRIBUTING.md](./CONTRIBUTING.md) to contribute
- **Discussions**: Open an issue for questions

## Roadmap to 1.0

The following features are planned before the stable 1.0 release:

- [ ] Comprehensive documentation site
- [ ] Additional format adapters
- [ ] Enhanced VS Code extension features
- [ ] Performance optimisations for large codebases
- [ ] Plugin architecture for custom gates

## Thank You

Your participation in this beta helps make Anvil better for everyone. We read
every piece of feedback and appreciate your time and effort.

---

**Note**: This is pre-release software. APIs and behaviour may change between
beta releases. Not recommended for production use without thorough testing.
