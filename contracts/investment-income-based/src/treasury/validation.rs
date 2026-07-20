use crate::require;
use crate::shared::types::{ContractData, Error};

pub fn validate_withdrawal(
    amount: i128,
    project_balance: i128,
    current_ts: u64,
    contract_data: &ContractData,
) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet,
        project_balance >= amount,
        Error::ContractInsufficientBalance
    );
    Ok(())
}

pub fn validate_withdrawal_commission(
    amount: i128,
    current_ts: u64,
    contract_data: &ContractData,
) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends,
        Error::FundrasingTimeOngoingYet,
        amount > 0,
        Error::ContractInsufficientBalance
    );
    Ok(())
}
