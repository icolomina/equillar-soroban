use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env};

use crate::require;
use crate::shared::types::ContractData;
use crate::shared::types::{Error, Position};

/// Validates reserve-backed periodic payment constraints.
///
/// Ensures the position has not already been paid for the current round and
/// reserve can cover the computed amount.
pub fn validate_reserve_balance(
    amount_to_transfer: i128,
    position: &Position,
    contract_liquidity: i128,
    next_payment_round: u32,
) -> Result<(), Error> {
    require!(
        position.payments_transferred == next_payment_round,
        Error::PaymentAlreadyProcessedForThisPeriod,
        amount_to_transfer <= contract_liquidity,
        Error::ContractReserveInsufficientBalance
    );
    Ok(())
}

/// Validates company transfer constraints for payment-round funding.
///
/// Enforces payment schedule start, reserve sufficiency (with special handling
/// on final round), and source address token balance.
pub fn validate_company_transfer(
    env: &Env,
    token: &TokenClient,
    owner: &Address,
    contract_data: &ContractData,
    amount: i128,
) -> Result<(), Error> {
    let current_ts = env.ledger().timestamp();
    require!(
        current_ts >= contract_data.ts_payments_start,
        Error::NextPaymentCannotBeScheduledYet
    );

    require!(
        amount + token.balance(&env.current_contract_address())
            >= contract_data.amount_to_pay_per_month,
        Error::ContractReserveInsufficientBalance
    );

    require!(
        token.balance(owner) >= amount,
        Error::OwnerInsufficientBalance
    );
    Ok(())
}

pub fn validate_enable_disable_investment_liquidations(
    current_ts: u64,
    ts_fundraising_ends: u64,
    ts_payments_starts: u64,
) -> Result<(), Error> {
    require!(
        current_ts > ts_fundraising_ends && current_ts < ts_payments_starts,
        Error::LiquidationPaymentsOutOfPeriod
    );
    Ok(())
}
