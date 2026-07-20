use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::constants::SECONDS_IN_DAY;

/// Aggregated accounting snapshot for contract-level financial state.
///
/// Fields track reserves, project funds, commissions, obligations, and
/// collateral/emergency/refund side effects over time.
#[contracttype]
pub struct ContractBalance {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub comission_withdrawal: i128,
    pub payments: i128,
    pub project_withdrawals: i128,
    pub payment_obligations: i128,
    pub collateral_received: i128,
    pub collateral_liquidated: i128,
    pub collateral_returned: i128,
    pub refunded_to_investor: i128,
}

impl Default for ContractBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractBalance {
    /// Creates a zero-initialized balance snapshot.
    pub fn new() -> Self {
        Self {
            reserve: 0,
            project: 0,
            comission: 0,
            comission_withdrawal: 0,
            payments: 0,
            project_withdrawals: 0,
            payment_obligations: 0,
            collateral_received: 0,
            collateral_liquidated: 0,
            collateral_returned: 0,
            refunded_to_investor: 0,
        }
    }

    /// Applies accounting effects of a newly accepted investment.
    pub fn recalculate_from_position(&mut self, position: &Position) -> Result<(), Error> {
        self.comission = self
            .comission
            .checked_add(position.commission)
            .ok_or(Error::BalanceUpdateOverflow)?;
        self.project = self
            .project
            .checked_add(position.deposited)
            .ok_or(Error::BalanceUpdateOverflow)?;
        self.payment_obligations = self
            .payment_obligations
            .checked_add(position.total)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of a regular investor payment.
    pub fn recalculate_from_payment_to_investor(&mut self, amount: i128) -> Result<(), Error> {
        self.reserve = self
            .reserve
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.payments = self
            .payments
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.payment_obligations = self
            .payment_obligations
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of a company transfer into reserve.
    pub fn recalculate_from_company_contribution(&mut self, amount: i128) -> Result<(), Error> {
        self.reserve = self
            .reserve
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of withdrawing project funds.
    pub fn recalculate_from_company_withdrawal(&mut self, amount: i128) -> Result<(), Error> {
        self.project = self
            .project
            .checked_sub(amount)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.project_withdrawals = self
            .project_withdrawals
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of commission withdrawal.
    pub fn recalculate_from_comission_withdrawal(&mut self, amount: i128) -> Result<(), Error> {
        self.comission_withdrawal = self
            .comission_withdrawal
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of collateral deposit reception.
    pub fn recalculate_from_collateral_received(&mut self, amount: i128) -> Result<(), Error> {
        self.collateral_received = self
            .collateral_received
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects when collateral is liquidated for a position.
    pub fn recalculate_from_collateral_liquidated(
        &mut self,
        collateral_amount: i128,
        remaining_obligations: i128,
    ) -> Result<(), Error> {
        self.collateral_liquidated = self
            .collateral_liquidated
            .checked_add(collateral_amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.payment_obligations = self
            .payment_obligations
            .checked_sub(remaining_obligations)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        Ok(())
    }

    /// Applies accounting effects when collateral is returned to provider.
    pub fn recalculate_from_collateral_returned(&mut self, amount: i128) -> Result<(), Error> {
        self.collateral_returned = self
            .collateral_returned
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        Ok(())
    }

    /// Applies accounting effects of one emergency payout.
    ///
    /// Payouts consume reserve first, then project balance for the remainder.
    pub fn recalculate_from_emergency_payment(
        &mut self,
        amount: i128,
        remaining_obligations: i128,
    ) -> Result<(), Error> {
        let reserve_to_use = if self.reserve >= amount {
            amount
        } else {
            self.reserve
        };
        let project_to_use = amount - reserve_to_use;

        self.reserve = self
            .reserve
            .checked_sub(reserve_to_use)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.project = self
            .project
            .checked_sub(project_to_use)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.payments = self
            .payments
            .checked_add(amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.payment_obligations = self
            .payment_obligations
            .checked_sub(remaining_obligations)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        Ok(())
    }

    /// Applies accounting effects of investor refund.
    pub fn recalculate_from_refunded_to_investor(
        &mut self,
        position: &Position,
    ) -> Result<(), Error> {
        self.project = self
            .project
            .checked_sub(position.deposited)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.comission = self
            .comission
            .checked_sub(position.commission)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        let refunded_amount = position
            .deposited
            .checked_add(position.commission)
            .ok_or(Error::BalanceUpdateOverflow)?;

        self.refunded_to_investor = self
            .refunded_to_investor
            .checked_add(refunded_amount)
            .ok_or(Error::BalanceUpdateOverflow)?;

        let obligations_to_sub = position
            .total
            .checked_sub(position.paid)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        self.payment_obligations = self
            .payment_obligations
            .checked_sub(obligations_to_sub)
            .ok_or(Error::BalanceUpdateUnderflow)?;

        Ok(())
    }

    pub fn reset_balance(&mut self) {
        self.reserve = 0_i128;
        self.project = 0_i128;
        self.comission = 0_i128;
        self.comission_withdrawal = 0_i128;
        self.payments = 0_i128;
        self.project_withdrawals = 0_i128;
        self.payment_obligations = 0_i128;
        self.collateral_received = 0_i128;
        self.collateral_liquidated = 0_i128;
        self.collateral_returned = 0_i128;
        self.refunded_to_investor = 0_i128;
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
    RecipientCannotReceivePayment = 28,
    InvalidPaymentData = 29,
    GoalReached = 30,
    CollateralLevelTooLow = 33,
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
    EmptyEmergencyPool = 52,
    PositionIdAlreadyExists = 53,
    LiquidationPaymentsOutOfPeriod = 54,
    PaymentsObligationsRemaining = 55,
    BalanceUpdateOverflow = 56,
    BalanceUpdateUnderflow = 57,
    CollateralAmountOverflow = 58,
    CollateralPriceOracleError = 59,
}
