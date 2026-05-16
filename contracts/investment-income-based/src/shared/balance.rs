use soroban_sdk::contracttype;

use crate::investment::{Investment, InvestmentAllocation};

/// Aggregated accounting snapshot for contract-level financial state.
///
/// Fields track reserves, project funds, commissions, obligations, and
/// collateral/emergency/refund side effects over time.
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
    pub refunded_to_investor: i128,
}

impl Default for ContractBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractBalance {
    /// Creates a zero-initialized balance snapshot.
    pub fn new() -> Self {
        Self {
            reserve: 0,
            project: 0,
            comission: 0,
            comission_withdrawal: 0,
            received_so_far: 0,
            payments: 0,
            reserve_contributions: 0,
            project_withdrawals: 0,
            moved_from_project_to_reserve: 0,
            payment_obligations: 0,
            collateral_received: 0,
            collateral_liquidated: 0,
            collateral_returned: 0,
            refunded_to_investor: 0,
        }
    }

    /// Applies accounting effects of a newly accepted investment.
    pub fn recalculate_from_investment(
        &mut self,
        allocation: &InvestmentAllocation,
        investment: &Investment,
    ) {
        self.comission += allocation.amount_to_commission;
        self.reserve += allocation.amount_to_reserve_fund;
        self.project += allocation.amount_to_invest;
        self.received_so_far += allocation.amount_to_reserve_fund + allocation.amount_to_invest;
        self.payment_obligations += investment.total;
    }

    /// Applies accounting effects of a regular investor payment.
    pub fn recalculate_from_payment_to_investor(&mut self, amount: &i128) {
        self.reserve -= amount;
        self.payments += amount;
        self.payment_obligations -= amount;
    }

    /// Applies accounting effects of a company transfer into reserve.
    pub fn recalculate_from_company_contribution(&mut self, amount: &i128) {
        self.reserve += amount;
        self.reserve_contributions += amount;
    }

    /// Applies accounting effects of withdrawing project funds.
    pub fn recalculate_from_company_withdrawal(&mut self, amount: &i128) {
        self.project -= amount;
        self.project_withdrawals += amount;
    }

    /// Applies accounting effects of commission withdrawal.
    pub fn recalculate_from_comission_withdrawal(&mut self, amount: &i128) {
        self.comission_withdrawal += amount;
    }

    /// Applies accounting effects of collateral deposit reception.
    pub fn recalculate_from_collateral_received(&mut self, amount: &i128) {
        self.collateral_received += amount;
    }

    /// Applies accounting effects when collateral is liquidated for a position.
    pub fn recalculate_from_collateral_liquidated(
        &mut self,
        collateral_amount: &i128,
        remaining_obligations: &i128,
    ) {
        self.collateral_liquidated += collateral_amount;
        self.payment_obligations -= remaining_obligations;
    }

    /// Applies accounting effects when collateral is returned to provider.
    pub fn recalculate_from_collateral_returned(&mut self, amount: &i128) {
        self.collateral_returned += amount;
    }

    /// Applies accounting effects of one emergency payout.
    ///
    /// Payouts consume reserve first, then project balance for the remainder.
    pub fn recalculate_from_emergency_payment(
        &mut self,
        amount: &i128,
        remaining_obligations: &i128,
    ) {
        let reserve_to_use = if self.reserve >= *amount {
            *amount
        } else {
            self.reserve
        };
        let project_to_use = *amount - reserve_to_use;

        self.reserve -= reserve_to_use;
        self.project -= project_to_use;
        self.payments += amount;
        self.payment_obligations -= remaining_obligations;
    }

    /// Applies accounting effects of investor refund.
    pub fn recalculate_from_refunded_to_investor(&mut self, investment: &Investment) {
        self.project -= investment.amount_invested;
        self.reserve -= investment.amount_to_reserve;
        self.comission -= investment.commission;
        self.refunded_to_investor += investment.deposited + investment.commission;
        self.payment_obligations -= investment.total - investment.paid;
    }
}