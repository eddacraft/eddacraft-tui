use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrustLevel {
    #[default]
    Unknown,
    Internal,
    Boundary,
    External,
    Privileged,
}
