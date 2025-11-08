# Community Addons

This directory is for community-contributed addons. Share your custom hooks and
workflows with others!

## Contributing an Addon

### Requirements

1. Must include `addon.json` with proper metadata
2. Hooks must be non-destructive (use `|| true` for non-blocking)
3. Include clear documentation
4. Test across different environments
5. No hardcoded paths or user-specific configuration

### Submission Process

1. Create your addon in a subdirectory
2. Follow the naming convention: `hooks-[purpose]` or `workflow-[name]`
3. Include examples of usage
4. Submit via PR to the main repository

### Example Structure

```
community/
├── hooks-django-dev/
│   ├── addon.json
│   ├── hooks.json
│   └── README.md
├── hooks-rust-cargo/
│   ├── addon.json
│   └── hooks.json
└── workflow-aws-deploy/
    ├── addon.json
    ├── hooks.json
    └── deploy-script.sh
```

## Available Community Addons

_Currently empty - be the first to contribute!_

## Ideas for Community Addons

- **Framework-Specific**
  - Django/Flask development hooks
  - React/Vue/Angular component generators
  - Rails conventions enforcement
- **Language-Specific**
  - Rust with cargo and clippy
  - Go with gofmt and go vet
  - Java with checkstyle
- **Tool-Specific**
  - Docker compose workflows
  - Kubernetes manifests validation
  - Terraform plan checks
- **Platform-Specific**
  - AWS deployment helpers
  - Vercel/Netlify integration
  - GitHub Actions optimisation

## Guidelines

### Naming Conventions

- `hooks-*` for hook collections
- `workflow-*` for multi-step workflows
- `tool-*` for specific tool integrations
- Use lowercase with hyphens

### Documentation

Each addon should include:

- Purpose and use cases
- Required tools/dependencies
- Configuration options
- Example usage
- Known limitations

### Testing

Before submitting:

1. Test on fresh project
2. Test with missing dependencies
3. Verify non-blocking behaviour
4. Check timeout values
5. Ensure cross-platform compatibility

## Support

For help creating addons:

1. Check existing core addons for examples
2. Ask in discussions/issues
3. Refer to the main addon documentation

## License

Community addons should be compatible with the main project license. By
contributing, you agree to make your addon available under the same terms.
