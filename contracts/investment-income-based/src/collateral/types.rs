use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone)]
pub(super) struct Collateral {
    pub token_collateral_address: Address,
    pub token_collateral_symbol: String,
    pub address_collateral_token: Address,
    pub collateral_amount: i128,
    pub collateral_level: u32,
}
