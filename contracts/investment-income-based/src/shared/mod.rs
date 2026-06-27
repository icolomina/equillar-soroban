//! Shared domain primitives used across contract modules.
//!
//! This module centralizes common storage, accounting types, token accessors,
//! and events so business modules can compose consistent state transitions.

pub mod events;
pub mod storage;
pub mod storage_helper;
pub mod token;
pub mod types;
pub mod oracle;