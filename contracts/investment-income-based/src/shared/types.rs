use soroban_sdk::{contracttype, Address, Env};

use crate::constants::SECONDS_IN_DAY;
use crate::investment::InvestmentReturnType;

/// Constructor parameters supplied at deployment.
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

/// Persisted immutable/semi-immutable configuration used across business flows.
#[contracttype]
pub struct ContractData {
    pub interest_rate: u32,
    pub claim_block_days: u64,
    pub fundraising_days: u64,
    pub ts_fundraising_ends: u64,
    pub ts_payments_start: u64,
    pub token: Address,
    pub price_oracle: Address,
    pub return_type: InvestmentReturnType,
    pub return_months: u32,
    pub min_per_investment: i128,
    pub goal: i128,
    pub amount_to_pay_per_month: i128,
}

impl ContractData {
    /// Builds persisted configuration from deployment parameters.
    ///
    /// Computes fundraising and payment-start timestamps relative to current
    /// ledger time.
    pub fn from_investment_contract_params(
        env: &Env,
        params: &InvestmentContractParams,
        token: Address,
        price_oracle: Address,
    ) -> Self {
        let ts_fundraising_ends = env.ledger().timestamp() + (params.fundraising_days * SECONDS_IN_DAY);
        let ts_payments_start = ts_fundraising_ends + (params.claim_block_days * SECONDS_IN_DAY);

        Self {
            interest_rate: params.i_rate,
            claim_block_days: params.claim_block_days,
            fundraising_days: params.fundraising_days,
            ts_fundraising_ends,
            ts_payments_start,
            token,
            price_oracle,
            return_type: InvestmentReturnType::from_number(params.return_type).unwrap(),
            return_months: params.return_months,
            min_per_investment: params.min_per_investment,
            goal: params.goal,
            amount_to_pay_per_month: 0,
        }
    }
}

#[derive(Clone)]
#[contracttype]
/// Instance-storage keys used by contract state.
pub enum DataKey {
    ContractData,
    NextPaymentRound,
    Investment(u32),
    ContractBalances,
    EmergencyCloseState,
    Collateral,
}