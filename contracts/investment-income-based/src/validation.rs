use crate::balance::ContractBalance;
use crate::data::ContractData;
use crate::investment::Investment;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{contracterror, Address, Env};

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracterror]
pub enum Error {
    // -- Constructor --
    InterestRateMustBeGreaterThanZero = 6,
    GoalMustBeGreaterThanZero = 7,
    UnsupportedReturnType = 8,
    ReturnMonthsMustBeGreaterThanZero = 9,
    MinPerInvestmentMustBeGreaterThanZero = 10,

    // -- Investment --
    AddressInsufficientBalance = 1,
    AmountLessThanMinimum = 5,
    GoalReached = 30,
    FundrasingTimeExceeded = 40,

    // -- Payment processing --
    AddressHasNotInvested = 14,
    InvestmentCompleted = 38,
    PaymentAlreadyProcessedForThisPeriod = 42,
    ContractReserveInsufficientBalance = 2,

    // -- Withdrawal --
    FundrasingTimeOngoingYet = 39,
    ContractInsufficientBalance = 3,

    // -- Company transfer --
    NextPaymentCannotBeScheduledYet = 43,

    // -- Token transfer execution --
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29,

    // -- Collateral --
    CollateralLevelTooLow = 33,
    OnlyOneCollateralTokenAllowed = 34,
    CollateralNotConfigured = 36,
    CollateralBalanceIsEmpty = 37,
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

/// Validates that there is sufficient reserve balance for payment
pub fn validate_reserve_balance(
    amount_to_transfer: i128,
    investment: &Investment,
    contract_balances: &ContractBalance,
    next_payment_round: u32
) -> Result<(), Error> {
    require!(
        investment.payments_transferred == next_payment_round, Error::PaymentAlreadyProcessedForThisPeriod,
        amount_to_transfer <= contract_balances.reserve, Error::ContractReserveInsufficientBalance
    );
    Ok(())
}

/// Validates investment parameters before accepting investment
pub fn validate_investment(
    amount: i128,
    contract_data: &ContractData,
    investor_balance: i128,
    current_ts: u64,
    contract_balance: &ContractBalance
) -> Result<(), Error> {
    require!(
        contract_balance.received_so_far < contract_data.goal, Error::GoalReached,
        amount >= contract_data.min_per_investment, Error::AmountLessThanMinimum,
        investor_balance >= amount, Error::AddressInsufficientBalance,
        current_ts < contract_data.ts_fundraising_ends, Error::FundrasingTimeExceeded
    );
    Ok(())
}

/// Validates sufficient project balance for withdrawal
pub fn validate_withdrawal(amount: i128, project_balance: i128, current_ts: u64, contract_data: &ContractData) -> Result<(), Error> {
    require!(
        current_ts > contract_data.ts_fundraising_ends, Error::FundrasingTimeOngoingYet,
        project_balance >= amount, Error::ContractInsufficientBalance
    );
    Ok(())
}

/// Validates sufficient balance for company transfer.
///
/// For any round except the last, verifies the transfer covers the monthly interest shortfall.
/// For the last round, verifies the reserve after the transfer covers all remaining
/// `payment_obligations` — this is critical for Coupon investments where the final payment
/// also returns the full deposited principal.
pub fn validate_company_transfer(
    e: &Env,
    token: &TokenClient,
    owner: &Address,
    contract_data: &ContractData,
    contract_balance: &ContractBalance,
    amount: i128,
    next_payment_round: u32,
) -> Result<(), Error> {
    let current_ts = e.ledger().timestamp();
    require!(current_ts >= contract_data.ts_payments_start, Error::NextPaymentCannotBeScheduledYet);

    let is_last_round = next_payment_round == contract_data.return_months - 1;
    if is_last_round {
        require!(
            contract_balance.reserve + amount >= contract_balance.payment_obligations,
            Error::ContractReserveInsufficientBalance
        );
    } else {
        require!(
            amount > (contract_data.amount_to_pay_per_month - contract_balance.reserve),
            Error::AddressInsufficientBalance
        );
    }

    require!(token.balance(owner) >= amount, Error::AddressInsufficientBalance);
    Ok(())
}
