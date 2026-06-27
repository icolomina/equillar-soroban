use soroban_sdk::{Address, Env};

use crate::emergency::EmergencyCloseState;
use crate::shared::storage_helper;
use crate::shared::types::Position;
use crate::shared::types::{ContractBalance, ContractData, DataKey};

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

    storage_helper::bump_instance_ttl(env);
    contract_data
}

/// Persists contract configuration in instance storage.
pub fn update_contract_data(env: &Env, contract_data: &ContractData) {
    env.storage()
        .instance()
        .set(&DataKey::ContractData, contract_data);
    storage_helper::bump_instance_ttl(env);
}

/// Returns current balances snapshot, defaulting to a zeroed balance object.
pub fn get_balances_or_new(env: &Env) -> ContractBalance {
    let balances = env
        .storage()
        .instance()
        .get(&DataKey::ContractBalances)
        .unwrap_or_default();

    storage_helper::bump_instance_ttl(env);
    balances
}

/// Persists contract balances snapshot.
pub fn update_contract_balances(env: &Env, balances: &ContractBalance) {
    env.storage()
        .instance()
        .set(&DataKey::ContractBalances, balances);
    storage_helper::bump_instance_ttl(env);
}

/// Returns emergency-close state if present.
pub fn get_emergency_close_state(env: &Env) -> Option<EmergencyCloseState> {
    let state = env.storage().instance().get(&DataKey::EmergencyCloseState);
    storage_helper::bump_instance_ttl(env);
    state
}

/// Persists emergency-close state.
pub fn set_emergency_close_state(env: &Env, state: &EmergencyCloseState) {
    env.storage()
        .instance()
        .set(&DataKey::EmergencyCloseState, state);
    storage_helper::bump_instance_ttl(env);
}

pub fn set_position(env: &Env, position_id: u32, position: &Position) {
    let key = DataKey::Position(position_id);
    env.storage().persistent().set(&key, position);

    if !position.completed {
        storage_helper::bump_persistent_ttl(env, &key);
    }
}

pub fn get_position(env: &Env, position_id: u32) -> Option<Position> {
    let key = DataKey::Position(position_id);
    let position: Option<Position> = env.storage().persistent().get(&key);

    if let Some(ref current) = position {
        if !current.completed {
            storage_helper::bump_persistent_ttl(env, &key);
        }
    }

    position
}

pub fn set_addr_position_id(env: &Env, token_id: u32, addr: Address) {
    let key = DataKey::PositionIdAddress(token_id);
    env.storage().persistent().set(&key, &addr);
    storage_helper::bump_persistent_ttl(env, &key);
}

pub fn get_addr_position_id(env: &Env, token_id: u32) -> Option<Address> {
    let key = DataKey::PositionIdAddress(token_id);
    let option = env.storage().persistent().get(&key);
    if let Some(addr) = option {
        storage_helper::bump_persistent_ttl(env, &key);
        return Some(addr);
    }

    None
}
