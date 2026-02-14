use crate::{
    balance::ContractBalance,
    claim::{calculate_next_claim, Claim},
    data::{ContractData, DataKey},
    investment::{Investment, InvestmentStatus},
};
use soroban_sdk::{Env, Map};

const DAY_IN_LEDGERS: u32 = 17280;

// Instance storage: accessed frequently, moderate TTL
const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // ~30 days
const INSTANCE_LIFETIME_THRESHOLD: u32 = 15 * DAY_IN_LEDGERS; // ~15 days

// Persistent storage: critical user data, long TTL for safety
const PERSISTENT_BUMP_AMOUNT: u32 = 180 * DAY_IN_LEDGERS; // ~6 months
const PERSISTENT_LIFETIME_THRESHOLD: u32 = 90 * DAY_IN_LEDGERS; // ~3 months

pub fn get_contract_data(e: &Env) -> ContractData {
    let contract_data = e
        .storage()
        .instance()
        .get(&DataKey::ContractData)
        .unwrap_or_else(|| panic!("Contract data has expired"));

    bump_instance_ttl(e);
    contract_data
}

pub fn update_contract_data(e: &Env, contract_data: &ContractData) {
    e.storage()
        .instance()
        .set(&DataKey::ContractData, contract_data);
}

pub fn get_investment(e: &Env, token_id: u32) -> Option<Investment> {
    let key = DataKey::Investment(token_id);
    let investment: Option<Investment> = e.storage().persistent().get(&key);

    if let Some(ref inv) = investment {
        if inv.status != InvestmentStatus::Finished {
            bump_persistent_ttl(e, &key);
        }
    }

    investment
}

pub fn update_investment_with_claim(e: &Env, token_id: u32, investment: &Investment) {
    set_investment(e, token_id, investment);
    let mut claims_map = get_claims_map_or_new(e);
    claims_map.set(token_id, calculate_next_claim(e, investment));
    e.storage().instance().set(&DataKey::ClaimsMap, &claims_map);
}

pub fn get_claims_map_or_new(e: &Env) -> Map<u32, Claim> {
    let key = DataKey::ClaimsMap;
    e.storage()
        .instance()
        .get(&key)
        .unwrap_or(Map::<u32, Claim>::new(e))
}

pub fn update_contract_balances(e: &Env, contract_balances: &ContractBalance) {
    e.storage()
        .instance()
        .set(&DataKey::ContractBalances, contract_balances);
    bump_instance_ttl(e);
}

pub fn get_balances_or_new(e: &Env) -> ContractBalance {
    let key = DataKey::ContractBalances;
    e.storage().instance().get(&key).unwrap_or_default()
}

fn bump_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

fn bump_persistent_ttl(e: &Env, key: &DataKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

fn set_investment(e: &Env, token_id: u32, investment: &Investment) {
    let key = DataKey::Investment(token_id);
    e.storage().persistent().set(&key, &investment);
    if investment.status != InvestmentStatus::Finished {
        bump_persistent_ttl(e, &key);
    }
}
