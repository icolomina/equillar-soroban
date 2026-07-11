mod events;
mod validation;
mod types;
mod storage;
mod liquidation;

use soroban_sdk::{token::Client as TokenClient, Address, Env, String};

use types::Collateral;
use crate::collateral::liquidation::{calculate_collateral_level, get_collateral_for_investment};
use crate::require;
use crate::shared;
use crate::shared::token::get_token;
use crate::shared::types::Error;

fn get_collateral_token<'a>(env: &'a Env, collateral_token: &Address) -> TokenClient<'a> {
    TokenClient::new(env, collateral_token)
}

/// Deposits collateral and updates persisted collateral metadata.
///
/// Supports topping up existing collateral only for the same token and
/// recalculates collateral level against current obligations.
///
/// # Errors
/// Returns collateral validation errors, transfer failures, or low collateral level.
pub(crate) fn add_collateral(
    env: &Env,
    collateral_token_addr: Address,
    collateral_token_amount: i128,
    collateral_token_symbol: String,
    collateral_addr: Address,
) -> Result<u32, Error> {
    let existing_collateral = storage::get_collateral(env);
    let collateral_token_client = get_collateral_token(env, &collateral_token_addr);
    validation::validate_add_collateral(
        existing_collateral.clone(),
        existing_collateral
            .as_ref()
            .map(|coll| coll.token_collateral_address == collateral_token_addr)
            .unwrap_or(true),
        collateral_token_client.balance(&collateral_addr) >= collateral_token_amount,
    )?;

    let current_collateral_token_amount =
        collateral_token_client.balance(&env.current_contract_address());

    let mut contract_balances = shared::storage::get_balances_or_new(env);
    contract_balances.recalculate_from_collateral_received(collateral_token_amount)?;

    let contract_data = shared::storage::get_contract_data(env);
    let contract_token_client = shared::token::get_token(env, &contract_data);
    let total_collateral_amount = collateral_token_amount
        .checked_add(current_collateral_token_amount)
        .ok_or(Error::CollateralAmountOverflow)?;

    if let Some(level) = calculate_collateral_level(
        env,
        &contract_data.price_oracle,
        &collateral_token_addr,
        contract_token_client.decimals(),
        &collateral_token_client.address,
        total_collateral_amount,
        contract_token_client.decimals(),
        contract_balances.payment_obligations,
    ) {

        collateral_token_client
            .try_transfer(&collateral_addr, &env.current_contract_address(), &collateral_token_amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let collateral = Collateral {
            token_collateral_address: collateral_token_addr,
            token_collateral_symbol: collateral_token_symbol,
            address_collateral_token: collateral_addr,
            collateral_amount: total_collateral_amount,
            collateral_level: level,
        };
        storage::update_collateral(env, &collateral);
        shared::storage::update_contract_balances(env, &contract_balances);

        events::emit_collateral_deposited(
            env,
            collateral.collateral_amount,
            collateral_token_amount,
            &collateral.token_collateral_address,
            &collateral.token_collateral_symbol
        );
        Ok(level)
    } else {
        Err(Error::CollateralLevelTooLow)
    }
}

/// Pays one investment using available collateral and closes that position.
///
/// Transfers the computed proportional collateral amount to the owner,
/// marks investment completed, and updates obligations/balances.
///
/// # Errors
/// Returns if investment/collateral is missing, already completed, or transfer fails.
pub(crate) fn pay_with_collateral(env: &Env, position_id: u32) -> Result<i128, Error> {
    let contract_data = &shared::storage::get_contract_data(env);
    let mut position = shared::storage::get_position(env, position_id)
        .ok_or(Error::AddressHasNotInvested)?;
    require!(!position.completed, Error::PositionCompleted);

    let collateral = storage::get_collateral(env).ok_or(Error::CollateralNotConfigured)?;
    let collateral_token = get_collateral_token(env, &collateral.token_collateral_address);
    let contract_token = get_token(env, contract_data);
    let investor_addr = shared::storage::get_addr_position_id(env, position_id).ok_or(Error::AddressHasNotInvested)?;

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    let collateral_amount = get_collateral_for_investment(
        env,
        &position,
        &contract_balance,
        collateral_token.balance(&env.current_contract_address()),
        &collateral_token.address,
        collateral_token.decimals(),
        &contract_token.address,
        contract_token.decimals(),
        &contract_data.price_oracle
    ).ok_or(Error::CollateralPriceOracleError)?;

    collateral_token
        .try_transfer(&env.current_contract_address(), &investor_addr, &collateral_amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let remaining_obligations = position.total - position.paid;
    position.completed = true;
    shared::storage::set_position(env, position_id, &position);
    contract_balance.recalculate_from_collateral_liquidated(collateral_amount, remaining_obligations)?;
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_collateral_sent(env, investor_addr, collateral_amount);
    Ok(collateral_amount)
}

/// Returns remaining collateral balance to configured collateral provider.
///
/// # Errors
/// Returns when collateral is not configured/empty or token transfer fails.
pub(crate) fn return_collateral_to_company(env: &Env) -> Result<i128, Error> {
    let coll = storage::get_collateral(env).ok_or(Error::CollateralNotConfigured)?;
    let collateral_token = get_collateral_token(env, &coll.token_collateral_address);
    let collateral_contract_balance = collateral_token.balance(&env.current_contract_address());
    validation::validate_collateral_return(collateral_contract_balance)?;

    collateral_token
        .try_transfer(
            &env.current_contract_address(),
            &coll.address_collateral_token,
            &collateral_contract_balance,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    contract_balance.recalculate_from_collateral_returned(collateral_contract_balance)?;
    shared::storage::update_contract_balances(env, &contract_balance);
    events::emit_collateral_returned(env, coll.address_collateral_token, collateral_contract_balance);
    Ok(collateral_contract_balance)
}