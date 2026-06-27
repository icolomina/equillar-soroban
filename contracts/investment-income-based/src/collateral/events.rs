use soroban_sdk::{contractevent, Address, Env, String};

/// Emitted when collateral is deposited into the contract.
#[contractevent]
pub(super) struct CollateralDeposited<'a> {
    #[topic]
    pub addr: Address,
    pub current_collateral_amount: i128,
    pub total_deposited: i128,
    pub token_address: &'a Address,
    pub token_symbol: &'a String,
}

/// Emitted when collateral is sent out to cover obligations.
#[contractevent]
pub(super) struct CollateralSent {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_sent: i128,
}

/// Emitted when remaining collateral is returned to the provider.
#[contractevent]
pub(super) struct CollateralReturned {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_returned: i128,
}

/// Publishes collateral deposit event.
pub(super) fn emit_collateral_deposited(
    env: &Env,
    current_collateral_amount: i128,
    total_deposited: i128,
    token_collateral_address: &Address,
    token_collateral_symbol: &String
) {
    CollateralDeposited {
        addr: env.current_contract_address(),
        current_collateral_amount,
        total_deposited,
        token_address: &token_collateral_address,
        token_symbol: &token_collateral_symbol,
    }
    .publish(env);
}

/// Publishes collateral payment event.
pub(super) fn emit_collateral_sent(env: &Env, to: Address, total_sent: i128) {
    CollateralSent {
        addr: env.current_contract_address(),
        to,
        total_sent,
    }
    .publish(env);
}

/// Publishes collateral return event.
pub(super) fn emit_collateral_returned(env: &Env, to: Address, total_returned: i128) {
    CollateralReturned {
        addr: env.current_contract_address(),
        to,
        total_returned,
    }
    .publish(env);
}