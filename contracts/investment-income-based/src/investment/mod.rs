mod allocation;
mod events;
mod types;
mod validation;

use soroban_sdk::{Address, Env};

/// shared creates modules
use crate::shared;
use crate::shared::types::{Error, Position};
use crate::validation::{self as shared_validation};

/// Creates and stores a new investment position.
///
/// This function validates business constraints, transfers investor funds to
/// the contract, mints the NFT receipt, and updates accounting state.
///
/// # Errors
/// Returns investment validation errors, existing token and transfer failures.
pub fn invest(
    env: &Env,
    investor: &Address,
    amount: i128,
    position_id: u32,
) -> Result<Position, Error> {
    if let Some(_addr) = shared::storage::get_addr_position_id(env, position_id) {
        return Err(Error::PositionIdAlreadyExists);
    }

    shared_validation::validate_not_in_emergency(
        shared::storage::get_emergency_close_state(env),
    )?;

    let mut contract_data = shared::storage::get_contract_data(env);
    let token = shared::token::get_token(env, &contract_data);
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    validation::validate_investment(
        amount,
        &contract_data,
        token.balance(investor),
        env.ledger().timestamp(),
        &contract_balance,
    )?;

    token
        .try_transfer(investor, &env.current_contract_address(), &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let position =
        allocation::create_position(env, &contract_data, &amount, token.decimals(), position_id);

    shared::storage::set_position(env, position_id, &position);
    shared::storage::set_addr_position_id(env, position_id, investor.clone());
    contract_balance.recalculate_from_position(&position);
    contract_data.amount_to_pay_per_month += position.regular_payment;

    shared::storage::update_contract_data(env, &contract_data);
    shared::storage::update_contract_balances(env, &contract_balance);

    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_investment_received_event(env, position.deposited, position.returns);

    if contract_balance.project >= contract_data.goal {
        events::emit_goal_reached_event(env, contract_balance.project, contract_data.goal);
    }

    Ok(position)
}

/// Refunds an existing investment during the allowed refund phase.
///
/// Marks the position as completed and updates aggregate balances after
/// transferring the refund amount back to the NFT owner.
///
/// # Errors
/// Returns if investment does not exist, refund is not allowed, or transfer fails.
pub fn refund_investor(env: &Env, position_id: u32) -> Result<i128, Error> {
    let mut position =
        shared::storage::get_position(env, position_id).ok_or(Error::AddressHasNotInvested)?;
    let contract_data = shared::storage::get_contract_data(env);
    let token = shared::token::get_token(env, &contract_data);
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    let amount_to_refund = position.deposited + position.commission;
    let investment_owner = shared::storage::get_addr_position_id(env, position_id)
        .ok_or(Error::AddressHasNotInvested)?;

    validation::validate_refund_investor(
        &position,
        &contract_data,
        amount_to_refund,
        env.ledger().timestamp(),
    )?;

    token
        .try_transfer(
            &env.current_contract_address(),
            &investment_owner,
            &amount_to_refund,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    position.completed = true;
    contract_balance.recalculate_from_refunded_to_investor(&position);
    shared::storage::set_position(env, position_id, &position);
    shared::storage::update_contract_balances(env, &contract_balance);
    events::emit_investment_deposit_refunded(env, investment_owner, amount_to_refund);

    Ok(amount_to_refund)
}
