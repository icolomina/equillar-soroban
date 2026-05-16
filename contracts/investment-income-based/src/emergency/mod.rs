pub mod events;
pub mod types;

use soroban_sdk::Env;
use stellar_tokens::non_fungible::Base;

use crate::investment;
use crate::require;
use crate::shared;
use crate::validation::{self, Error};

pub use types::EmergencyCloseState;

/// Activates emergency-close mode and snapshots emergency distribution state.
///
/// Captures emergency pool and obligations at activation time so subsequent
/// payouts are computed against a fixed baseline.
///
/// # Errors
/// Returns if activation preconditions are not met.
pub fn activate_emergency_close(env: &Env) -> Result<bool, Error> {
    let contract_data = shared::storage::get_contract_data(env);

    let contract_balance = shared::storage::get_balances_or_new(env);
    validation::validate_activate_emergency_close(
        env.ledger().timestamp(),
        &contract_data,
        &contract_balance,
        shared::storage::get_emergency_close_state(env),
    )?;

    let emergency_obligations_total = contract_balance.payment_obligations;
    let emergency_state = EmergencyCloseState::from_contract_balance(&contract_balance);

    shared::storage::set_emergency_close_state(env, &emergency_state);
    events::emit_emergency_close_activated(
        env,
        emergency_state.emergency_pool_total,
        emergency_obligations_total,
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
pub fn emergency_pay_investor(env: &Env, token_id: u32) -> Result<i128, Error> {
    let contract_data = shared::storage::get_contract_data(env);

    let mut investment = investment::storage::get_investment(env, token_id)
        .ok_or(Error::AddressHasNotInvested)?;
    let mut contract_balance = shared::storage::get_balances_or_new(env);
    require!(
        env.ledger().timestamp() > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet
    );
    let mut emergency_state = shared::storage::get_emergency_close_state(env)
        .ok_or(Error::EmergencyNotActive)?;

    validation::validate_emergency_payment(
        &investment,
        &contract_balance,
        &emergency_state,
        env.ledger().timestamp(),
        &contract_data,
    )?;

    let remaining_obligations = investment.total - investment.paid;
    let amount_to_pay = emergency_state.calculate_amount_to_pay(&investment);

    let token_owner = Base::owner_of(env, token_id);
    let token = shared::get_token(env, &contract_data);

    if amount_to_pay > 0 {
        token
            .try_transfer(&env.current_contract_address(), &token_owner, &amount_to_pay)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;
    }

    investment.completed = true;
    emergency_state.update_after_payment(&amount_to_pay, &remaining_obligations);

    investment::storage::set_investment(env, token_id, &investment);
    contract_balance.recalculate_from_emergency_payment(&amount_to_pay, &remaining_obligations);
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::storage::set_emergency_close_state(env, &emergency_state);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_emergency_payment_sent(env, token_owner, amount_to_pay);

    Ok(amount_to_pay)
}