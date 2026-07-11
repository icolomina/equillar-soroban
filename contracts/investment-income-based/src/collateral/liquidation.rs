use soroban_sdk::{Address, Env};
use stellar_contract_utils::math::wad::Wad;

use crate::shared::{
    oracle::{Asset, ReflectorClient},
    types::{ContractBalance, Position},
};

fn get_collateral_cross_price(
    env: &Env,
    collateral_token_addr: &Address,
    contract_token_addr: &Address,
    oracle_addr: &Address,
) -> Option<Wad> {
    let oracle = ReflectorClient::new(env, oracle_addr);
    let collateral_price_data = oracle.lastprice(&Asset::Stellar(collateral_token_addr.clone()))?;

    // Contract token price in terms of the same base asset
    let contract_token_price_data =
        oracle.lastprice(&Asset::Stellar(contract_token_addr.clone()))?;

    // Price cross: collateral / contract_token,
    // both expressed in the oracle's base asset, at the same scale (oracle_decimals)
    let cross_price_wad = Wad::from_ratio(
        env,
        collateral_price_data.price,
        contract_token_price_data.price,
    );

    Some(cross_price_wad)
}

/// Computes the collateral amount attributable to a specific investment.
///
/// Allocation is proportional to remaining obligations of the position over
/// total contract payment obligations.
pub(super) fn get_collateral_for_investment(
    env: &Env,
    position: &Position,
    contract_balance: &ContractBalance,
    collateral_amount: i128,
    collateral_token_addr: &Address,
    collateral_token_decimals: u32,
    contract_token_addr: &Address,
    contract_token_decimals: u32,
    oracle_addr: &Address,
) -> Option<i128> {
    let mut required_collateral_amount_wad =
        Wad::from_token_amount(env, collateral_amount, collateral_token_decimals as u8);

    let cross_price_wad =
        get_collateral_cross_price(env, collateral_token_addr, contract_token_addr, oracle_addr)?;

    let remaining_obligations_wad = Wad::from_token_amount(
        env,
        contract_balance.payment_obligations,
        contract_token_decimals as u8,
    );

    let collateral_value_wad = required_collateral_amount_wad * cross_price_wad;
    if collateral_value_wad > remaining_obligations_wad {
        required_collateral_amount_wad = remaining_obligations_wad / cross_price_wad;
    }

    let amount_to_paid_pending = position.total - position.paid;
    let investment_collateral_corresponding_ratio = Wad::from_ratio(
        env,
        amount_to_paid_pending,
        contract_balance.payment_obligations,
    );

    let investment_corresponding_collateral_amount_wad =
        investment_collateral_corresponding_ratio * required_collateral_amount_wad;
    Some(
        investment_corresponding_collateral_amount_wad
            .to_token_amount(env, collateral_token_decimals as u8),
    )
}

/// Calculates collateral coverage level in basis points (x100).
///
/// Returns `Some(10_000)` when payment obligations are zero. Otherwise,
/// converts collateral value through oracle price and normalizes to contract
/// token decimals.
///
/// # Returns
/// `None` when oracle price is unavailable.
pub(super) fn calculate_collateral_level(
    env: &Env,
    oracle_addr: &Address,
    contract_token_addr: &Address,
    contract_token_decimals: u32,
    collateral_token_addr: &Address,
    collateral_amount: i128,
    collateral_token_decimals: u32,
    payment_obligations: i128,
) -> Option<u32> {
    if payment_obligations == 0 {
        return Some(10_000);
    }

    let cross_price_wad =
        get_collateral_cross_price(env, collateral_token_addr, contract_token_addr, oracle_addr)?;

    let collateral_wad =
        Wad::from_token_amount(env, collateral_amount, collateral_token_decimals as u8);
    let collateral_value_wad = collateral_wad * cross_price_wad;
    let collateral_value = collateral_value_wad.to_token_amount(env, contract_token_decimals as u8);

    Some((collateral_value * 10_000_i128 / payment_obligations) as u32)
}
