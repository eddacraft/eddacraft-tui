# APS Non-Goals

> What Anvil Planning Spec explicitly **does not** do.

## Purpose of This Document

APS has a focused scope. This document clarifies what APS deliberately excludes
to prevent scope creep and maintain simplicity.

---

## 1. Project Management

### What APS Is Not

APS is **not** a project management tool like Jira, Linear, or Asana.

### What's Excluded

- **Time tracking** — No effort estimates, time logged, or burndown charts
- **Sprint planning** — No sprint assignments, velocity tracking, or iteration
  management
- **Assignee workflows** — No task assignment, status transitions, or approval
  flows
- **Notifications** — No alerts, reminders, or email integrations
- **Reporting** — No dashboards, metrics, or progress visualisation

### Why

Project management tools exist and integrate with code repositories. APS focuses
on **planning as code**, not replacing PM tools.

### Integration Path

If PM integration is needed:

- Export APS tasks to your PM tool
- Sync status bidirectionally via API
- Keep planning docs as source of truth

---

## 2. Issue Tracking

### What APS Is Not

APS is **not** a bug tracker or issue management system.

### What's Excluded

- **Bug reports** — No structured bug templates, severity levels, or
  reproduction steps
- **Feature requests** — No voting, prioritisation, or roadmap planning
- **Support tickets** — No customer-facing issue tracking
- **Triage workflows** — No labelling, assignment, or escalation

### Why

GitHub Issues, GitLab Issues, and similar tools handle this well. APS focuses on
**planned work**, not emergent issues.

### When to Use APS vs Issues

**Use APS for:**

- Planned features with clear scope
- Refactoring or technical debt with defined tasks
- System design with module boundaries

**Use Issues for:**

- Bug reports
- User-reported problems
- Feature requests from users
- Support questions

---

## 3. Execution Tracking

### What APS Is Not

APS is **not** a task runner or CI/CD system.

### What's Excluded

- **Automated execution** — No background jobs, scheduled tasks, or runners
- **Progress monitoring** — No real-time logs, output streaming, or telemetry
- **Retry logic** — No automatic retries, failure handling, or recovery
- **Parallelisation** — No concurrency control or distributed execution
- **Resource management** — No CPU/memory limits or quota enforcement

### Why

Execution is handled by Anvil's execution layer (powered by LLM agents). APS
only provides the **planning input** and **state tracking**.

### Where Execution Happens

- **Planning:** APS documents define what to do
- **Locking:** `anvil plan lock` creates an execution plan
- **Execution:** Anvil agents execute the task
- **Completion:** `anvil plan status` shows results

---

## 4. Version Control

### What APS Is Not

APS is **not** a replacement for Git or version control.

### What's Excluded

- **File versioning** — No built-in diff, blame, or history
- **Branching** — No concept of feature branches for planning docs
- **Merging** — No conflict resolution or merge strategies
- **Tagging** — No release tagging or version milestones

### Why

Planning docs are **Markdown files committed to Git**. Use Git for version
control, as you would with any code.

### Best Practices

- Commit planning docs to Git
- Use branches for experimental plans
- Review planning docs in pull requests
- Tag releases with Git tags

---

## 5. Knowledge Base / Documentation

### What APS Is Not

APS is **not** a documentation system or wiki.

### What's Excluded

- **General documentation** — No API docs, user guides, or tutorials
- **Search functionality** — No full-text search across docs
- **Cross-referencing** — No automatic backlinks or related pages
- **Publishing** — No static site generation or doc hosting

### Why

APS is for **actionable planning**, not general documentation. Use dedicated
tools like:

- Docusaurus, MkDocs, or Sphinx for documentation
- Notion or Confluence for wikis
- README files for project documentation

### When Documentation Is Relevant

Planning docs can link to documentation:

```markdown
### AUTH-001: Implement OAuth

**Intent:** Add OAuth support per
[Authentication RFC](../../docs/rfcs/auth-rfc.md)
```

But the RFC itself is not an APS document.

---

## 6. Artifact Storage

### What APS Is Not

APS is **not** a file storage system for design assets, diagrams, or artifacts.

### What's Excluded

- **File attachments** — No binary file uploads (PDFs, images, videos)
- **Diagram embedding** — No built-in diagram rendering (Mermaid, PlantUML)
- **Asset management** — No versioning of design files or mockups

### Why

Git is not optimised for binary files. Use appropriate tools:

- **Design files:** Figma, Sketch, or design repositories
- **Diagrams:** Mermaid in README files, or diagram-as-code tools
- **Documents:** Link to Google Docs or other doc systems

### Linking to Assets

Planning docs can link to external assets:

```markdown
### UI-001: Build login screen

**Intent:** Implement login UI per [Figma mockups](https://figma.com/...)
```

---

## 7. Communication / Collaboration

### What APS Is Not

APS is **not** a collaboration platform like Slack or Discord.

### What's Excluded

- **Comments** — No inline comments or discussion threads
- **Mentions** — No `@user` notifications
- **Reactions** — No emoji reactions or voting
- **Chat** — No real-time messaging

### Why

GitHub already provides PR comments, code review, and discussions. APS doesn't
duplicate this.

### Collaboration Workflow

- **Planning discussion:** Use GitHub Issues or Discussions
- **Review planning docs:** Use pull requests
- **Inline questions:** Use PR review comments
- **Decisions:** Document in `## Decisions` section of planning doc

---

## 8. Estimation / Forecasting

### What APS Is Not

APS is **not** an estimation or forecasting tool.

### What's Excluded

- **Story points** — No complexity estimates
- **Time estimates** — No hours or days for tasks
- **Velocity tracking** — No historical throughput analysis
- **Forecasting** — No projected completion dates

### Why

Estimation is subjective and often inaccurate. APS focuses on **clear intent**,
not predicting duration.

