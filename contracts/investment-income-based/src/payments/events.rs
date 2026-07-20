use soroban_sdk::{contractevent, Address, Env};

/// Emitted after an investor payment transfer is executed.
#[contractevent]
pub struct PaymentSent {
    #[topic]
    pub addr: Address,
    pub total_sent: i128,
}

#[contractevent]
pub struct CompanyTransferReceived {
    #[topic]
    pub addr: Address,
    pub total_received: i128,
}

/// Publishes investor payment event.
pub fn emit_payment_sent(env: &Env, total_sent: i128) {
    PaymentSent {
        addr: env.current_contract_address(),
        total_sent,
    }
    .publish(env);
}

/// Publishes company transfer event.
pub fn emit_company_transfer_received(env: &Env, total_received: i128) {
    CompanyTransferReceived {
        addr: env.current_contract_address(),
        total_received,
    }
    .publish(env);
}
