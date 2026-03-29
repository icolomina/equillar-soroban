use soroban_sdk::{Address, Env, contracttype};

use crate::{constants::SECONDS_IN_DAY, investment::InvestmentReturnType};

#[contracttype]
pub struct InvestmentContractParams {
    pub i_rate: u32,
    pub claim_block_days: u64,
    pub fundraising_days: u64,
    pub goal: i128,
    pub return_type: u32,
    pub return_months: u32,
    pub min_per_investment: i128,
}

#[contracttype]
pub struct ContractData {
    pub interest_rate: u32,
    pub claim_block_days: u64,
    pub fundraising_days: u64,
    pub ts_fundraising_ends: u64,
    pub ts_payments_start: u64,
    pub token: Address,
    pub project_address: Address,
    pub price_oracle: Address,
    pub return_type: InvestmentReturnType,
    pub return_months: u32,
    pub min_per_investment: i128,
    pub goal: i128,
    pub amount_to_pay_per_month: i128
}

impl ContractData {
    pub fn from_investment_contract_params(
        env: &Env,
        params: &InvestmentContractParams,
        token: Address,
        project_address: Address,
        price_oracle: Address,
    ) -> Self {

        let ts_fundraising_ends = env.ledger().timestamp() + (params.fundraising_days * SECONDS_IN_DAY);
        let ts_payments_start = ts_fundraising_ends + (params.claim_block_days * SECONDS_IN_DAY);

        ContractData {
            interest_rate: params.i_rate,
            claim_block_days: params.claim_block_days,
            fundraising_days: params.fundraising_days,
            ts_fundraising_ends: ts_fundraising_ends,
            ts_payments_start: ts_payments_start,
            token,
            project_address,
            price_oracle,
            return_type: InvestmentReturnType::from_number(params.return_type).unwrap(),
            return_months: params.return_months,
            min_per_investment: params.min_per_investment,
            goal: params.goal,
            amount_to_pay_per_month: 0_i128
        }
    }
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    ContractData,
    NextPaymentRound,
    Investment(u32),
    TotalSupply,
    ClaimsMap,
    MultisigRequest,
    ContractBalances,
    Collateral,
    CollateralSigners
}
