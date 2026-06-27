use soroban_sdk::{contractevent, Address, Env};

/// Emitted when one investor receives emergency-close payout.
#[contractevent]
pub(super) struct EmergencyPaymentSent {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_sent: i128,
}

/// Emitted when emergency-close mode becomes active.
#[contractevent]
pub(super) struct EmergencyCloseActivated {
    #[topic]
    pub addr: Address,
    pub emergency_pool_total: i128,
    pub emergency_obligations_total: i128,
}

/// Publishes emergency investor payment event.
pub(super) fn emit_emergency_payment_sent(env: &Env, to: Address, total_sent: i128) {
    EmergencyPaymentSent {
        addr: env.current_contract_address(),
        to,
        total_sent,
    }
    .publish(env);
}

/// Publishes emergency-close activation event.
pub(super) fn emit_emergency_close_activated(
    env: &Env,
    emergency_pool_total: i128,
    emergency_obligations_total: i128,
) {
    EmergencyCloseActivated {
        addr: env.current_contract_address(),
        emergency_pool_total,
        emergency_obligations_total,
    }
    .publish(env);
}