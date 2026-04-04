use soroban_sdk::{Address, Env, String, Symbol, contractclient, contracttype};
use stellar_contract_utils::math::wad::Wad;

use crate::{balance::ContractBalance, investment::Investment};

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

/// Calculates collateral coverage level in basis points.
///
/// Formula:
/// `level_bps = collateral_value_in_contract_token * 10_000 / payment_obligations`.
///
/// Example: `8160` means `81.60%` coverage.
///
/// Uses OpenZeppelin `Wad` helpers for decimal-safe conversion between token units.
///
/// # Returns
///
/// * `Some(level)` when pricing is available.
/// * `Some(100_000)` when `payment_obligations == 0` (sentinel high coverage value).
/// * `None` when oracle has no price for the pair.
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
        return Some(100_000_u32);
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

/// Calculates the investor's proportional collateral payout.
///
/// Uses the investor's pending obligations ratio:
/// `investment_remaining / contract_payment_obligations`,
/// then applies that ratio to current `collateral_amount`.
///
/// # Preconditions
///
/// `contract_balance.payment_obligations` must be strictly positive.
/// Callers should enforce this before invoking the function.
pub fn get_collateral_for_investment(
    env : &Env, 
    investment: &Investment, 
    contract_balance: &ContractBalance, 
    collateral_amount: i128,
    collateral_token_decimals: u32
) -> i128{

    let amount_to_paid_pending = investment.total - investment.paid;

    let investment_collateral_corresponding_ratio = Wad::from_ratio(env, amount_to_paid_pending, contract_balance.payment_obligations);
    let amount_collateral_wad = Wad::from_token_amount(env, collateral_amount, collateral_token_decimals as u8);

    let investment_corresponding_collateral_amount_wad = investment_collateral_corresponding_ratio * amount_collateral_wad;
    let investment_corresponding_collateral_amount = investment_corresponding_collateral_amount_wad.to_token_amount(env, collateral_token_decimals as u8);

    investment_corresponding_collateral_amount
}