use crate::require;
use crate::shared::types::Position;
use crate::shared::types::{ContractBalance, ContractData, Error};

/// Validates whether a new investment can be accepted at current state.
///
/// Checks goal cap, minimum amount, investor balance, and fundraising window.
pub fn validate_investment(
    amount: i128,
    contract_data: &ContractData,
    investor_balance: i128,
    current_ts: u64,
    contract_balance: &ContractBalance,
) -> Result<(), Error> {
    require!(
        contract_balance.project < contract_data.goal,
        Error::GoalReached,
        amount >= contract_data.min_per_investment,
        Error::AmountLessThanMinimum,
        investor_balance >= amount,
        Error::AddressInsufficientBalance,
        current_ts < contract_data.ts_fundraising_ends,
        Error::FundrasingTimeExceeded
    );
    Ok(())
}

/// Validates whether an investment can be refunded.
///
/// Refund is allowed only before fundraising end, for non-completed positions,
/// and with non-empty refund amount.
pub fn validate_refund_investor(
    position: &Position,
    contract_data: &ContractData,
    amount_to_refund: i128,
    current_ts: u64,
) -> Result<(), Error> {
    require!(
        current_ts < contract_data.ts_fundraising_ends,
        Error::FundrasingTimeExceeded,
        !position.completed,
        Error::PositionCompleted,
        amount_to_refund > 0,
        Error::EmptyRefundAmount
    );
    Ok(())
}
