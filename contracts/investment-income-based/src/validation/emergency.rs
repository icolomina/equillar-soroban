use crate::investment::Investment;
use crate::require;
use crate::emergency::EmergencyCloseState;
use crate::shared::{ContractBalance, ContractData};
use crate::validation::Error;

/// Validates if an investment can be paid under emergency-close mode.
///
/// Enforces post-fundraising timing, investment activity, non-empty emergency
/// pool, and non-empty obligations.
pub fn validate_emergency_payment(
    investment: &Investment,
    contract_balance: &ContractBalance,
    emergency_state: &EmergencyCloseState,
    current_ts: u64,
    contract_data: &ContractData,
) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet,
        !investment.completed,
        Error::InvestmentCompleted,
        emergency_state.emergency_pool_remaining > 0,
        Error::EmptyEmergencyPool,
        emergency_state.emergency_obligations_left > 0,
        Error::EmptyPaymentObligations,
        contract_balance.payment_obligations > 0,
        Error::EmptyPaymentObligations
    );
    Ok(())
}

/// Validates whether emergency-close mode can be activated.
///
/// Requires fundraising to be finished, no active emergency state, no pending
/// commission withdrawal, and a non-empty distributable pool/obligations.
pub fn validate_activate_emergency_close(
    current_ts: u64,
    contract_data: &ContractData,
    contract_balance: &ContractBalance,
    emergency_state: Option<EmergencyCloseState>,
) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet,
        emergency_state.is_none(),
        Error::EmergencyAlreadyActive,
        contract_balance.comission == contract_balance.comission_withdrawal,
        Error::PendingCommissionWithdrawal,
        contract_balance.reserve + contract_balance.project > 0,
        Error::EmptyEmergencyPool,
        contract_balance.payment_obligations > 0,
        Error::EmptyPaymentObligations
    );
    Ok(())
}

/// Returns active emergency state or `EmergencyNotActive`.
pub fn validate_emergency_active(
    emergency_state: Option<EmergencyCloseState>,
) -> Result<EmergencyCloseState, Error> {
    emergency_state.ok_or(Error::EmergencyNotActive)
}