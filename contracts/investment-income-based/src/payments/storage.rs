use soroban_sdk::{Env};

use crate::shared::types::{DataKey, LiquidateInvestmentsStatus};
use crate::shared::storage_helper;

pub(super) fn enable_liquidate_investments(env: &Env) {
    let key = DataKey::LiquidateInvestmentEnabled;
    env.storage().instance().set(&key, &LiquidateInvestmentsStatus::Enabled);
    storage_helper::bump_instance_ttl(env);

}

pub(super) fn disable_liquidate_investments(env: &Env) {
    let key = DataKey::LiquidateInvestmentEnabled;
    env.storage().instance().set(&key, &LiquidateInvestmentsStatus::Disabled);
    storage_helper::bump_instance_ttl(env);
}

pub(super) fn liquidate_investments_enabled(env: &Env) -> LiquidateInvestmentsStatus {
    let key = DataKey::LiquidateInvestmentEnabled;
    let result: Option<LiquidateInvestmentsStatus> = env.storage().instance().get(&key);

    if let Some(status) = result {
        storage_helper::bump_instance_ttl(env);
        return status;

    }

    LiquidateInvestmentsStatus::Enabled
}

/// Increments tracked payment round index.
pub(super) fn incr_next_payment_round(env: &Env) {
    let next_round: u32 = env.storage().instance().get(&DataKey::NextPaymentRound).unwrap_or(0);
    env.storage()
        .instance()
        .set(&DataKey::NextPaymentRound, &(next_round + 1));
    storage_helper::bump_instance_ttl(env);
}

/// Returns current payment round index.
pub(super) fn get_next_payment_round(env: &Env) -> u32 {
    let next_round: u32 = env.storage().instance().get(&DataKey::NextPaymentRound).unwrap_or(0);
    storage_helper::bump_instance_ttl(env);
    next_round
}

