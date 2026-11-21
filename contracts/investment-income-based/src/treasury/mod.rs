mod events;
mod validation;

use soroban_sdk::{Address, Env};

use crate::require;
use crate::shared::{self, types::Error};
use crate::validation::{self as shared_validation};

/// Withdraws project funds from contract treasury to a company address.
///
/// Applies timing and balance checks, performs token transfer, and updates
/// accounting snapshots/events.
///
/// # Errors
/// Returns withdrawal validation errors and transfer failures.
pub fn withdrawn(env: &Env, amount: i128, to: &Address) -> Result<(), Error> {
    let contract_data = shared::storage::get_contract_data(env);
    shared_validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let mut balance = shared::storage::get_balance_or_new(env);
    let token = shared::token::get_token(env, &contract_data);
    validation::validate_withdrawal(
        amount,
        balance.assets.managed_volume - balance.assets.contract_earnings,
        token.balance(&env.current_contract_address()),
        env.ledger().timestamp(),
        &contract_data,
    )?;

    token
        .try_transfer(&env.current_contract_address(), to, &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    balance.recalculate_from_company_withdrawal(amount)?;
    shared::storage::update_balance(env, &balance);
    shared::events::emit_balance_updated_event(env, &balance);
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
pub fn withdrawn_commissions(env: &Env, to: &Address) -> Result<i128, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    shared_validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;
    let mut balance = shared::storage::get_balance_or_new(env);
    let amount_to_withdrawn = balance.equity.contract_commissions;

    let token = shared::token::get_token(env, &contract_data);
    validation::validate_withdrawal_commission(
        amount_to_withdrawn,
        token.balance(&env.current_contract_address()),
        env.ledger().timestamp(),
        &contract_data,
    )?;

    token
        .try_transfer(&env.current_contract_address(), to, &amount_to_withdrawn)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    balance.recalculate_from_comission_withdrawal(amount_to_withdrawn)?;
    shared::storage::update_balance(env, &balance);
    shared::events::emit_balance_updated_event(env, &balance);
    events::emit_commission_withdrawn(env, amount_to_withdrawn);

    Ok(amount_to_withdrawn)
}

/// Withdraws the remaining token balance locked in the contract
///
/// Resets the balance
///
/// # Errors
/// Fails if there are payment obligations yet
pub fn withdrawn_all(env: &Env, to: &Address) -> Result<i128, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    let token = shared::token::get_token(env, &contract_data);
    let contract_token_balance = token.balance(&env.current_contract_address());
    let mut balance = shared::storage::get_balance_or_new(env);

    require!(
        balance.assets.managed_collateral == 0, 
        Error::CollateralStillRemainingOnContract,
        shared::storage::get_emergency_close_state(env).is_some(),
        Error::EmergencyNotActive
    );

    if contract_token_balance > 0 {
        token
            .try_transfer(&env.current_contract_address(), to, &contract_token_balance)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;
    }

    balance.reset();
    shared::storage::update_balance(env, &balance);
    shared::events::emit_balance_updated_event(env, &balance);
    events::emit_all_withdrawn(env, contract_token_balance);

    Ok(contract_token_balance)
}
