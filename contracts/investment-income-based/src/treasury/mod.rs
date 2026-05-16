pub mod events;

use soroban_sdk::{Address, Env};

use crate::shared;
use crate::validation::{self, Error};

/// Withdraws project funds from contract treasury to a company address.
///
/// Applies timing and balance checks, performs token transfer, and updates
/// accounting snapshots/events.
///
/// # Errors
/// Returns withdrawal validation errors and transfer failures.
pub fn withdrawn(env: &Env, amount: i128, to: Address) -> Result<(), Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    validation::validate_withdrawal(
        amount,
        contract_balance.project,
        env.ledger().timestamp(),
        &contract_data,
    )?;

    let token = shared::get_token(env, &contract_data);
    token
        .try_transfer(&env.current_contract_address(), &to, &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    contract_balance.recalculate_from_company_withdrawal(&amount);
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_withdrawal_done(env, amount);

    Ok(())
}

/// Withdraws currently available commissions to a manager address.
///
/// Computes pending commission amount, enforces withdrawal constraints, performs
/// transfer, and updates commission-withdrawal accounting.
///
/// # Errors
/// Returns commission validation errors and transfer failures.
pub fn withdrawn_commissions(env: &Env, to: Address) -> Result<i128, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    let token = shared::get_token(env, &contract_data);
    let amount_to_withdraw = contract_balance.comission - contract_balance.comission_withdrawal;
    validation::validate_withdrawal_commission(
        amount_to_withdraw,
        env.ledger().timestamp(),
        &contract_data,
    )?;

    token
        .try_transfer(&env.current_contract_address(), &to, &amount_to_withdraw)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    contract_balance.recalculate_from_comission_withdrawal(&amount_to_withdraw);
    shared::storage::update_contract_balances(env, &contract_balance);
    events::emit_commission_withdrawn(env, amount_to_withdraw);

    Ok(amount_to_withdraw)
}

/// Registers and transfers a company contribution for the upcoming payment round.
///
/// Validates round timing and reserve rules, transfers funds from `from`, then
/// updates reserves and increments the tracked payment round.
///
/// # Errors
/// Returns company-transfer validation errors and transfer failures.
pub fn add_company_transfer(env: &Env, from: Address, amount: i128) -> Result<bool, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    let token = shared::get_token(env, &contract_data);
    let next_payment_round = shared::storage::get_next_payment_round(env);

    validation::validate_company_transfer(
        env,
        &token,
        &from,
        &contract_data,
        &contract_balance,
        amount,
        next_payment_round,
    )?;

    token
        .try_transfer(&from, &env.current_contract_address(), &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    contract_balance.recalculate_from_company_contribution(&amount);
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_company_transfer_received(env, amount);
    shared::storage::incr_next_payment_round(env);

    Ok(true)
}