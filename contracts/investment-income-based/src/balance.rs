use soroban_sdk::contracttype;

use crate::{amounts::Amount, investment::Investment};


#[contracttype]
pub struct ContractBalance {
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
    pub refunded_to_investor: i128
}

impl Default for ContractBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractBalance {
    /// Returns a zero-initialized contract balance snapshot.
    pub fn new() -> Self {
        ContractBalance {
            reserve: 0_i128,
            project: 0_i128,
            comission: 0_i128,
            comission_withdrawal: 0_i128,
            received_so_far: 0_i128,
            payments: 0_i128,
            reserve_contributions: 0_i128,
            project_withdrawals: 0_i128,
            moved_from_project_to_reserve: 0_i128,
            payment_obligations: 0_i128,
            collateral_received: 0_i128,
            collateral_liquidated: 0_i128,
            collateral_returned: 0_i128,
            refunded_to_investor: 0_i128

        }
    }

    /// Returns the liquid contract-token balance tracked by main buckets.
    ///
    /// This sum excludes accounting-only counters (withdrawals, refunds, collateral stats).
    pub fn sum(&self) -> i128 {
        self.comission + self.project + self.reserve
    }

    /// Applies balance changes after accepting a new investment.
    pub fn recalculate_from_investment(&mut self, amounts: &Amount, investment: &Investment) {
        self.comission += amounts.amount_to_commission;
        self.reserve += amounts.amount_to_reserve_fund;
        self.project += amounts.amount_to_invest;
        self.received_so_far += amounts.amount_to_reserve_fund + amounts.amount_to_invest;
        self.payment_obligations += investment.total
    }

    /// Applies balance changes after receiving a company transfer into reserve.
    pub fn recalculate_from_company_contribution(&mut self, amount: &i128) {
        self.reserve += amount;
        self.reserve_contributions += amount;
    }

    /// Applies balance changes after project withdrawal.
    pub fn recalculate_from_company_withdrawal(&mut self, amount: &i128) {
        self.project -= amount;
        self.project_withdrawals += amount;
    }

    /// Applies balance changes after commission withdrawal.
    pub fn recalculate_from_comission_withdrawal(&mut self, amount: &i128) {
        self.comission_withdrawal += amount
    }

    /// Applies balance changes after a regular investor payment.
    pub fn recalculate_from_payment_to_investor(&mut self, amount: &i128) {
        self.reserve -= amount;
        self.payments += amount;
        self.payment_obligations -= amount;
    }

    /// Applies balance changes when moving funds from project bucket to reserve bucket.
    pub fn recalculate_from_project_to_reserver_movement(&mut self, amount: &i128) {
        self.project -= amount;
        self.reserve += amount;
        self.moved_from_project_to_reserve += amount;
    }

    /// Applies balance changes after collateral deposit.
    pub fn recalculate_from_collateral_received(&mut self, amount: &i128) {
        self.collateral_received += amount;
    }

    /// Applies balance changes after collateral liquidation payout.
    ///
    /// Reduces obligations by the investment's remaining obligations, not by
    /// collateral amount, because settlement is obligation-based.
    pub fn recalculate_from_collateral_liquidated(&mut self, collateral_amount: &i128, remaining_obligations: &i128) {
        self.collateral_liquidated += collateral_amount;
        self.payment_obligations -= remaining_obligations;
    }

    /// Applies balance changes after returning remaining collateral to provider.
    pub fn recalculate_from_collateral_returned(&mut self, amount: &i128) {
        self.collateral_returned += amount;
    }

    /// Applies balance changes after emergency payout to an investor.
    ///
    /// Reduces obligations by remaining obligations of the settled investment.
    pub fn recalculate_from_emergency_payment(&mut self, amount: &i128, remaining_obligations: &i128) {
        self.reserve -= amount;
        self.payments += amount;
        self.payment_obligations -= remaining_obligations;
    }

    /// Applies balance changes after refunding an investor during fundraising.
    pub fn recalculate_from_refunded_to_investor(&mut self, investment: &Investment) {
        self.project -= investment.amount_invested;
        self.reserve -= investment.amount_to_reserve;
        self.comission -= investment.commission;
        self.refunded_to_investor += investment.deposited + investment.commission;
        self.payment_obligations -= investment.total - investment.paid;
    }
    
}

