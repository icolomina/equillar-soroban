use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::constants::SECONDS_IN_DAY;

#[contracttype]
pub struct BalanceAsset {
    pub payment_rigths: i128,
    pub contract_earnings: i128,
    pub managed_volume: i128,
    pub managed_collateral: i128,
}

#[contracttype]
pub struct BalanceLiabilities {
    pub payment_obligations: i128,
    pub return_collateral_obligations: i128,
}

#[contracttype]
pub struct BalanceEquity {
    pub contract_commissions: i128,
}

#[contracttype]
pub struct Balance {
    pub assets: BalanceAsset,
    pub liabilities: BalanceLiabilities,
    pub equity: BalanceEquity,
}

impl Default for Balance {
    fn default() -> Self {
        Self::new()
    }
}

impl Balance {
    /// Creates a zero-initialized balance snapshot.
    pub fn new() -> Self {
        Self {
            assets: BalanceAsset {
                payment_rigths: 0_i128,
                contract_earnings: 0_i128,
                managed_volume: 0_i128,
                managed_collateral: 0_i128,
            },
            liabilities: BalanceLiabilities {
                payment_obligations: 0_i128,
                return_collateral_obligations: 0_i128,
            },
            equity: BalanceEquity {
                contract_commissions: 0_i128,
            },
        }
    }

    pub fn reset(&mut self) {
        self.assets.payment_rigths = 0_i128;
        self.assets.contract_earnings = 0_i128;
        self.assets.managed_volume = 0_i128;
        self.assets.managed_collateral = 0_i128;

        self.liabilities.payment_obligations = 0_i128;
        self.liabilities.return_collateral_obligations = 0_i128;

        self.equity.contract_commissions = 0_i128;
    }

    pub fn check_balance(&mut self) -> bool {
        let total_assets = self.assets.contract_earnings
            + self.assets.managed_volume
            + self.assets.payment_rigths
            + self.assets.managed_collateral;

        let liabilities =
            self.liabilities.payment_obligations + self.liabilities.return_collateral_obligations;

        let equity = self.equity.contract_commissions;

        total_assets == (liabilities + equity)
    }

