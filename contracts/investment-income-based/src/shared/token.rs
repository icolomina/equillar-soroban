use soroban_sdk::token::TokenClient;
use soroban_sdk::{token, Env};

use crate::shared::ContractData;

/// Returns the configured payment token client for this contract.
pub fn get_token<'a>(env: &'a Env, contract_data: &ContractData) -> TokenClient<'a> {
    token::Client::new(env, &contract_data.token)
}