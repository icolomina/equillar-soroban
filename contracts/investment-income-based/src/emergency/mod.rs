mod events;
mod validation;

use soroban_sdk::Env;

use crate::shared::{self, types::Error};

/// Activates emergency-close mode and snapshots emergency distribution state.
///
/// Captures emergency pool and obligations at activation time so subsequent
/// payouts are computed against a fixed baseline.
///
/// # Errors
/// Returns if activation preconditions are not met.
pub(crate) fn activate_emergency_close(env: &Env) -> Result<bool, Error> {
    let contract_data = shared::storage::get_contract_data(env);

    let balance = shared::storage::get_balance_or_new(env);
    validation::validate_activate_emergency_close(
        env.ledger().timestamp(),
        &contract_data,
        &balance,
        shared::storage::get_emergency_close_state(env),
    )?;

    shared::storage::set_emergency_close_state(env);
    events::emit_emergency_close_activated(env);

    Ok(true)
}
