//! Shared domain primitives used across contract modules.
//!
//! This module centralizes common storage, accounting types, token accessors,
//! and events so business modules can compose consistent state transitions.

pub mod balance;
pub mod events;
pub mod storage;
pub mod token;
pub mod types;

pub use balance::ContractBalance;
pub use token::get_token;
pub use types::{ContractData, DataKey, InvestmentContractParams};