use soroban_sdk::{contractevent, contracttype, Address, Env};

use crate::investment::InvestmentReturnType;

pub trait FromNumber {
    fn from_number<N>(number: N) -> Option<Self>
    where
        Self: Sized,
        N: Into<u32>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracttype]
pub enum State {
    Active = 2,
    FundsReached = 3,
}

impl State {
    /// Emits a ContractStateUpdated event
    pub fn emit_event(&self, env: &Env) {
        ContractStateUpdated { new_state: *self }.publish(env);
    }
}

#[contractevent(topics = ["STUPDATED"])]
pub struct ContractStateUpdated {
    pub new_state: State,
}

#[contracttype]
pub struct InvestmentContractParams {
    pub i_rate: u32,
    pub claim_block_days: u64,
    pub goal: i128,
    pub return_type: u32,
    pub return_months: u32,
    pub min_per_investment: i128,
}

#[contracttype]
pub struct ContractData {
    pub interest_rate: u32,
    pub claim_block_days: u64,
    pub token: Address,
    pub project_address: Address,
    pub state: State,
    pub return_type: InvestmentReturnType,
    pub return_months: u32,
    pub min_per_investment: i128,
    pub goal: i128,
}

impl ContractData {
    pub fn from_investment_contract_params(
        params: &InvestmentContractParams,
        token: Address,
        project_address: Address,
    ) -> Self {
        ContractData {
            interest_rate: params.i_rate,
            claim_block_days: params.claim_block_days,
            token,
            project_address,
            state: State::Active,
            return_type: InvestmentReturnType::from_number(params.return_type).unwrap(),
            return_months: params.return_months,
            min_per_investment: params.min_per_investment,
            goal: params.goal,
        }
    }
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    ContractData,
    Investment(u32),
    ClaimsMap,
    MultisigRequest,
    ContractBalances,
}
