# Runtime adapter contract

`agent-messaging` owns message behaviour and the JSON wire format. A runtime
adapter may replace the bundled POSIX scripts only when it preserves the
following `agent-messaging/v1` operations and results:

- send a validated message from a structured file or standard input;
- receive with type, sender, priority, timestamp, unread, and limit filters;
- acknowledge returned message IDs;
- enqueue, list, pop, and complete agent triggers; and
- use project-local storage with an explicit override.

Mutations validate their complete input before changing state. Concurrent
writes require a cross-platform lock and atomic replacement or an equivalent
transaction; POSIX append behaviour is not a portable guarantee. Timestamps are
RFC 3339 UTC instants. Existing schema-valid JSON and JSONL state remains
readable.

Capability discovery must report the contract name and version. If no adapter
is available, the bundled Bash and `jq` scripts are a POSIX fallback. They are
not a native Windows implementation, and a binding must not silently route
through Git Bash while claiming native support.
