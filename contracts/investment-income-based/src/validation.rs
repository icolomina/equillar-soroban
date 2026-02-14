use crate::balance::ContractBalance;
use crate::constants::SECONDS_IN_MONTH;
use crate::data::{ContractData, State};
use crate::investment::{Investment, InvestmentStatus};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{contracterror, Address, Env};

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracterror]
pub enum Error {
    AddressInsufficientBalance = 1,
    ContractInsufficientBalance = 2,
    AmountLessThanMinimum = 5,
    InterestRateMustBeGreaterThanZero = 6,
    GoalMustBeGreaterThanZero = 7,
    UnsupportedReturnType = 8,
    ReturnMonthsMustBeGreaterThanZero = 9,
    MinPerInvestmentMustBeGreaterThanZero = 10,
    AddressHasNotInvested = 14,
    AddressInvestmentIsNotClaimableYet = 15,
    AddressInvestmentIsFinished = 16,
    AddressInvestmentNextTransferNotClaimableYet = 17,
    ProjectBalanceInsufficientAmount = 24,
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29,
    WouldExceedGoal = 30,
    GoalAlreadyReached = 31,
    AmountToInvestMustBeGreaterThanZero = 32,
}

/// Macro for validation checks with early return on error
#[macro_export]
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
    ($($cond:expr, $err:expr),+) => {
        $(
            if !$cond {
                return Err($err);
            }
        )+
    };
}

/// Validates constructor parameters for contract initialization
pub fn validate_constructor_params(
    i_rate: u32,
    goal: i128,
    return_months: u32,
    min_per_investment: i128,
) -> Result<(), Error> {
    require!(
        i_rate > 0, Error::InterestRateMustBeGreaterThanZero,
        goal > 0, Error::GoalMustBeGreaterThanZero,
        return_months > 0, Error::ReturnMonthsMustBeGreaterThanZero,
        min_per_investment > 0, Error::MinPerInvestmentMustBeGreaterThanZero
    );
    Ok(())
}

/// Validates that an investment is ready for payment processing
pub fn validate_investment_payment(env: &Env, investment: &Investment) -> Result<(), Error> {
    require!(
        env.ledger().timestamp() >= investment.claimable_ts, Error::AddressInvestmentIsNotClaimableYet,
        investment.status != InvestmentStatus::Finished, Error::AddressInvestmentIsFinished,
        investment.last_transfer_ts == 0 || (env.ledger().timestamp() - investment.last_transfer_ts) >= SECONDS_IN_MONTH, Error::AddressInvestmentNextTransferNotClaimableYet
    );
    Ok(())
}

/// Validates that there is sufficient reserve balance for payment
pub fn validate_reserve_balance(
    amount_to_transfer: i128,
    contract_balances: &ContractBalance,
) -> Result<(), Error> {
    require!(
        amount_to_transfer <= contract_balances.reserve,
        Error::ContractInsufficientBalance
    );
    Ok(())
}

/// Validates investment parameters before accepting investment
pub fn validate_investment(
    amount: i128,
    contract_data: &ContractData,
    investor_balance: i128,
) -> Result<(), Error> {
    require!(
        amount >= contract_data.min_per_investment, Error::AmountLessThanMinimum,
        contract_data.state != State::FundsReached, Error::GoalAlreadyReached,
        investor_balance >= amount, Error::AddressInsufficientBalance,
        amount > 0, Error::AmountToInvestMustBeGreaterThanZero
    );
    Ok(())
}

/// Validates that investment won't exceed funding goal
pub fn validate_investment_goal(
    received_so_far: i128,
    amount_to_invest: i128,
    goal: i128,
) -> Result<(), Error> {
    require!(
        received_so_far + amount_to_invest <= goal,
        Error::WouldExceedGoal
    );
    Ok(())
}

/// Validates sufficient project balance for withdrawal
pub fn validate_withdrawal(amount: i128, project_balance: i128) -> Result<(), Error> {
    require!(
        project_balance >= amount,
        Error::ContractInsufficientBalance
    );
    Ok(())
}

/// Validates sufficient balance for company transfer
pub fn validate_company_transfer(
    token: &TokenClient,
    owner: &Address,
    amount: i128,
) -> Result<(), Error> {
    require!(
        token.balance(owner) >= amount,
        Error::AddressInsufficientBalance
    );
    Ok(())
}

/// Validates sufficient project balance for moving funds to reserve
pub fn validate_move_to_reserve(amount: i128, project_balance: i128) -> Result<(), Error> {
    require!(
        project_balance > amount,
        Error::ProjectBalanceInsufficientAmount
    );
    Ok(())
}

/// Validates that an investment is eligible for investor self-claim
pub fn validate_claim(env: &Env, investment: &Investment) -> Result<(), Error> {
    require!(
        env.ledger().timestamp() >= investment.claimable_ts, Error::AddressInvestmentIsNotClaimableYet,
        investment.status != InvestmentStatus::Finished, Error::AddressInvestmentIsFinished
    );
    Ok(())
}
