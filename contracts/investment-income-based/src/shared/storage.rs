use soroban_sdk::Env;

use crate::collateral::Collateral;
use crate::emergency::EmergencyCloseState;
use crate::shared::{ContractBalance, ContractData, DataKey};

const DAY_IN_LEDGERS: u32 = 17_280;
const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = 15 * DAY_IN_LEDGERS;

/// Loads persisted contract configuration from instance storage.
///
/// # Panics
/// Panics when configuration is missing/expired.
pub fn get_contract_data(env: &Env) -> ContractData {
    let contract_data = env
        .storage()
        .instance()
        .get(&DataKey::ContractData)
        .unwrap_or_else(|| panic!("Contract data has expired"));

    bump_instance_ttl(env);
    contract_data
}

/// Persists contract configuration in instance storage.
pub fn update_contract_data(env: &Env, contract_data: &ContractData) {
    env.storage().instance().set(&DataKey::ContractData, contract_data);
    bump_instance_ttl(env);
}

/// Returns current balances snapshot, defaulting to a zeroed balance object.
pub fn get_balances_or_new(env: &Env) -> ContractBalance {
    let balances = env
        .storage()
        .instance()
        .get(&DataKey::ContractBalances)
        .unwrap_or_default();

    bump_instance_ttl(env);
    balances
}

/// Persists contract balances snapshot.
pub fn update_contract_balances(env: &Env, balances: &ContractBalance) {
    env.storage().instance().set(&DataKey::ContractBalances, balances);
    bump_instance_ttl(env);
}

/// Returns emergency-close state if present.
pub fn get_emergency_close_state(env: &Env) -> Option<EmergencyCloseState> {
    let state = env.storage().instance().get(&DataKey::EmergencyCloseState);
    bump_instance_ttl(env);
    state
}

/// Persists emergency-close state.
pub fn set_emergency_close_state(env: &Env, state: &EmergencyCloseState) {
    env.storage()
        .instance()
        .set(&DataKey::EmergencyCloseState, state);
    bump_instance_ttl(env);
}

/// Returns configured collateral metadata, if any.
pub fn get_collateral(env: &Env) -> Option<Collateral> {
    let collateral = env.storage().instance().get(&DataKey::Collateral);
    bump_instance_ttl(env);
    collateral
}

/// Persists collateral metadata.
pub fn update_collateral(env: &Env, collateral: &Collateral) {
    env.storage().instance().set(&DataKey::Collateral, collateral);
    bump_instance_ttl(env);
}

/// Increments tracked payment round index.
pub fn incr_next_payment_round(env: &Env) {
    let next_round: u32 = env.storage().instance().get(&DataKey::NextPaymentRound).unwrap_or(0);
    env.storage()
        .instance()
        .set(&DataKey::NextPaymentRound, &(next_round + 1));
    bump_instance_ttl(env);
}

/// Returns current payment round index.
pub fn get_next_payment_round(env: &Env) -> u32 {
    let next_round: u32 = env.storage().instance().get(&DataKey::NextPaymentRound).unwrap_or(0);
    bump_instance_ttl(env);
    next_round
}

/// Extends instance storage TTL for contract state keys.
fn bump_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}