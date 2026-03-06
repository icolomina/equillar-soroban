use soroban_sdk::{contractclient, contracttype, Address, Env, String, Symbol, log};
use stellar_contract_utils::math::wad::Wad;

#[contracttype]
#[derive(Clone)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorClient")]
pub trait ReflectorOracle {
    fn decimals(env: &Env) -> u32;
    fn x_last_price(env: &Env, base_asset: Asset, quote_asset: Asset) -> Option<PriceData>;
}

#[contracttype]
pub struct Collateral {
    pub token_collateral_address: Address,
    pub token_collateral_symbol: String,
    pub address_collateral_token: Address,
    pub collateral_amount: i128,
    pub collateral_level: u32,
}

/// Calculates the collateral coverage level as a percentage (basis points, 2 decimals).
/// 
/// Formula: (collateral_amount * collateral_price) / payment_obligations
/// Returns a value in basis points, e.g. 8160 = 81.60%
/// 
/// Wad "from_token_amount" and "to_token_amount" functions requires u8 numbers. Althougth this function receives
/// both collateral_decimals and contract_token_decimals as u32, they will fit into u8 numbers son decimal tokens will never exceed
/// 255
/// 
/// Returns None if:
/// - The oracle has no price for the asset
/// - payment_obligations is zero (no investors yet)
pub fn calculate_collateral_level(
    env: &Env,
    oracle_addr: &Address,
    collateral_token_addr: &Address,
    collateral_amount: i128,
    collateral_decimals: u32,
    contract_token_addr: &Address,
    contract_token_decimals: u32,
    payment_obligations: i128,
) -> Option<u32> {
    if payment_obligations == 0 {
        return None;
    }

    let oracle = ReflectorClient::new(env, oracle_addr);
    let oracle_decimals = oracle.decimals();

    let price_data = oracle.x_last_price(
        &Asset::Stellar(collateral_token_addr.clone()),
        &Asset::Stellar(contract_token_addr.clone()),
    )?;
    let price_wad = Wad::from_token_amount(env, price_data.price, oracle_decimals as u8);

    let collateral_wad = Wad::from_token_amount(env, collateral_amount, collateral_decimals as u8);
    let collateral_value_wad = collateral_wad * price_wad;
    let collateral_value = collateral_value_wad.to_token_amount(env, contract_token_decimals as u8);

    let level = collateral_value * 10_000_i128 / payment_obligations;

    Some(level as u32)
}