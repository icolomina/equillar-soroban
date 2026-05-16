use soroban_sdk::{contractevent, Address, Env, String};

use crate::shared::ContractBalance;

/// Emitted once on successful contract deployment/configuration.
#[contractevent]
pub struct ContractDeployed {
    addr: Address,
    symbol: String,
    ts_fundraising_ends: u64,
    ts_payments_start: u64,
}

/// Emitted whenever aggregated contract balance snapshot is updated.
#[contractevent]
pub struct ContractBalanceUpdated {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub comission_withdrawal: i128,
    pub received_so_far: i128,
    pub payments: i128,
    pub reserve_contributions: i128,
    pub project_withdrawals: i128,
    pub moved_from_project_to_reserve: i128,
    pub payment_obligations: i128,
    pub collateral_received: i128,
    pub collateral_liquidated: i128,
    pub collateral_returned: i128,
    pub refunded_to_investor: i128,
}

/// Publishes contract deployment event.
pub fn emit_contract_deployed_event(
    env: &Env,
    addr: Address,
    symbol: String,
    ts_fundraising_ends: u64,
    ts_payments_start: u64,
) {
    ContractDeployed {
        addr,
        symbol,
        ts_fundraising_ends,
        ts_payments_start,
    }
    .publish(env);
}

/// Publishes balance snapshot update event.
pub fn emit_balance_updated_event(env: &Env, contract_balance: &ContractBalance) {
    ContractBalanceUpdated {
        reserve: contract_balance.reserve,
        project: contract_balance.project,
        comission: contract_balance.comission,
        comission_withdrawal: contract_balance.comission_withdrawal,
        received_so_far: contract_balance.received_so_far,
        payments: contract_balance.payments,
        reserve_contributions: contract_balance.reserve_contributions,
        project_withdrawals: contract_balance.project_withdrawals,
        moved_from_project_to_reserve: contract_balance.moved_from_project_to_reserve,
        payment_obligations: contract_balance.payment_obligations,
        collateral_received: contract_balance.collateral_received,
        collateral_liquidated: contract_balance.collateral_liquidated,
        collateral_returned: contract_balance.collateral_returned,
        refunded_to_investor: contract_balance.refunded_to_investor,
    }
    .publish(env);
}