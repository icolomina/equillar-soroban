use soroban_sdk::{token::Client as TokenClient, Env};

use crate::shared::types::ContractData;

/// Returns the configured payment token client for this contract.
pub(crate) fn get_token<'a>(env: &'a Env, contract_data: &ContractData) -> TokenClient<'a> {
    TokenClient::new(env, &contract_data.token)
}
