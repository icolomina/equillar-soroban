use soroban_sdk::{contractevent, Address, Env};

/// Emitted when an investment is accepted.
#[contractevent]
pub struct InvestmentReceived {
    #[topic]
    pub addr: Address,
    pub deposited: i128,
    pub capital_gains: i128,
}

/// Emitted when fundraising goal is reached.
#[contractevent]
pub struct GoalReached {
    #[topic]
    pub addr: Address,
    pub total_received: i128,
    pub goal: i128,
}

/// Emitted when a fundraising-window refund is processed.
#[contractevent]
pub struct InvestmentDepositRefunded {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub amount: i128,
}

/// Publishes investment received event.
pub fn emit_investment_received_event(env: &Env, deposited: i128, capital_gains: i128) {
    InvestmentReceived {
        addr: env.current_contract_address(),
        deposited,
        capital_gains,
    }
    .publish(env);
}

/// Publishes goal reached event.
pub fn emit_goal_reached_event(env: &Env, total_received: i128, goal: i128) {
    GoalReached {
        addr: env.current_contract_address(),
        total_received,
        goal,
    }
    .publish(env);
}

/// Publishes investment refund event.
pub fn emit_investment_deposit_refunded(env: &Env, to: Address, total_refunded: i128) {
    InvestmentDepositRefunded {
        addr: env.current_contract_address(),
        to,
        amount: total_refunded,
    }
    .publish(env);
}