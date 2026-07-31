//! MLP-014 attribution: env propagation + Linux process-tree walk.
//!
//! Env tags (`ANVIL_AGENT_TAG` / `ANVIL_TASK_ID`) are advisory — daemon must
//! cross-check issued tags. Linux `/proc` only in v1; other platforms return
//! Unsupported.

pub mod env;
pub mod process;
pub mod walk;

pub use anvil_intercept_proto::session::{ANVIL_AGENT_TAG_ENV, ANVIL_TASK_ID_ENV, AgentTag};
pub use env::{ParseAgentTagError, agent_tag_from_env_value, agent_tag_to_env_value};
pub use process::{ProcessInfoError, parent_pid, pid_starttime};
pub use walk::{WalkError, WalkOutcome, walk_ancestors};