    pub fn recalculate_from_position(&mut self, position: &Position) -> Result<(), Error> {
        self.assets.contract_earnings = self
            .assets
            .contract_earnings
            .checked_add(position.commission)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.equity.contract_commissions = self
            .equity
            .contract_commissions
            .checked_add(position.commission)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.liabilities.payment_obligations = self
            .liabilities
            .payment_obligations
            .checked_add(position.total)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_add(position.total)
            .ok_or(Error::BalanceUpdateOverflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_payment_to_investor(&mut self, amount: i128) -> Result<(), Error> {
        self.liabilities.payment_obligations = self
            .liabilities
            .payment_obligations
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;
        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    /// Applies accounting effects of a company transfer into reserve.
    pub fn recalculate_from_company_contribution(&mut self, amount: i128) -> Result<(), Error> {
        self.assets.payment_rigths = self
            .assets
            .payment_rigths
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;
        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_company_withdrawal(&mut self, amount: i128) -> Result<(), Error> {
        self.assets.payment_rigths = self
            .assets
            .payment_rigths
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;
        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    /// Applies accounting effects of commission withdrawal.
    pub fn recalculate_from_comission_withdrawal(&mut self, amount: i128) -> Result<(), Error> {
        self.assets.contract_earnings = self
            .assets
            .contract_earnings
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.equity.contract_commissions = self
            .equity
            .contract_commissions
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_collateral_received(&mut self, amount: i128) -> Result<(), Error> {
        self.assets.managed_collateral = self
            .assets
            .managed_collateral
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.liabilities.return_collateral_obligations = self
            .liabilities
            .return_collateral_obligations
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_collateral_liquidated(
        &mut self,
        collateral_amount: i128,
        remaining_obligations: i128,
    ) -> Result<(), Error> {
        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_sub(remaining_obligations)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.assets.managed_collateral = self
            .assets
            .managed_collateral
            .checked_sub(collateral_amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.liabilities.payment_obligations = self
            .liabilities
            .payment_obligations
            .checked_sub(remaining_obligations)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.liabilities.return_collateral_obligations = self
            .liabilities
            .return_collateral_obligations
            .checked_sub(collateral_amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_collateral_returned(&mut self, amount: i128) -> Result<(), Error> {
        self.assets.managed_collateral = self
            .assets
            .managed_collateral
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.liabilities.return_collateral_obligations = self
            .liabilities
            .return_collateral_obligations
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }

    pub fn recalculate_from_refunded_to_investor(
        &mut self,
        position: &Position,
    ) -> Result<(), Error> {
        self.assets.managed_volume = self
            .assets
            .managed_volume
            .checked_sub(position.total)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.assets.contract_earnings = self
            .assets
            .contract_earnings
            .checked_sub(position.commission)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.liabilities.payment_obligations = self
            .liabilities
            .payment_obligations
            .checked_sub(position.total)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.equity.contract_commissions = self
            .equity
            .contract_commissions
            .checked_sub(position.commission)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        if !self.check_balance() {
            return Err(Error::BalanceEquationViolation);
        }

        Ok(())
    }
}

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
    pub cmr_upper_divisor: u32,
    pub cmr_lower_divisor: u32,
    pub cmr_reductor: i128,
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
    pub return_type: PositionReturnType,
    pub return_months: u32,
    pub min_per_investment: i128,
    pub goal: i128,
    pub amount_to_pay_per_month: i128,
    pub cmr_upper_divisor: u32,
    pub cmr_lower_divisor: u32,
    pub cmr_reductor: i128,
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
        let ts_fundraising_ends =
            env.ledger().timestamp() + (params.fundraising_days * SECONDS_IN_DAY);
        let ts_payments_start = ts_fundraising_ends + (params.claim_block_days * SECONDS_IN_DAY);

        Self {
            interest_rate: params.i_rate,
            claim_block_days: params.claim_block_days,
            fundraising_days: params.fundraising_days,
            ts_fundraising_ends,
            ts_payments_start,
            token,
            price_oracle,
            return_type: PositionReturnType::from_number(params.return_type).unwrap(),
            return_months: params.return_months,
            min_per_investment: params.min_per_investment,
            goal: params.goal,
            amount_to_pay_per_month: 0,
            cmr_upper_divisor: params.cmr_upper_divisor,
            cmr_lower_divisor: params.cmr_lower_divisor,
            cmr_reductor: params.cmr_reductor,
        }
    }
}

#[contracttype]
#[derive(Copy, Clone)]
pub struct Position {
    pub deposited: i128,
    pub commission: i128,
    pub returns: i128,
    pub total: i128,
    pub completed: bool,
    pub regular_payment: i128,
    pub paid: i128,
    pub payments_transferred: u32,
    pub token_id: u32,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
#[contracttype]
/// Repayment profile used by investment positions.
pub enum PositionReturnType {
    ReverseLoan = 1,
    Coupon = 2,
}

impl PositionReturnType {
    /// Parses numeric return type code into enum variant.
    pub fn from_number<N>(value: N) -> Option<Self>
    where
        N: Into<u32>,
    {
        match value.into() {
            1 => Some(Self::ReverseLoan),
            2 => Some(Self::Coupon),
            _ => None,
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
    Position(u32),
    ContractBalances,
    Balance,
    EmergencyCloseState,
    Collateral,
    PositionIdAddress(u32),
    LiquidateInvestmentEnabled,
}

#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum LiquidateInvestmentsStatus {
    Enabled,
    Disabled,
}

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
    CmrUpperDividorMustBeGreaterThanZero = 11,
    CmrLowerDividorMustBeGreaterThanZero = 12,
    CmrReductorMustBeGreaterThanZero = 13,
    AddressHasNotInvested = 14,
    CmrUpperDivisorMustBeGreaterTheCmrLowerDivisor = 15,
    AddressAlreadyHoldsRole = 16,
    AddressRoleAlreadyExists = 17,
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29,
    GoalReached = 30,
    CollateralLevelNotComputable = 33,
    OnlyOneCollateralTokenAllowed = 34,
    CollateralNotConfigured = 36,
    CollateralBalanceIsEmpty = 37,
    PositionCompleted = 38,
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
    PositionIdAlreadyExists = 53,
    LiquidationPaymentsOutOfPeriod = 54,
    BalanceUpdateOverflow = 56,
    BalanceUpdateUnderflow = 57,
    CollateralAmountOverflow = 58,
    CollateralPriceOracleError = 59,
    CollateralStillRemainingOnContract = 60,
    TooLargeInvestment = 61,
    BalanceEquationViolation = 62,
    EmptyCommissionBalance = 63,
}
