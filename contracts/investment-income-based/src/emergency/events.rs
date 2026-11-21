use soroban_sdk::{contractevent, Address, Env};

/// Emitted when emergency-close mode becomes active.
#[contractevent]
pub(super) struct EmergencyCloseActivated {
    #[topic]
    pub addr: Address,
}

/// Publishes emergency-close activation event.
pub(super) fn emit_emergency_close_activated(env: &Env) {
    EmergencyCloseActivated {
        addr: env.current_contract_address(),
    }
    .publish(env);
}
