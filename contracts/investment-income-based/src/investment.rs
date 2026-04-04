use crate::{
    amounts::Amount,
    data::ContractData
};
use soroban_sdk::{contracttype};

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
    /// Builds a new investment snapshot from contract configuration and amount split.
    ///
    /// Derives:
    /// * `deposited` as investable amount plus reserve contribution.
    /// * `accumulated_interests` from `interest_rate`.
    /// * `total` as principal plus interests.
    /// * `regular_payment` according to the configured return type.
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
            amount_invested: amounts.amount_to_invest,
            amount_to_reserve: amounts.amount_to_reserve_fund,
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

    /// Applies one payment round to this investment and returns the transfer amount.
    ///
    /// Increments `payments_transferred`, updates `paid`, and marks investment as
    /// completed when the last round is reached. For `Coupon`, the last round also
    /// returns full principal (`deposited`).
    ///
    /// # Edge cases
    ///
    /// Integer division in `regular_payment` can leave minimal rounding dust between
    /// `paid` and theoretical totals depending on configuration.
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

    /// Returns the refundable amount (`deposited + commission`).
    ///
    /// Used by the fundraising-time refund path.
    pub fn get_amount_to_refund(self) -> i128 {
        let amount_to_refund = self.deposited + self.commission;
        amount_to_refund
    }

    /// Computes per-round scheduled payment for the selected return type.
    ///
    /// * `Coupon`: interest-only monthly amount.
    /// * `ReverseLoan`: principal + interest spread over all rounds.
    ///
    /// Uses integer division and therefore truncates fractional remainders.
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
    /// Converts numeric contract input into a typed `InvestmentReturnType`.
    ///
    /// Accepted values:
    /// * `1` => `ReverseLoan`
    /// * `2` => `Coupon`
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
