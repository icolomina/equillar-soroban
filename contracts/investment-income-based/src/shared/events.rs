use soroban_sdk::{contractevent, Address, Env};

use crate::shared::types::{Balance, BalanceAsset, BalanceEquity, BalanceLiabilities};

/// Emitted once on successful contract deployment/configuration.
#[contractevent]
pub struct ContractDeployed {
    addr: Address,
    ts_fundraising_ends: u64,
    ts_payments_start: u64,
}

#[contractevent]
pub struct BalanceUpdated {
    pub assets: BalanceAsset,
    pub liabilities: BalanceLiabilities,
    pub equity: BalanceEquity,
}

/// Publishes contract deployment event.
pub fn emit_contract_deployed_event(
    env: &Env,
    addr: Address,
    ts_fundraising_ends: u64,
    ts_payments_start: u64,
) {
    ContractDeployed {
        addr,
        ts_fundraising_ends,
        ts_payments_start,
    }
    .publish(env);
}

pub fn emit_balance_updated_event(env: &Env, balance: &Balance) {
    BalanceUpdated {
        assets: BalanceAsset {
            payment_rigths: balance.assets.payment_rigths,
            contract_earnings: balance.assets.contract_earnings,
            managed_volume: balance.assets.managed_volume,
            managed_collateral: balance.assets.managed_collateral,
        },
        liabilities: BalanceLiabilities {
            payment_obligations: balance.liabilities.payment_obligations,
            return_collateral_obligations: balance.liabilities.return_collateral_obligations,
        },
        equity: BalanceEquity {
            contract_commissions: balance.equity.contract_commissions,
        },
    }
    .publish(env);
}
