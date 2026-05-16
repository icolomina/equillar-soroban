use crate::investment::Investment;
use crate::require;
use crate::emergency::EmergencyCloseState;
use crate::shared::{ContractBalance, ContractData};
use crate::validation::Error;

/// Validates constructor economic parameters.
///
/// Ensures all configured values are strictly positive.
pub fn validate_constructor_params(
    i_rate: u32,
    goal: i128,
    return_months: u32,
    min_per_investment: i128,
) -> Result<(), Error> {
    require!(
        i_rate > 0, Error::InterestRateMustBeGreaterThanZero,
        goal > 0, Error::GoalMustBeGreaterThanZero,
        return_months > 0, Error::ReturnMonthsMustBeGreaterThanZero,
        min_per_investment > 0, Error::MinPerInvestmentMustBeGreaterThanZero
    );
    Ok(())
}

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
        contract_balance.received_so_far < contract_data.goal, Error::GoalReached,
        amount >= contract_data.min_per_investment, Error::AmountLessThanMinimum,
        investor_balance >= amount, Error::AddressInsufficientBalance,
        current_ts < contract_data.ts_fundraising_ends, Error::FundrasingTimeExceeded
    );
    Ok(())
}

/// Validates whether an investment can be refunded.
///
/// Refund is allowed only before fundraising end, for non-completed positions,
/// and with non-empty refund amount.
pub fn validate_refund_investor(
    investment: &Investment,
    contract_data: &ContractData,
    amount_to_refund: i128,
    current_ts: u64,
) -> Result<(), Error> {
    require!(
        current_ts < contract_data.ts_fundraising_ends, Error::FundrasingTimeExceeded,
        !investment.completed, Error::InvestmentCompleted,
        amount_to_refund > 0, Error::EmptyRefundAmount
    );
    Ok(())
}

/// Blocks operations while emergency mode is active.
pub fn validate_not_in_emergency(
    emergency_state: Option<EmergencyCloseState>,
) -> Result<(), Error> {
    require!(emergency_state.is_none(), Error::OperationNotAllowedInEmergency);
    Ok(())
}