use soroban_sdk::{Symbol, symbol_short};

pub const SECONDS_IN_DAY: u64 = 86_400;

pub const MANAGER_ROLE: Symbol = symbol_short!("manager");
pub const COMPANY_ROLE: Symbol = symbol_short!("company");
pub const OPERATOR_ROLE: Symbol = symbol_short!("operator");