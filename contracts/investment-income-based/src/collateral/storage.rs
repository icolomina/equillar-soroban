use soroban_sdk::Env;

use crate::{
    collateral::Collateral,
    shared::{storage_helper, types::DataKey},
};

/// Returns configured collateral metadata, if any.
pub(super) fn get_collateral(env: &Env) -> Option<Collateral> {
    let collateral = env.storage().instance().get(&DataKey::Collateral);
    storage_helper::bump_instance_ttl(env);
    collateral
}

/// Persists collateral metadata.
pub(super) fn update_collateral(env: &Env, collateral: &Collateral) {
    env.storage()
        .instance()
        .set(&DataKey::Collateral, collateral);
    storage_helper::bump_instance_ttl(env);
}
