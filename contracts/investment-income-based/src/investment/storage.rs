use soroban_sdk::Env;

use crate::investment::Investment;
use crate::shared::DataKey;

const DAY_IN_LEDGERS: u32 = 17_280;
const PERSISTENT_BUMP_AMOUNT: u32 = 180 * DAY_IN_LEDGERS;
const PERSISTENT_LIFETIME_THRESHOLD: u32 = 90 * DAY_IN_LEDGERS;

/// Loads an investment position from persistent storage.
///
/// Active (non-completed) positions extend their TTL on read.
pub fn get_investment(env: &Env, token_id: u32) -> Option<Investment> {
    let key = DataKey::Investment(token_id);
    let investment: Option<Investment> = env.storage().persistent().get(&key);

    if let Some(ref current) = investment {
        if !current.completed {
            bump_persistent_ttl(env, &key);
        }
    }

    investment
}

/// Persists an investment position.
///
/// Active (non-completed) positions extend their TTL on write.
pub fn set_investment(env: &Env, token_id: u32, investment: &Investment) {
    let key = DataKey::Investment(token_id);
    env.storage().persistent().set(&key, investment);

    if !investment.completed {
        bump_persistent_ttl(env, &key);
    }
}

/// Extends persistent TTL for one investment key.
fn bump_persistent_ttl(env: &Env, key: &DataKey) {
    env.storage().persistent().extend_ttl(
        key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}