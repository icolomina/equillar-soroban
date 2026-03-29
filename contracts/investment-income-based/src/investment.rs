use crate::{
    amounts::Amount,
    data::ContractData
};
use soroban_sdk::{contracttype};

#[contracttype]
#[derive(Copy, Clone)]
pub struct Investment {
    pub deposited: i128,
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
    pub fn new(cd: &ContractData, amounts: &Amount, token_id: u32) -> Self {
        let real_amount = amounts.amount_to_invest + amounts.amount_to_reserve_fund;
        let current_interest = (real_amount * cd.interest_rate as i128) / 100 / 100;
        let total_gains = real_amount + current_interest;
        let regular_payment = Self::calculate_regular_payment(
            &current_interest,
            &total_gains,
            &cd.return_months,
            &cd.return_type,
        );

        Investment {
            deposited: real_amount,
            commission: amounts.amount_to_commission,
            accumulated_interests: current_interest,
            total: total_gains,
            completed: false,
            regular_payment,
            paid: 0_i128,
            payments_transferred: 0_u32,
            token_id,
        }
    }

    pub fn process_investment_payment(&mut self, contract_data: &ContractData) -> i128 {
        let mut amount_to_transfer: i128;

        self.paid += &self.regular_payment;
        self.payments_transferred += 1;
        amount_to_transfer = self.regular_payment;

        let is_last_payment = self.payments_transferred >= contract_data.return_months;

        if is_last_payment {
            self.completed = true;

            if contract_data.return_type == InvestmentReturnType::Coupon {
                self.paid += self.deposited;
                amount_to_transfer += self.deposited;
            }
        }

        amount_to_transfer
    }

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
pub enum InvestmentReturnType {
    ReverseLoan = 1,
    Coupon = 2,
}

impl InvestmentReturnType {
    pub fn from_number<N>(value: N) -> Option<InvestmentReturnType>
    where
        N: Into<u32>,
    {
        let value: u32 = value.into();
        match value {
            1 => Some(InvestmentReturnType::ReverseLoan),
            2 => Some(InvestmentReturnType::Coupon),
            _ => None,
        }
    }
}
