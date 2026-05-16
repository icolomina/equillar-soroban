pub mod events;

use soroban_sdk::token::TokenClient;
use soroban_sdk::{contractclient, contracttype, token, Address, Env, String, Symbol};
use stellar_contract_utils::math::wad::Wad;
use stellar_tokens::non_fungible::Base;

use crate::investment;
use crate::require;
use crate::shared;
use crate::validation::{self, Error};

#[contracttype]
#[derive(Clone)]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorClient")]
pub trait ReflectorOracle {
    fn decimals(env: &Env) -> u32;
    fn x_last_price(env: &Env, base_asset: Asset, quote_asset: Asset) -> Option<PriceData>;
}

#[contracttype]
#[derive(Clone)]
pub struct Collateral {
    pub token_collateral_address: Address,
    pub token_collateral_symbol: String,
    pub address_collateral_token: Address,
    pub collateral_amount: i128,
    pub collateral_level: u32,
}

fn get_collateral_token<'a>(env: &'a Env, collateral_token: &Address) -> TokenClient<'a> {
    token::Client::new(env, collateral_token)
}

/// Calculates collateral coverage level in basis points (x100).
///
/// Returns `Some(100_000)` when payment obligations are zero. Otherwise,
/// converts collateral value through oracle price and normalizes to contract
/// token decimals.
///
/// # Returns
/// `None` when oracle price is unavailable.
pub fn calculate_collateral_level(
    env: &Env,
    oracle_addr: &Address,
    collateral_token_addr: &Address,
    collateral_amount: i128,
    collateral_decimals: u32,
    contract_token_addr: &Address,
    contract_token_decimals: u32,
    payment_obligations: i128,
) -> Option<u32> {
    if payment_obligations == 0 {
        return Some(100_000);
    }

    let oracle = ReflectorClient::new(env, oracle_addr);
    let oracle_decimals = oracle.decimals();

    let price_data = oracle.x_last_price(
        &Asset::Stellar(collateral_token_addr.clone()),
        &Asset::Stellar(contract_token_addr.clone()),
    )?;
    let price_wad = Wad::from_token_amount(env, price_data.price, oracle_decimals as u8);

    let collateral_wad = Wad::from_token_amount(env, collateral_amount, collateral_decimals as u8);
    let collateral_value_wad = collateral_wad * price_wad;
    let collateral_value = collateral_value_wad.to_token_amount(env, contract_token_decimals as u8);

    Some((collateral_value * 10_000_i128 / payment_obligations) as u32)
}

/// Computes the collateral amount attributable to a specific investment.
///
/// Allocation is proportional to remaining obligations of the position over
/// total contract payment obligations.
pub fn get_collateral_for_investment(
    env: &Env,
    investment: &investment::Investment,
    contract_balance: &shared::ContractBalance,
    collateral_amount: i128,
    collateral_token_decimals: u32,
) -> i128 {
    let amount_to_paid_pending = investment.total - investment.paid;
    let investment_collateral_corresponding_ratio =
        Wad::from_ratio(env, amount_to_paid_pending, contract_balance.payment_obligations);
    let amount_collateral_wad =
        Wad::from_token_amount(env, collateral_amount, collateral_token_decimals as u8);

    let investment_corresponding_collateral_amount_wad =
        investment_collateral_corresponding_ratio * amount_collateral_wad;
    investment_corresponding_collateral_amount_wad.to_token_amount(env, collateral_token_decimals as u8)
}

/// Deposits collateral and updates persisted collateral metadata.
///
/// Supports topping up existing collateral only for the same token and
/// recalculates collateral level against current obligations.
///
/// # Errors
/// Returns collateral validation errors, transfer failures, or low collateral level.
pub fn add_collateral(
    env: &Env,
    collateral_token_addr: Address,
    collateral_token_amount: i128,
    collateral_token_symbol: String,
    collateral_addr: Address,
) -> Result<Collateral, Error> {
    let existing_collateral = shared::storage::get_collateral(env);
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

    collateral_token_client
        .try_transfer(&collateral_addr, &env.current_contract_address(), &collateral_token_amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let mut contract_balances = shared::storage::get_balances_or_new(env);
    contract_balances.recalculate_from_collateral_received(&collateral_token_amount);
    shared::storage::update_contract_balances(env, &contract_balances);

    let contract_data = shared::storage::get_contract_data(env);
    let contract_token_client = shared::get_token(env, &contract_data);
    let total_collateral_amount = collateral_token_amount + current_collateral_token_amount;

    if let Some(level) = calculate_collateral_level(
        env,
        &contract_data.price_oracle,
        &collateral_token_addr,
        total_collateral_amount,
        collateral_token_client.decimals(),
        &contract_data.token,
        contract_token_client.decimals(),
        contract_balances.payment_obligations,
    ) {
        let collateral = Collateral {
            token_collateral_address: collateral_token_addr,
            token_collateral_symbol: collateral_token_symbol,
            address_collateral_token: collateral_addr,
            collateral_amount: total_collateral_amount,
            collateral_level: level,
        };
        shared::storage::update_collateral(env, &collateral);
        events::emit_collateral_deposited(
            env,
            current_collateral_token_amount,
            collateral_token_amount,
            &collateral,
        );
        Ok(collateral)
    } else {
        Err(Error::CollateralLevelTooLow)
    }
}

/// Pays one investment using available collateral and closes that position.
///
/// Transfers the computed proportional collateral amount to the NFT owner,
/// marks investment completed, and updates obligations/balances.
///
/// # Errors
/// Returns if investment/collateral is missing, already completed, or transfer fails.
pub fn pay_with_collateral(env: &Env, token_id: u32) -> Result<i128, Error> {
    let mut investment = investment::storage::get_investment(env, token_id)
        .ok_or(Error::AddressHasNotInvested)?;
    require!(!investment.completed, Error::InvestmentCompleted);

    let collateral = shared::storage::get_collateral(env).ok_or(Error::CollateralNotConfigured)?;
    let collateral_token = get_collateral_token(env, &collateral.token_collateral_address);
    let token_owner = Base::owner_of(env, token_id);

    let mut contract_balance = shared::storage::get_balances_or_new(env);
    let current_collateral_balance = collateral_token.balance(&env.current_contract_address());
    let collateral_amount = get_collateral_for_investment(
        env,
        &investment,
        &contract_balance,
        current_collateral_balance,
        collateral_token.decimals(),
    );

    collateral_token
        .try_transfer(&env.current_contract_address(), &token_owner, &collateral_amount)
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

    let remaining_obligations = investment.total - investment.paid;
    investment.completed = true;
    investment::storage::set_investment(env, token_id, &investment);
    contract_balance.recalculate_from_collateral_liquidated(&collateral_amount, &remaining_obligations);
    shared::storage::update_contract_balances(env, &contract_balance);
    shared::events::emit_balance_updated_event(env, &contract_balance);
    events::emit_collateral_sent(env, token_owner, collateral_amount);
    Ok(collateral_amount)
}

/// Returns remaining collateral balance to configured collateral provider.
///
/// # Errors
/// Returns when collateral is not configured/empty or token transfer fails.
pub fn return_collateral_to_company(env: &Env) -> Result<i128, Error> {
    let coll = shared::storage::get_collateral(env).ok_or(Error::CollateralNotConfigured)?;
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
    contract_balance.recalculate_from_collateral_returned(&collateral_contract_balance);
    shared::storage::update_contract_balances(env, &contract_balance);
    events::emit_collateral_returned(env, coll.address_collateral_token, collateral_contract_balance);
    Ok(collateral_contract_balance)
}