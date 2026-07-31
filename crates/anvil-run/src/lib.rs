//! Process runner utilities shared by CLI commands (spawn, capture, status).

#![forbid(unsafe_code)]

pub mod blocked;
pub mod cleanup;
pub mod cli;
pub mod context;
pub mod detection;
pub mod exit_codes;
pub mod heartbeat;
pub mod hook;
pub mod ipc;
pub mod preflight;
pub mod run;
pub mod session;
pub mod spawn;
pub mod tty;
