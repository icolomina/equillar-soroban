use soroban_sdk::Env;
use stellar_contract_utils::math::wad::Wad;

use crate::{
    investment::types::DepositAllocation,
    shared::types::{ContractData, Position, PositionReturnType},
};

/// Computes commission denominator tier based on token amount.
///
/// Larger investments increase the denominator (thus reducing commission rate)
/// up to the configured upper bound.
fn calculate_rate_denominator(
    amount: &i128,
    cmr_upper_divisor: u32,
    cmr_lower_divisor: u32,
    cmr_reductor: &i128
) -> u32 {
    let step = amount / cmr_reductor; 

    if step > cmr_upper_divisor as i128 {
        return cmr_upper_divisor;
    }

    cmr_lower_divisor + step as u32
}

fn get_deposit_allocation(
    env: &Env,
    amount: &i128,
    decimals: u32,
    rate_denominator: u32,
    interest_rate: u32,
) -> DepositAllocation {
    let decimals_for_wad: u8 = decimals.try_into().expect("Token decimals must fit in u8");

    let amount_wad = Wad::from_token_amount(env, *amount, decimals_for_wad);
    let commission_rate_wad = Wad::from_ratio(
        env,
        interest_rate as i128,
        (rate_denominator as i128) * 10_000,
    );

    let return_rate_wad = Wad::from_ratio(
        env,
        interest_rate as i128,
        10_000, // o el denominador que corresponda según tu semántica de negocio
    );

    let amount_to_commission_wad = amount_wad * commission_rate_wad;
    let amount_to_invest_wad = amount_wad - amount_to_commission_wad;
    let returns_wad = amount_to_invest_wad * return_rate_wad;

    let commission = amount_to_commission_wad.to_token_amount(env, decimals_for_wad);
    let deposited = *amount - commission; // 
    let returns = returns_wad.to_token_amount(env, decimals_for_wad);

    DepositAllocation {
        commission,
        returns,
        deposited,
    }
}

fn calculate_regular_payment(
    deposit_allocation: &DepositAllocation,
    return_type: &PositionReturnType,
    return_months: u32,
) -> i128 {
    match return_type {
        PositionReturnType::Coupon => deposit_allocation.returns / return_months as i128,
        PositionReturnType::ReverseLoan => {
            deposit_allocation.get_total_claimable() / return_months as i128
        }
    }
}

pub fn create_position(
    env: &Env,
    cd: &ContractData,
    amount: &i128,
    decimals: u32,
    token_id: u32,
) -> Position {
    let rate_denominator = calculate_rate_denominator(
        amount,
        cd.cmr_upper_divisor,
        cd.cmr_lower_divisor,
        &cd.cmr_reductor,
    );

    let deposit_allocation =
        get_deposit_allocation(env, amount, decimals, rate_denominator, cd.interest_rate);

    let regular_payment =
        calculate_regular_payment(&deposit_allocation, &cd.return_type, cd.return_months);

    Position {
        deposited: deposit_allocation.deposited,
        commission: deposit_allocation.commission,
        returns: deposit_allocation.returns,
        total: deposit_allocation.get_total_claimable(),
        completed: false,
        regular_payment,
        paid: 0_i128,
        payments_transferred: 0_u32,
        token_id,
    }
}
