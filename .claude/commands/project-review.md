---
description:
  Comprehensive project review covering code quality, security, architecture,
  testing, and documentation
---

# Project Review Workflow

Holistic assessment of project health across all quality dimensions:

1. **Architecture Review** - Evaluate system design, patterns, and scalability
2. **Code Quality Review** - Assess maintainability, correctness, and
   performance
3. **Security Audit** - Identify vulnerabilities and security risks
4. **Test Coverage Analysis** - Review test strategy and coverage
5. **Documentation Assessment** - Verify completeness and accuracy
6. **Comprehensive Report** - Actionable recommendations with priorities

## Agent Sequence

- **architect**: Reviews architecture patterns, design decisions, and
  scalability
- **reviewer**: Performs code quality review for correctness and maintainability
- **security-auditor**: Comprehensive security audit
- **tester**: Analyses test coverage and strategy
- **docs-writer**: Assesses documentation quality and completeness

## Usage

```
/project-review
```

Run this command for a comprehensive health check of your project.

## What Gets Reviewed

### Architecture & Design

- **Patterns**: Consistency with established patterns (DDD, Clean Architecture,
  etc.)
- **Modularity**: Component separation and boundaries
- **Scalability**: Performance bottlenecks and scaling concerns
- **Dependencies**: Coupling and dependency management
- **Data Flow**: Request/response cycles and state management
- **Infrastructure**: Deployment, monitoring, and observability
- **Decision Records**: ADR completeness and quality

### Code Quality

- **Correctness**: Logic errors, edge cases, error handling
- **Maintainability**: Code organisation, naming, complexity
- **Performance**: Algorithm efficiency, N+1 queries, bottlenecks
- **Type Safety**: TypeScript usage, type coverage
- **Code Duplication**: DRY violations and refactoring opportunities
- **Error Handling**: Graceful degradation and recovery
- **Logging**: Appropriate logging levels and context

### Security

- **Authentication/Authorization**: Access control completeness
- **Input Validation**: Injection vulnerabilities
- **Secrets Management**: Credential handling
- **PII Protection**: Sensitive data handling
- **Dependencies**: Known vulnerabilities (CVEs)
- **OWASP Top 10**: Common vulnerability coverage
- **Security Headers**: HTTP security configuration

### Testing

- **Test Coverage**: Unit, integration, E2E test completeness
- **Test Quality**: Test clarity, reliability, speed
- **Edge Cases**: Boundary condition coverage
- **Error Scenarios**: Failure mode testing
- **Test Strategy**: Appropriate test types and pyramid balance
- **CI/CD Integration**: Automated test execution
- **Test Documentation**: Test plan and strategy docs

### Documentation

- **README**: Completeness, accuracy, onboarding clarity
- **API Documentation**: Endpoint documentation, examples
- **Architecture Docs**: System design, patterns, decisions
- **ADRs**: Decision records for major choices
- **Code Comments**: Inline documentation quality
- **Runbooks**: Operational procedures and troubleshooting
- **Contributing Guide**: Development workflow and standards

## Output Artifacts

### Executive Summary

- **Overall Health Score**: Aggregate assessment
- **Critical Issues**: Blockers requiring immediate attention
- **Risk Assessment**: Technical debt and risk areas
- **Recommendations**: Top 5 prioritised improvements

### Detailed Reports

1. **Architecture Assessment**
   - Design patterns analysis
   - Scalability concerns
   - Recommended refactoring

2. **Code Quality Report**
   - Maintainability score
   - Complexity hotspots
   - Refactoring opportunities

3. **Security Audit**
   - Vulnerability inventory (by severity)
   - OWASP checklist results
   - Remediation roadmap

4. **Test Coverage Analysis**
   - Coverage metrics by module
   - Test quality assessment
   - Missing test scenarios

5. **Documentation Audit**
   - Documentation gaps
   - Outdated content
   - Recommended additions

### Action Plan

- **Immediate Actions** (Critical) - Address within days
- **Short-term Actions** (High) - Address this sprint
- **Medium-term Actions** (Medium) - Address this quarter
- **Long-term Actions** (Low) - Consider for future

## When to Run

- **Quarterly Health Checks** - Regular project assessment
- **Pre-Release Reviews** - Before major version releases
- **Onboarding New Teams** - Assess inherited codebases
- **Technical Debt Assessment** - Evaluate refactoring needs
- **Post-Incident Reviews** - After major issues or outages
- **Merger/Acquisition Due Diligence** - Code quality assessment
- **Compliance Audits** - Prepare for SOC 2, ISO, etc.

## Project Context Detection

The review automatically detects and adapts to your project's:

- **Tech Stack**: Identifies frameworks, languages, and build tools from
  package.json, requirements.txt, go.mod, etc.
- **Architecture**: Discovers patterns by analysing directory structure and code
  organisation
- **Testing**: Detects test frameworks and existing test patterns
- **Tooling**: Identifies linters, formatters, CI/CD configuration
- **Conventions**: Learns naming patterns, file organisation, and coding
  standards

The review assesses your project against its own technology choices and
architectural decisions.

## Follow-up Actions

Based on findings, agents will create handoffs for:

- **planner**: To create implementation plan for major improvements
- **coder**: To implement fixes and refactoring
- **docs-writer**: To address documentation gaps
- **architect**: To design architectural improvements
- **tester**: To improve test coverage

## Focused Reviews

You can focus the review on specific areas by adding context to your command:

```
/project-review - focus on security
/project-review - focus on architecture
/project-review - focus on testing
/project-review - only review the API module
```

The agents will adjust their scope based on your request.
