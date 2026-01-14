# General Development Rules

## Code Quality

1. **Clean Code First**: Write readable, maintainable code
2. **DRY Principle**: Avoid duplication, but don't over-abstract
3. **SOLID Principles**: Follow when appropriate, don't force
4. **Meaningful Names**: Variables, functions, and classes should be
   self-documenting

## Security

1. **Never Trust Input**: Validate all external data
2. **Least Privilege**: Request minimum permissions needed
3. **No Secrets in Code**: Use environment variables or secret managers
4. **Defense in Depth**: Multiple layers of security

## Testing

1. **Test First**: Write tests before implementation when possible
2. **Meaningful Tests**: Test behavior, not implementation
3. **Edge Cases**: Always consider boundaries and error conditions
4. **Fast Feedback**: Keep unit tests fast

## Git Workflow

1. **Atomic Commits**: One logical change per commit
2. **Conventional Commits**: Use standard prefixes (feat, fix, docs, etc.)
3. **Descriptive Messages**: Explain why, not just what
4. **Review Before Merge**: All code should be reviewed

## Documentation

1. **Code is Documentation**: Write self-documenting code
2. **Comments for Why**: Explain reasoning, not obvious what
3. **README Updates**: Keep docs in sync with code
4. **API Documentation**: Document public interfaces

## Error Handling

1. **Fail Fast**: Detect errors early
2. **Meaningful Errors**: Provide actionable error messages
3. **Don't Swallow Errors**: Log or propagate, never ignore
4. **Graceful Degradation**: Handle failures elegantly

## Performance

1. **Measure First**: Don't optimize without profiling
2. **Premature Optimization**: Avoid until necessary
3. **Efficient Algorithms**: Choose appropriate data structures
4. **Resource Cleanup**: Always release resources
