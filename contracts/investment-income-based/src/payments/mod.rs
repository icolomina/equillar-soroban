pub mod events;

use soroban_sdk::Env;
use stellar_tokens::non_fungible::Base;

use crate::investment::{self, Investment};
use crate::shared;
use crate::validation::{self, Error};

/// Processes one periodic payment for an investment.
///
/// Validates payment timing/state, computes amount due, transfers funds to the
/// investment owner, and updates investment plus global accounting state.
///
/// # Errors
/// Returns if position is missing/completed, reserve constraints fail, or transfer fails.
pub fn process_investor_payment(env: &Env, token_id: u32) -> Result<Investment, Error> {
    let contract_data = shared::storage::get_contract_data(env);
    validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let addr = Base::owner_of(env, token_id);
    let mut investment = investment::storage::get_investment(env, token_id)
        .ok_or(Error::AddressHasNotInvested)?;
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    if investment.completed {
        return Err(Error::InvestmentCompleted);
    }

    let token = shared::get_token(env, &contract_data);
    let next_payment_round = shared::storage::get_next_payment_round(env);
    let amount_to_transfer = investment.process_investment_payment(&contract_data);

    validation::validate_reserve_balance(
        amount_to_transfer,
        &investment,
        &contract_balance,
        next_payment_round,
    )?;

    token
        .try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    investment::storage::set_investment(env, token_id, &investment);
    contract_balance.recalculate_from_payment_to_investor(&amount_to_transfer);
    shared::storage::update_contract_balances(env, &contract_balance);

    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_payment_sent(env, amount_to_transfer);

    Ok(investment)
}