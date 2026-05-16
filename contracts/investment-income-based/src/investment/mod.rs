pub mod allocation;
pub mod events;
pub mod storage;
pub mod types;

pub use allocation::InvestmentAllocation;
pub use types::{Investment, InvestmentReturnType};

use soroban_sdk::{Address, Env};
use stellar_tokens::non_fungible::Base;

use crate::shared;
use crate::validation::{self, Error};

/// Creates and stores a new investment position.
///
/// This function validates business constraints, transfers investor funds to
/// the contract, mints the NFT receipt, and updates accounting state.
///
/// # Errors
/// Returns investment validation errors and transfer failures.
pub fn invest(env: &Env, investor: &Address, amount: i128) -> Result<Investment, Error> {
    validation::validate_not_in_emergency(shared::storage::get_emergency_close_state(env))?;

    let mut contract_data = shared::storage::get_contract_data(env);
    let token = shared::get_token(env, &contract_data);
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    validation::validate_investment(
        amount,
        &contract_data,
        token.balance(investor),
        env.ledger().timestamp(),
        &contract_balance,
    )?;

    let allocation = InvestmentAllocation::from_investment(
        env,
        &amount,
        &contract_data.interest_rate,
        token.decimals(),
    );

    token.try_transfer(investor, &env.current_contract_address(), &amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let token_id = Base::sequential_mint(env, investor);
    let investment = Investment::new(&contract_data, &allocation, token_id);

    storage::set_investment(env, token_id, &investment);
    contract_balance.recalculate_from_investment(&allocation, &investment);
    contract_data.amount_to_pay_per_month += investment.regular_payment;

    shared::storage::update_contract_data(env, &contract_data);
    shared::storage::update_contract_balances(env, &contract_balance);

    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_investment_received_event(env, allocation.amount_to_invest, investment.accumulated_interests);

    if contract_balance.received_so_far >= contract_data.goal {
        events::emit_goal_reached_event(env, contract_balance.received_so_far, contract_data.goal);
    }

    Ok(investment)
}

/// Refunds an existing investment during the allowed refund phase.
///
/// Marks the position as completed and updates aggregate balances after
/// transferring the refund amount back to the NFT owner.
///
/// # Errors
/// Returns if investment does not exist, refund is not allowed, or transfer fails.
pub fn refund_investor(env: &Env, token_id: u32) -> Result<i128, Error> {
    let mut investment = storage::get_investment(env, token_id).ok_or(Error::AddressHasNotInvested)?;
    let contract_data = shared::storage::get_contract_data(env);
    let token = shared::get_token(env, &contract_data);
    let mut contract_balance = shared::storage::get_balances_or_new(env);

    let amount_to_refund = investment.get_amount_to_refund();
    let investment_owner = Base::owner_of(env, token_id);

    validation::validate_refund_investor(
        &investment,
        &contract_data,
        amount_to_refund,
        env.ledger().timestamp(),
    )?;

    token
        .try_transfer(&env.current_contract_address(), &investment_owner, &amount_to_refund)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    investment.completed = true;
    contract_balance.recalculate_from_refunded_to_investor(&investment);
    storage::set_investment(env, token_id, &investment);
    shared::storage::update_contract_balances(env, &contract_balance);
    events::emit_investment_deposit_refunded(env, investment_owner, amount_to_refund);

    Ok(amount_to_refund)
}