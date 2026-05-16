use soroban_sdk::contracttype;

use crate::investment::InvestmentAllocation;
use crate::shared::ContractData;

/// NFT-backed investment position state.
///
/// Tracks principal split, commission, accrued interests, payment progress,
/// completion status, and NFT token id.
#[contracttype]
#[derive(Copy, Clone)]
pub struct Investment {
    pub deposited: i128,
    pub amount_invested: i128,
    pub amount_to_reserve: i128,
    pub commission: i128,
    pub accumulated_interests: i128,
    pub total: i128,
    pub completed: bool,
    pub regular_payment: i128,
    pub paid: i128,
    pub payments_transferred: u32,
    pub token_id: u32,
}

impl Investment {
    /// Creates a new investment state from allocation and contract parameters.
    pub fn new(contract_data: &ContractData, allocation: &InvestmentAllocation, token_id: u32) -> Self {
        let deposited = allocation.amount_to_invest + allocation.amount_to_reserve_fund;
        let accumulated_interests = (deposited * contract_data.interest_rate as i128) / 100 / 100;
        let total = deposited + accumulated_interests;
        let regular_payment = Self::calculate_regular_payment(
            &accumulated_interests,
            &total,
            &contract_data.return_months,
            &contract_data.return_type,
        );

        Self {
            deposited,
            amount_invested: allocation.amount_to_invest,
            amount_to_reserve: allocation.amount_to_reserve_fund,
            commission: allocation.amount_to_commission,
            accumulated_interests,
            total,
            completed: false,
            regular_payment,
            paid: 0,
            payments_transferred: 0,
            token_id,
        }
    }

    /// Applies one payment round to this investment and returns transfer amount.
    ///
    /// For coupon mode, principal is transferred in the final round.
    pub fn process_investment_payment(&mut self, contract_data: &ContractData) -> i128 {
        let mut amount_to_transfer = self.regular_payment;

        self.paid += self.regular_payment;
        self.payments_transferred += 1;

        if self.payments_transferred >= contract_data.return_months {
            self.completed = true;

            if contract_data.return_type == InvestmentReturnType::Coupon {
                self.paid += self.deposited;
                amount_to_transfer += self.deposited;
            }
        }

        amount_to_transfer
    }

    /// Returns full refund amount for fundraising-window refunds.
    pub fn get_amount_to_refund(self) -> i128 {
        self.deposited + self.commission
    }

    /// Computes per-round payment according to selected return type.
    fn calculate_regular_payment(
        interest_gains: &i128,
        total_gains: &i128,
        return_months: &u32,
        return_type: &InvestmentReturnType,
    ) -> i128 {
        match return_type {
            InvestmentReturnType::Coupon => interest_gains / *return_months as i128,
            InvestmentReturnType::ReverseLoan => total_gains / *return_months as i128,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
#[contracttype]
/// Repayment profile used by investment positions.
pub enum InvestmentReturnType {
    ReverseLoan = 1,
    Coupon = 2,
}

impl InvestmentReturnType {
    /// Parses numeric return type code into enum variant.
    pub fn from_number<N>(value: N) -> Option<Self>
    where
        N: Into<u32>,
    {
        match value.into() {
            1 => Some(Self::ReverseLoan),
            2 => Some(Self::Coupon),
            _ => None,
        }
    }
}