### Where Estimation Fits

If estimation is needed:

- Add estimates as tags: `**Tags:** estimate-large, estimate-2-days`
- Track externally in PM tool
- Use Git commit history for actual time analysis

---

## 9. Access Control / Permissions

### What APS Is Not

APS is **not** a permission or access control system.

### What's Excluded

- **User roles** — No admin, editor, viewer roles
- **File-level permissions** — No read/write restrictions
- **Task ownership enforcement** — No "only assignee can lock"
- **Audit logs** — No built-in activity tracking

### Why

Git and hosting platforms (GitHub, GitLab) handle access control. APS relies on
repository permissions.

### Security Model

- **Read access:** Anyone with repository read access can view planning docs
- **Write access:** Anyone with write access can modify planning docs
- **Lock enforcement:** First lock wins, but not enforced by permissions

---

## 10. Complex Workflows

### What APS Is Not

APS is **not** a workflow engine or state machine.

### What's Excluded

- **Custom states** — Only 4 states: `open`, `locked`, `completed`, `cancelled`
- **State transitions** — No approval flows or multi-step processes
- **Conditional logic** — No if/then rules or branching workflows
- **Triggers** — No automated actions on state changes

### Why

Complex workflows add complexity without clear benefit. APS uses simple, linear
task states.

### If Complex Workflows Are Needed

- Use dedicated workflow tools (GitHub Actions, Temporal, etc.)
- Trigger workflows based on APS state changes
- Keep APS simple, orchestrate externally

---

## 11. Natural Language Processing

### What APS Is Not

APS is **not** an AI-powered planning assistant.

### What's Excluded

- **Intent parsing** — No natural language understanding
- **Task generation** — No automated task breakdown from descriptions
- **Dependency inference** — No automatic dependency detection
- **Scope suggestion** — No AI-driven scope recommendations

### Why

APS is a **structured format**, not an AI system. Humans (or LLMs via Anvil
agents) write planning docs explicitly.

### Where AI Fits

LLMs can:

- Help **write** planning docs (via prompts)
- Validate planning docs (via `anvil plan validate`)
- Execute tasks (via `anvil plan lock` + Anvil agents)

But APS itself is static structured Markdown.

---

## 12. Dynamic Content

### What APS Is Not

APS is **not** a dynamic or database-backed system.

### What's Excluded

- **Queries** — No SQL-like queries across tasks
- **Computed fields** — No calculated values or aggregations
- **Dynamic lists** — No auto-generated task lists
- **Live updates** — No real-time synchronisation

### Why

Planning docs are **static Markdown files**. Parse them, extract data, but don't
expect database semantics.

### If Dynamic Queries Are Needed

- Parse planning docs into a database (SQLite, PostgreSQL)
- Query the database
- Regenerate reports or views
- Keep planning docs as canonical source

---

## 13. Multi-Tenancy

### What APS Is Not

APS is **not** a multi-tenant SaaS platform.

### What's Excluded

- **Organisations** — No concept of orgs, teams, or workspaces
- **Isolation** — No data separation between projects
- **Billing** — No usage tracking or subscription management
- **Branding** — No custom themes or white-labelling

### Why

APS is a **file format and library**, not a hosted service. Each repository has
its own planning docs.

### Multi-Project Usage

If you have multiple projects:

- Each repository has its own `docs/planning/` directory
- No shared planning docs across repos
- Use Git submodules or monorepos if sharing is needed

---

## 14. Configuration Management

### What APS Is Not

APS is **not** a configuration or infrastructure-as-code tool.

### What's Excluded

- **Environment config** — No prod/staging/dev configuration
- **Secrets management** — No encrypted credentials
- **Deployment config** — No Kubernetes manifests or Terraform files
- **Feature flags** — No runtime toggles

### Why

Use dedicated tools for configuration:

- **Config:** dotenv, config files, environment variables
- **Secrets:** Vault, AWS Secrets Manager, GitHub Secrets
- **IaC:** Terraform, Pulumi, CloudFormation

---

## Summary Table

| Feature Area    | APS Does                      | APS Does Not                        |
| --------------- | ----------------------------- | ----------------------------------- |
| Planning        | ✅ Structured planning docs   | ❌ Project management               |
| Task Management | ✅ Task definitions + locking | ❌ Issue tracking                   |
| Execution       | ✅ Lock tasks for execution   | ❌ Run tasks automatically          |
| Version Control | ✅ Uses Git                   | ❌ Built-in versioning              |
| Documentation   | ✅ Links to docs              | ❌ General documentation system     |
| Storage         | ✅ Markdown files             | ❌ Binary file attachments          |
| Collaboration   | ✅ PR reviews                 | ❌ Chat or comments                 |
| Estimation      | ✅ Confidence levels          | ❌ Time estimates or story points   |
| Access Control  | ✅ Git permissions            | ❌ Custom roles or permissions      |
| Workflows       | ✅ Simple states              | ❌ Complex state machines           |
| AI              | ✅ LLM-friendly format        | ❌ AI-powered parsing or generation |
| Dynamic Content | ✅ Static Markdown            | ❌ Queries or computed fields       |
| Multi-Tenancy   | ✅ Per-repository             | ❌ Multi-tenant SaaS                |
| Configuration   | ✅ Links to config            | ❌ Config management                |

---

## Design Philosophy

APS follows these principles:

1. **Do one thing well** — Planning as structured Markdown
2. **Leverage existing tools** — Git, GitHub, editors
3. **Stay simple** — Markdown + conventions, not a platform
4. **Integrate, don't replace** — Works with PM tools, not against them

By explicitly excluding these features, APS remains focused, maintainable, and
easy to adopt.

---

## Version History

- **v0.1** (2025-12-17) — Initial non-goals
