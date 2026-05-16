use soroban_sdk::contracttype;

use crate::investment::Investment;
use crate::shared::ContractBalance;

/// Snapshot of pool/obligations used during emergency-close settlement.
#[contracttype]
pub struct EmergencyCloseState {
    pub emergency_pool_total: i128,
    pub emergency_pool_remaining: i128,
    pub emergency_obligations_left: i128,
}

impl EmergencyCloseState {
    /// Builds emergency-close state from current contract balances.
    pub fn from_contract_balance(contract_balance: &ContractBalance) -> Self {
        let emergency_pool_total = contract_balance.reserve + contract_balance.project;

        Self {
            emergency_pool_total,
            emergency_pool_remaining: emergency_pool_total,
            emergency_obligations_left: contract_balance.payment_obligations,
        }
    }

    /// Calculates proportional emergency payout for one investment.
    pub fn calculate_amount_to_pay(&self, investment: &Investment) -> i128 {
        let remaining_obligations = investment.total - investment.paid;

        if remaining_obligations >= self.emergency_obligations_left {
            self.emergency_pool_remaining
        } else {
            remaining_obligations * self.emergency_pool_remaining / self.emergency_obligations_left
        }
    }

    /// Updates remaining pool and obligations after one emergency payment.
    pub fn update_after_payment(&mut self, amount_paid: &i128, remaining_obligations: &i128) {
        self.emergency_pool_remaining -= amount_paid;
        self.emergency_obligations_left -= remaining_obligations;
    }
}