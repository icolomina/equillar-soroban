use crate::{
    balance::ContractBalance, collateral::Collateral, data::{ContractData, DataKey}, investment::Investment
};
use soroban_sdk::Env;

const DAY_IN_LEDGERS: u32 = 17280;

// Instance storage: accessed frequently, moderate TTL
const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // ~30 days
const INSTANCE_LIFETIME_THRESHOLD: u32 = 15 * DAY_IN_LEDGERS; // ~15 days

// Persistent storage: critical user data, long TTL for safety
const PERSISTENT_BUMP_AMOUNT: u32 = 180 * DAY_IN_LEDGERS; // ~6 months
const PERSISTENT_LIFETIME_THRESHOLD: u32 = 90 * DAY_IN_LEDGERS; // ~3 months

/// Loads `ContractData` from instance storage and refreshes instance TTL.
///
/// # Panics
///
/// Panics if `ContractData` is missing/expired.
pub fn get_contract_data(e: &Env) -> ContractData {
    let contract_data = e
        .storage()
        .instance()
        .get(&DataKey::ContractData)
        .unwrap_or_else(|| panic!("Contract data has expired"));

    bump_instance_ttl(e);
    contract_data
}

/// Stores `ContractData` in instance storage.
pub fn update_contract_data(e: &Env, contract_data: &ContractData) {
    e.storage()
        .instance()
        .set(&DataKey::ContractData, contract_data);
}

/// Loads an investment by `token_id` from persistent storage.
///
/// For active investments (`completed == false`), refreshes persistent TTL.
/// Completed investments keep their current TTL to avoid unnecessary lifetime extension.
pub fn get_investment(e: &Env, token_id: u32) -> Option<Investment> {
    let key = DataKey::Investment(token_id);
    let investment: Option<Investment> = e.storage().persistent().get(&key);

    if let Some(ref inv) = investment {
        if !inv.completed {
            bump_persistent_ttl(e, &key);
        }
    }

    investment
}  

/// Stores the contract balance snapshot in instance storage and refreshes TTL.
pub fn update_contract_balances(e: &Env, contract_balances: &ContractBalance) {
    e.storage()
        .instance()
        .set(&DataKey::ContractBalances, contract_balances);
    bump_instance_ttl(e);
}

/// Returns the current contract balances or a zeroed default value.
pub fn get_balances_or_new(e: &Env) -> ContractBalance {
    let key = DataKey::ContractBalances;
    e.storage().instance().get(&key).unwrap_or_default()
}


/// Stores collateral configuration in instance storage and refreshes TTL.
pub fn update_collateral(e: &Env, collateral: &Collateral) {
    e.storage()
        .instance()
        .set(&DataKey::Collateral, collateral);
    bump_instance_ttl(e);
}

/// Loads collateral configuration from instance storage.
pub fn get_collateral(e: &Env) -> Option<Collateral> {
    e.storage().instance().get(&DataKey::Collateral)
}

/// Stores an investment in persistent storage.
///
/// Active investments refresh persistent TTL; completed investments do not.
pub fn set_investment(e: &Env, token_id: u32, investment: &Investment) {
    let key = DataKey::Investment(token_id);
    e.storage().persistent().set(&key, &investment);
    if ! investment.completed {
        bump_persistent_ttl(e, &key);
    }
}

/// Increments the `NextPaymentRound` counter in instance storage and refreshes TTL.
pub fn incr_next_payment_round(e: &Env) {
    let key = DataKey::NextPaymentRound;
    let next_round: u32 = e.storage().instance().get(&key).unwrap_or(0);
    e.storage().instance().set(&key, &(next_round + 1));
    bump_instance_ttl(e);
}

/// Returns the current `NextPaymentRound` counter.
///
/// If absent, returns `0` and refreshes instance TTL.
pub fn get_next_payment_round(e: &Env) -> u32 {
    let key = DataKey::NextPaymentRound;
    let next_round: u32 = e.storage().instance().get(&key).unwrap_or(0);
    bump_instance_ttl(e);
    next_round
}

/// Extends instance-storage TTL for frequently accessed shared contract state.
fn bump_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

    /// Extends persistent-storage TTL for the given key.
    ///
    /// Used for user-critical records (for example, active investments).
fn bump_persistent_ttl(e: &Env, key: &DataKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}