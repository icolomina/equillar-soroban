use crate::require;
use crate::shared::types::{Balance, ContractData, Error};

/// Validates whether emergency-close mode can be activated.
///
/// Requires fundraising to be finished, no active emergency state, no pending
/// commission withdrawal, and a non-empty distributable pool/obligations.
pub(super) fn validate_activate_emergency_close(
    current_ts: u64,
    contract_data: &ContractData,
    balance: &Balance,
    emergency_state: Option<u32>,
) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet,
        emergency_state.is_none(),
        Error::EmergencyAlreadyActive,
        balance.equity.contract_commissions == 0_i128,
        Error::PendingCommissionWithdrawal
    );
    Ok(())
}
