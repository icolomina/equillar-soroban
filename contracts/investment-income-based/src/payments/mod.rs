mod events;
mod storage;
mod validation;

use soroban_sdk::{Address, Env};

use crate::shared;
use crate::shared::types::{Error, LiquidateInvestmentsStatus, Position, PositionReturnType};
use crate::validation::{self as shared_validation};

fn update_position_for_payment_round(
    position: &mut Position,
    return_type: PositionReturnType,
    return_months: u32,
) -> i128 {
    let total = match return_type {
        PositionReturnType::Coupon => position.returns,
        PositionReturnType::ReverseLoan => position.total,
    };

    let mut amount_to_transfer: i128;
    position.payments_transferred += 1;

    if position.payments_transferred >= return_months {
        position.completed = true;
        amount_to_transfer = total - position.paid;

        if return_type == PositionReturnType::Coupon {
            amount_to_transfer += position.deposited;
        }
    } else {
        amount_to_transfer = position.regular_payment;
    }

    position.paid += amount_to_transfer;
    amount_to_transfer
}

/// Processes one periodic payment for an investment.
///
/// Validates payment timing/state, computes amount due, transfers funds to the
/// investment owner, and updates investment plus global accounting state.
///
/// Note: If `LiquidateInvestmentEnabled` key is not enabled, this function updates the position but
/// does not check reserve neither transfer the payment. It is assumed that payment will be sent in other ways
/// chosen by the integrator
///
/// # Errors
/// Returns if position is missing/completed, reserve constraints fail, or transfer fails.
pub(crate) fn process_investor_payment(env: &Env, position_id: u32) -> Result<Position, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    shared_validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let addr = shared::storage::get_addr_position_id(env, position_id)
        .ok_or(Error::AddressHasNotInvested)?;
    let mut position =
        shared::storage::get_position(env, position_id).ok_or(Error::AddressHasNotInvested)?;
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    if position.completed {
        return Err(Error::PositionCompleted);
    }

    let token = shared::token::get_token(env, &contract_data);
    let next_payment_round = storage::get_next_payment_round(env);
    let amount_to_transfer = update_position_for_payment_round(
        &mut position,
        contract_data.return_type,
        contract_data.return_months,
    );

    if LiquidateInvestmentsStatus::Enabled == storage::liquidate_investments_enabled(env) {
        validation::validate_reserve_balance(
            amount_to_transfer,
            &position,
            &contract_balance,
            next_payment_round,
        )?;

        token
            .try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;
    }

    shared::storage::set_position(env, position_id, &position);
    contract_balance.recalculate_from_payment_to_investor(amount_to_transfer)?;
    shared::storage::update_contract_balances(env, &contract_balance);

    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_payment_sent(env, amount_to_transfer);

    Ok(position)
}

/// Registers and transfers a company contribution for the upcoming payment round.
///
/// Validates round timing and reserve rules, transfers funds from `from`, then
/// updates reserves and increments the tracked payment round.
///
/// # Errors
/// Returns company-transfer validation errors and transfer failures.
pub(crate) fn add_company_transfer(env: &Env, from: &Address, amount: i128) -> Result<bool, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    shared_validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    let token = shared::token::get_token(env, &contract_data);
    let next_payment_round = storage::get_next_payment_round(env);

    validation::validate_company_transfer(
        env,
        &token,
        from,
        &contract_data,
        &contract_balance,
        amount,
        next_payment_round,
    )?;

    token
        .try_transfer(from, env.current_contract_address(), &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    contract_balance.recalculate_from_company_contribution(amount)?;
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_company_transfer_received(env, amount);
    storage::incr_next_payment_round(env);

    Ok(true)
}

pub(crate) fn enable_liquidate_investments(env: &Env) -> Result<(), Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_enable_disable_investment_liquidations(
        env.ledger().timestamp(),
        contract_data.ts_fundraising_ends,
        contract_data.ts_payments_start,
    )?;
    storage::enable_liquidate_investments(env);
    Ok(())
}

pub(crate) fn disable_liquidate_investments(env: &Env) -> Result<(), Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_enable_disable_investment_liquidations(
        env.ledger().timestamp(),
        contract_data.ts_fundraising_ends,
        contract_data.ts_payments_start,
    )?;
    storage::disable_liquidate_investments(env);
    Ok(())
}

pub(crate) fn check_investment_liquidations(env: &Env) -> LiquidateInvestmentsStatus {
    let status: LiquidateInvestmentsStatus =
        if storage::liquidate_investments_enabled(env) == LiquidateInvestmentsStatus::Enabled {
            LiquidateInvestmentsStatus::Enabled
        } else {
            LiquidateInvestmentsStatus::Disabled
        };

    status
}
