use crate::investment::Investment;
use crate::require;
use crate::shared::ContractBalance;
use crate::validation::Error;

/// Validates reserve-backed periodic payment constraints.
///
/// Ensures the position has not already been paid for the current round and
/// reserve can cover the computed amount.
pub fn validate_reserve_balance(
    amount_to_transfer: i128,
    investment: &Investment,
    contract_balances: &ContractBalance,
    next_payment_round: u32,
) -> Result<(), Error> {
    require!(
        investment.payments_transferred == next_payment_round,
        Error::PaymentAlreadyProcessedForThisPeriod,
        amount_to_transfer <= contract_balances.reserve,
        Error::ContractReserveInsufficientBalance
    );
    Ok(())
}