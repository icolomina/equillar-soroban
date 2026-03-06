use soroban_sdk::{contractevent, contracttype, Env};

use crate::{amounts::Amount, investment::Investment};


#[contracttype]
pub struct ContractBalance {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub received_so_far: i128,
    pub payments: i128,
    pub reserve_contributions: i128,
    pub project_withdrawals: i128,
    pub moved_from_project_to_reserve: i128,
    pub payment_obligations: i128,
    pub collateral_received: i128,
    pub collateral_liquidated: i128
}

#[contractevent(topics = ["CBUPDATED"])]
pub struct ContractBalanceUpdated {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub received_so_far: i128,
    pub payments: i128,
    pub reserve_contributions: i128,
    pub project_withdrawals: i128,
    pub moved_from_project_to_reserve: i128,
    pub payment_obligations: i128,
    pub collateral_received: i128,
    pub collateral_liquidated: i128
}

impl Default for ContractBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractBalance {
    pub fn new() -> Self {
        ContractBalance {
            reserve: 0_i128,
            project: 0_i128,
            comission: 0_i128,
            received_so_far: 0_i128,
            payments: 0_i128,
            reserve_contributions: 0_i128,
            project_withdrawals: 0_i128,
            moved_from_project_to_reserve: 0_i128,
            payment_obligations: 0_i128,
            collateral_received: 0_i128,
            collateral_liquidated: 0_i128
        }
    }

    pub fn sum(&self) -> i128 {
        self.comission + self.project + self.reserve
    }

    pub fn recalculate_from_investment(&mut self, amounts: &Amount, investment: &Investment) {
        self.comission += amounts.amount_to_commission;
        self.reserve += amounts.amount_to_reserve_fund;
        self.project += amounts.amount_to_invest;
        self.received_so_far += amounts.amount_to_reserve_fund + amounts.amount_to_invest;
        self.payment_obligations += investment.total
    }

    pub fn recalculate_from_company_contribution(&mut self, amount: &i128) {
        self.reserve += amount;
        self.reserve_contributions += amount;
    }

    pub fn recalculate_from_company_withdrawal(&mut self, amount: &i128) {
        self.project -= amount;
        self.project_withdrawals += amount;
    }

    pub fn recalculate_from_payment_to_investor(&mut self, amount: &i128) {
        self.reserve -= amount;
        self.payments += amount;
        self.payment_obligations -= amount;
    }

    pub fn recalculate_from_project_to_reserver_movement(&mut self, amount: &i128) {
        self.project -= amount;
        self.reserve += amount;
        self.moved_from_project_to_reserve += amount;
    }

    pub fn recalculate_from_collateral_received(&mut self, amount: &i128) {
        self.collateral_received += amount;
    }

    pub fn recalculate_from_collateral_liquidated(&mut self, amount: &i128) {
        self.collateral_liquidated += amount;
        self.collateral_received -= amount;
    }

    /// Emits a ContractBalancesUpdated event
    pub fn emit_event(&self, env: &Env) {
        ContractBalanceUpdated {
            reserve: self.reserve,
            project: self.project,
            comission: self.comission,
            received_so_far: self.received_so_far,
            payments: self.payments,
            reserve_contributions: self.reserve_contributions,
            project_withdrawals: self.project_withdrawals,
            moved_from_project_to_reserve: self.moved_from_project_to_reserve,
            payment_obligations: self.payment_obligations,
            collateral_received: self.collateral_received,
            collateral_liquidated: self.collateral_liquidated
        }
        .publish(env);
    }
}

