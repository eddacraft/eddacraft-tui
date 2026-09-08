# GitHub review-thread GraphQL templates

Use these templates when the compact workflow in `address-reviews` needs a
copy-paste query. REST review-comment endpoints do not expose thread resolution
state or the thread node ID required by `resolveReviewThread`.

## Fetch unresolved review threads

Resolve `<owner>`, `<repo>`, and `<pr-number>` from live repository and PR state.
Do not embed repository-specific values in the skill.

```console
gh api graphql -f query='query($owner: String!, $repo: String!, $pr: Int!) { repository(owner: $owner, name: $repo) { pullRequest(number: $pr) { reviewThreads(first: 100) { nodes { id isResolved isOutdated path line comments(first: 50) { nodes { id databaseId body author { login } createdAt } } } } } } }' -f owner='<owner>' -f repo='<repo>' -F pr=<pr-number>
```

Filter to `isResolved: false`. The `id` on each thread is the GraphQL node ID;
the top-level comment's `databaseId` is the value used by the REST reply
endpoint.

## Reply to the top-level review comment

```console
gh api --method POST repos/<owner>/<repo>/pulls/<pr-number>/comments/<comment-database-id>/replies -f body='Addressed: <concise evidence-backed response>'
```

Do not `@`-mention automated reviewers because that can trigger another review.

## Resolve the thread

```console
gh api graphql -f query='mutation($threadId: ID!) { resolveReviewThread(input: { threadId: $threadId }) { thread { isResolved } } }' -f threadId='<thread-node-id>'
```

Re-fetch unresolved threads after replies and resolutions; do not infer success
from the mutation request alone.
