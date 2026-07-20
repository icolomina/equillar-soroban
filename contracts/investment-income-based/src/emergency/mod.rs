mod events;
mod types;
mod validation;

use soroban_sdk::Env;

use crate::shared::{self, types::Error};

pub use types::EmergencyCloseState;

/// Activates emergency-close mode and snapshots emergency distribution state.
///
/// Captures emergency pool and obligations at activation time so subsequent
/// payouts are computed against a fixed baseline.
///
/// # Errors
/// Returns if activation preconditions are not met.
pub(crate) fn activate_emergency_close(env: &Env) -> Result<bool, Error> {
    let contract_data = shared::storage::get_contract_data(env);

    let contract_balance = shared::storage::get_balances_or_new(env);
    validation::validate_activate_emergency_close(
        env.ledger().timestamp(),
        &contract_data,
        &contract_balance,
        shared::storage::get_emergency_close_state(env),
    )?;

    let emergency_state = EmergencyCloseState::from_contract_balance(&contract_balance);

    shared::storage::set_emergency_close_state(env, &emergency_state);
    events::emit_emergency_close_activated(
        env,
        emergency_state.emergency_pool_total,
        emergency_state.emergency_obligations_left,
    );

    Ok(true)
}

/// Pays one investor from emergency pool and closes that position.
///
/// Computes amount proportionally to remaining obligations under the frozen
/// emergency snapshot, transfers funds, and updates both emergency and contract
/// accounting.
///
/// # Errors
/// Returns if emergency is inactive, investment state is invalid, or transfer fails.
pub(crate) fn emergency_pay_investor(env: &Env, position_id: u32) -> Result<i128, Error> {
    let contract_data = shared::storage::get_contract_data(env);

    let mut position =
        shared::storage::get_position(env, position_id).ok_or(Error::AddressHasNotInvested)?;
    let mut contract_balance = shared::storage::get_balances_or_new(env);
    let mut emergency_state =
        shared::storage::get_emergency_close_state(env).ok_or(Error::EmergencyNotActive)?;

    validation::validate_emergency_payment(&position, &contract_balance, &emergency_state)?;

    let remaining_obligations = position.total - position.paid;
    let amount_to_pay = emergency_state.calculate_amount_to_pay(&position);

    let token_owner = shared::storage::get_addr_position_id(env, position_id)
        .ok_or(Error::AddressHasNotInvested)?;
    let token = shared::token::get_token(env, &contract_data);

    if amount_to_pay > 0 {
        token
            .try_transfer(
                &env.current_contract_address(),
                &token_owner,
                &amount_to_pay,
            )
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;
    }

    position.completed = true;
    emergency_state.update_after_payment(&amount_to_pay, &remaining_obligations);

    shared::storage::set_position(env, position_id, &position);
    contract_balance.recalculate_from_emergency_payment(amount_to_pay, remaining_obligations)?;
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::storage::set_emergency_close_state(env, &emergency_state);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_emergency_payment_sent(env, token_owner, amount_to_pay);

    Ok(amount_to_pay)
}
