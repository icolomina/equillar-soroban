use soroban_sdk::{contractevent, contracttype, Env};
use stellar_contract_utils::math::wad::Wad;

const LOWER_AMOUNT_FOR_COMMISSION_REDUCTION: i128 = 100;
const LOWER_DIVISOR: u32 = 10;
const UPPER_DIVISOR: u32 = 60;
const AMOUNT_PER_COMMISSION_REDUCTION: i128 = 400;

pub fn calculate_rate_denominator(amount: &i128, decimals: u32) -> u32 {
    let scale_factor = 10_i128.pow(decimals);
    let token_amount = amount / scale_factor;

    if token_amount <= LOWER_AMOUNT_FOR_COMMISSION_REDUCTION {
        return LOWER_DIVISOR;
    }

    let a =
        (token_amount - LOWER_AMOUNT_FOR_COMMISSION_REDUCTION) / AMOUNT_PER_COMMISSION_REDUCTION;
    if a > UPPER_DIVISOR as i128 {
        return UPPER_DIVISOR;
    }

    LOWER_DIVISOR + a as u32
}

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
        }
    }

    pub fn sum(&self) -> i128 {
        self.comission + self.project + self.reserve
    }

    pub fn recalculate_from_investment(&mut self, amounts: &Amount) {
        self.comission += amounts.amount_to_commission;
        self.reserve += amounts.amount_to_reserve_fund;
        self.project += amounts.amount_to_invest;
        self.received_so_far += amounts.amount_to_reserve_fund + amounts.amount_to_invest;
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
    }

    pub fn recalculate_from_project_to_reserver_movement(&mut self, amount: &i128) {
        self.project -= amount;
        self.reserve += amount;
        self.moved_from_project_to_reserve += amount;
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
        }
        .publish(env);
    }
}

pub struct Amount {
    pub amount_to_invest: i128,
    pub amount_to_reserve_fund: i128,
    pub amount_to_commission: i128,
}

impl Amount {
    pub fn get_invested_amount(&self) -> i128 {
        self.amount_to_invest + self.amount_to_reserve_fund
    }
}

pub trait CalculateAmounts {
    fn from_investment(e: &Env, amount: &i128, i_rate: &u32, decimals: u8) -> Amount;
}

impl CalculateAmounts for Amount {
    fn from_investment(e: &Env, amount: &i128, i_rate: &u32, decimals: u8) -> Amount {
        let rate_denominator: u32 = calculate_rate_denominator(amount, decimals as u32);

        let amount_wad = Wad::from_token_amount(e, *amount, decimals);
        let commission_rate = Wad::from_ratio(e, *i_rate as i128, (rate_denominator as i128) * 10_000);

        let reserve_rate = Wad::from_ratio(e, 5, 100);

        let amount_to_commission_wad = amount_wad * commission_rate;
        let amount_to_reserve_fund_wad = amount_wad * reserve_rate;
        let amount_to_invest_wad = amount_wad - amount_to_commission_wad - amount_to_reserve_fund_wad;

        Amount {
            amount_to_commission: amount_to_commission_wad.to_token_amount(e, decimals),
            amount_to_reserve_fund: amount_to_reserve_fund_wad.to_token_amount(e, decimals),
            amount_to_invest: amount_to_invest_wad.to_token_amount(e, decimals),
        }
    }
}
