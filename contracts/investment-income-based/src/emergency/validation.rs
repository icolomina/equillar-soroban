use super::EmergencyCloseState;

use crate::require;
use crate::shared::types::{ContractBalance, ContractData, Error, Position};

/// Validates if an investment can be paid under emergency-close mode.
///
/// Enforces post-fundraising timing, investment activity, non-empty emergency
/// pool, and non-empty obligations.
pub(super) fn validate_emergency_payment(
    position: &Position,
    contract_balance: &ContractBalance,
    emergency_state: &EmergencyCloseState,
) -> Result<(), Error> {
    require!(
        !position.completed,
        Error::PositionCompleted,
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
pub(super) fn validate_activate_emergency_close(
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
