//! Centralized validation layer for business preconditions.
//!
//! Domain modules call these helpers before state transitions or transfers.
//! This keeps failure semantics consistent and maps checks to stable contract
//! error codes.

mod collateral;
mod investment;
mod emergency;
mod payments;
mod treasury;

pub use collateral::{
    validate_add_collateral, validate_collateral_return,
};
pub use emergency::{
    validate_activate_emergency_close, validate_emergency_active, validate_emergency_payment,
};
pub use investment::{
    validate_constructor_params, validate_investment, validate_not_in_emergency,
    validate_refund_investor,
};
pub use payments::validate_reserve_balance;
pub use treasury::{
    validate_company_transfer, validate_withdrawal, validate_withdrawal_commission,
};

use soroban_sdk::contracterror;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
#[contracterror]
/// Canonical business error codes returned by the contract.
///
/// Values are stable numeric discriminants used by tests and integrations.
pub enum Error {
    AddressInsufficientBalance = 1,
    ContractReserveInsufficientBalance = 2,
    ContractInsufficientBalance = 3,
    AmountLessThanMinimum = 5,
    InterestRateMustBeGreaterThanZero = 6,
    GoalMustBeGreaterThanZero = 7,
    UnsupportedReturnType = 8,
    ReturnMonthsMustBeGreaterThanZero = 9,
    MinPerInvestmentMustBeGreaterThanZero = 10,
    AddressHasNotInvested = 14,
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29,
    GoalReached = 30,
    CollateralLevelTooLow = 33,
    OnlyOneCollateralTokenAllowed = 34,
    CollateralNotConfigured = 36,
    CollateralBalanceIsEmpty = 37,
    InvestmentCompleted = 38,
    FundrasingTimeOngoingYet = 39,
    FundrasingTimeExceeded = 40,
    PaymentAlreadyProcessedForThisPeriod = 42,
    NextPaymentCannotBeScheduledYet = 43,
    EmptyPaymentObligations = 45,
    EmptyRefundAmount = 46,
    OwnerInsufficientBalance = 47,
    EmergencyAlreadyActive = 48,
    EmergencyNotActive = 49,
    OperationNotAllowedInEmergency = 50,
    PendingCommissionWithdrawal = 51,
    EmptyEmergencyPool = 52,
}