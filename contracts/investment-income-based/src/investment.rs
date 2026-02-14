use crate::{
    balance::{Amount, CalculateAmounts},
    constants::SECONDS_IN_DAY,
    data::{ContractData, FromNumber},
};
use soroban_sdk::{contracttype, Env};

#[contracttype]
#[derive(Copy, Clone)]
pub struct Investment {
    pub deposited: i128,
    pub commission: i128,
    pub accumulated_interests: i128,
    pub total: i128,
    pub claimable_ts: u64,
    pub last_transfer_ts: u64,
    pub status: InvestmentStatus,
    pub regular_payment: i128,
    pub paid: i128,
    pub payments_transferred: u32,
    pub token_id: u32,
}

impl Investment {
    pub fn new(env: &Env, cd: &ContractData, amount: &i128, decimals: u8, token_id: u32) -> Self {
        let amounts: Amount = Amount::from_investment(env, amount, &cd.interest_rate, decimals);
        let real_amount = amounts.amount_to_invest + amounts.amount_to_reserve_fund;
        let current_interest = (real_amount * cd.interest_rate as i128) / 100 / 100;
        let total_gains = real_amount + current_interest;

        let status = Self::calculate_initial_status(&cd.claim_block_days);
        let claimable_ts = Self::calculate_claimable_ts(env, &cd.claim_block_days);
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
            claimable_ts,
            last_transfer_ts: 0_u64,
            status,
            regular_payment,
            paid: 0_i128,
            payments_transferred: 0_u32,
            token_id,
        }
    }

    pub fn process_investment_payment(&mut self, env: &Env, contract_data: &ContractData) -> i128 {
        let mut amount_to_transfer: i128;
        if self.status != InvestmentStatus::CashFlowing {
            self.status = InvestmentStatus::CashFlowing;
        }

        self.paid += &self.regular_payment;
        self.last_transfer_ts = env.ledger().timestamp();
        self.payments_transferred += 1;
        amount_to_transfer = self.regular_payment;

        let is_last_payment = self.payments_transferred >= contract_data.return_months;

        if is_last_payment {
            self.status = InvestmentStatus::Finished;

            if contract_data.return_type == InvestmentReturnType::Coupon {
                self.paid += self.deposited;
                amount_to_transfer += self.deposited;
            }
        }

        amount_to_transfer
    }

    fn calculate_initial_status(claim_block_days: &u64) -> InvestmentStatus {
        let status: InvestmentStatus = match claim_block_days {
            claim_block_days if *claim_block_days > 0 => InvestmentStatus::Blocked,
            _ => InvestmentStatus::Claimable,
        };
        status
    }

    fn calculate_claimable_ts(env: &Env, claim_block_days: &u64) -> u64 {
        env.ledger().timestamp() + (claim_block_days * SECONDS_IN_DAY)
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

    pub fn process_multiple_payments(
        &mut self,
        env: &Env,
        contract_data: &ContractData,
        num_payments: u32,
    ) -> i128 {
        if self.status != InvestmentStatus::CashFlowing {
            self.status = InvestmentStatus::CashFlowing;
        }

        let mut total_amount: i128 = self.regular_payment * num_payments as i128;
        self.paid += total_amount;
        self.last_transfer_ts = env.ledger().timestamp();
        self.payments_transferred += num_payments;

        let is_last_payment = self.payments_transferred >= contract_data.return_months;

        if is_last_payment {
            self.status = InvestmentStatus::Finished;

            if contract_data.return_type == InvestmentReturnType::Coupon {
                self.paid += self.deposited;
                total_amount += self.deposited;
            }
        }

        total_amount
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
#[contracttype]
pub enum InvestmentStatus {
    Blocked = 1,
    Claimable = 2,
    CashFlowing = 4,
    Finished = 5,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
#[contracttype]
pub enum InvestmentReturnType {
    ReverseLoan = 1,
    Coupon = 2,
}

impl FromNumber for InvestmentReturnType {
    fn from_number<N>(value: N) -> Option<InvestmentReturnType>
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
