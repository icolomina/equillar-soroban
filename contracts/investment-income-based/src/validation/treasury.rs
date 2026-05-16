use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env};

use crate::require;
use crate::shared::{ContractBalance, ContractData};
use crate::validation::Error;

/// Validates project withdrawal timing and available project balance.
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

/// Validates commission withdrawal timing and pending commission amount.
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

/// Validates company transfer constraints for payment-round funding.
///
/// Enforces payment schedule start, reserve sufficiency (with special handling
/// on final round), and source address token balance.
pub fn validate_company_transfer(
    env: &Env,
    token: &TokenClient,
    owner: &Address,
    contract_data: &ContractData,
    contract_balance: &ContractBalance,
    amount: i128,
    next_payment_round: u32,
) -> Result<(), Error> {
    let current_ts = env.ledger().timestamp();
    require!(
        current_ts >= contract_data.ts_payments_start,
        Error::NextPaymentCannotBeScheduledYet
    );

    let is_last_round = next_payment_round == contract_data.return_months - 1;
    if is_last_round {
        require!(
            contract_balance.reserve + amount >= contract_balance.payment_obligations,
            Error::ContractReserveInsufficientBalance
        );
    } else {
        require!(
            amount >= (contract_data.amount_to_pay_per_month - contract_balance.reserve),
            Error::ContractReserveInsufficientBalance
        );
    }

    require!(token.balance(owner) >= amount, Error::OwnerInsufficientBalance);
    Ok(())
}