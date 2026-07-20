use soroban_sdk::{contractevent, Address, Env};

/// Emitted when project funds are withdrawn.
#[contractevent]
pub struct WithdrawalDone {
    #[topic]
    pub addr: Address,
    pub total_withdrawn: i128,
}

/// Emitted when commissions are withdrawn.
#[contractevent]
pub struct CommissionWithdrawn {
    #[topic]
    pub addr: Address,
    pub amount: i128,
}

/// Emitted when commissions are withdrawn.
#[contractevent]
pub struct AllWithdrawn {
    #[topic]
    pub addr: Address,
    pub amount: i128,
}

/// Publishes project withdrawal event.
pub fn emit_withdrawal_done(env: &Env, total_withdrawn: i128) {
    WithdrawalDone {
        addr: env.current_contract_address(),
        total_withdrawn,
    }
    .publish(env);
}

/// Publishes commission withdrawal event.
pub fn emit_commission_withdrawn(env: &Env, total_withdrawn: i128) {
    CommissionWithdrawn {
        addr: env.current_contract_address(),
        amount: total_withdrawn,
    }
    .publish(env);
}

pub fn emit_all_withdrawn(env: &Env, total_withdrawn: i128) {
    AllWithdrawn {
        addr: env.current_contract_address(),
        amount: total_withdrawn,
    }
    .publish(env);
}
