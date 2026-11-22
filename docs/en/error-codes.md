---
sidebar_position: 14
title: Error codes appendix
---

# Appendix: business error codes

All business errors are declared in `src/shared/types.rs` as the canonical `Error` enum. Values are stable numeric discriminants — integrators can rely on them for their own error mapping. The enum contains no orphan codes: every declared variant is returned by some contract path. Codes 52 (`EmptyEmergencyPool`) and 55 (`PaymentsObligationsRemaining`) were removed from the enum after becoming unused under the new `withdrawn_all` semantics.

| Code | Name | When it fires |
|---:|---|---|
| 1 | `AddressInsufficientBalance` | The investor holds less than the investment amount, or the collateral provider less than the deposit. |
| 2 | `ContractReserveInsufficientBalance` | A payment amount exceeds the contract's token balance, or the company transfer would leave the projected funding below `amount_to_pay_per_month`. |
| 3 | `ContractInsufficientBalance` | The contract lacks liquidity to pay a refund/payment/withdrawal. |
| 5 | `AmountLessThanMinimum` | Investment below `min_per_investment`. |
| 6 | `InterestRateMustBeGreaterThanZero` | Constructor validation. |
| 7 | `GoalMustBeGreaterThanZero` | Constructor validation. |
| 8 | `UnsupportedReturnType` | Constructor: `return_type` must be `1` (ReverseLoan) or `2` (Coupon). |
| 9 | `ReturnMonthsMustBeGreaterThanZero` | Constructor validation. |
| 10 | `MinPerInvestmentMustBeGreaterThanZero` | Constructor validation. |
| 11 | `CmrUpperDividorMustBeGreaterThanZero` | Constructor validation. |
| 12 | `CmrLowerDividorMustBeGreaterThanZero` | Constructor validation. |
| 13 | `CmrReductorMustBeGreaterThanZero` | Constructor validation. |
| 14 | `AddressHasNotInvested` | The referenced `position_id` does not exist (or the id→address mapping is missing). |
| 15 | `CmrUpperDivisorMustBeGreaterTheCmrLowerDivisor` | Constructor validation (`cmr_upper_divisor` must be greater than `cmr_lower_divisor`). |
| 16 | `AddressAlreadyHoldsRole` | Granting a role to an address that already holds it. |
| 17 | `AddressRoleAlreadyExists` | Granting `company` or `manager` while that role already has a member (single-holder rule). |
| 28 | `RecipientCannotReceivePayment` | Token transfer rejected by the token contract (e.g. the receiving contract cannot be trusted to accept). |
| 29 | `InvalidPaymentData` | Token transfer returned an invalid result. |
| 30 | `GoalReached` | `invest` after the fundraising goal has been reached. |
| 33 | `CollateralLevelNotComputable` | The collateral level could not be computed (oracle pricing unavailable) on `add_collateral`. |
| 34 | `OnlyOneCollateralTokenAllowed` | Collateral already configured with a different token. |
| 36 | `CollateralNotConfigured` | No collateral configured for this instance. |
| 37 | `CollateralBalanceIsEmpty` | `return_collateral_to_company` with nothing held. |
| 38 | `PositionCompleted` | Operation on a completed position (payment, refund, collateral settlement). |
| 39 | `FundrasingTimeOngoingYet` | Operation requiring the fundraising period to be over (withdrawals, emergency activation). |
| 40 | `FundrasingTimeExceeded` | `invest` after the fundraising window closed. |
| 42 | `PaymentAlreadyProcessedForThisPeriod` | The position's round counter does not match the current payment round. |
| 43 | `NextPaymentCannotBeScheduledYet` | Company transfer before `ts_payments_start`. |
| 45 | `EmptyPaymentObligations` | Accounting guard on `pay_with_collateral`: total payment obligations are not positive (degenerate accounting state). |
| 46 | `EmptyRefundAmount` | Refund of a zero amount (defensive guard). |
| 47 | `OwnerInsufficientBalance` | The `company` source lacks funds for a transfer. |
| 48 | `EmergencyAlreadyActive` | Emergency activation while already active. |
| 49 | `EmergencyNotActive` | `withdrawn_all` or `withdrawn_all_collateral` without the emergency gate active. |
| 50 | `OperationNotAllowedInEmergency` | Any capital-moving operation during emergency mode. |
| 51 | `PendingCommissionWithdrawal` | Emergency activation while commissions are still pending withdrawal. |
| 53 | `PositionIdAlreadyExists` | The `position_id` was already used. |
| 54 | `LiquidationPaymentsOutOfPeriod` | Enable/disable liquidations outside the fundraising-end → payments-start window. |
| 56 | `BalanceUpdateOverflow` | Checked addition overflow during a `Balance` recalculation. |
| 57 | `BalanceUpdateUnderflow` | Checked subtraction underflow during a `Balance` recalculation. |
| 58 | `CollateralAmountOverflow` | Total collateral would overflow `i128`. |
| 59 | `CollateralPriceOracleError` | Cross price could not be computed during liquidation (no oracle price or zero contract-token price). |
| 60 | `CollateralStillRemainingOnContract` | `withdrawn_all` with collateral still held by the contract — sweep it first with `withdrawn_all_collateral`. |
| 61 | `TooLargeInvestment` | Deposit too large for the WAD arithmetic — clean no-op rejection: nothing is transferred or stored. |
| 62 | `BalanceEquationViolation` | The accounting-equation check failed during a `Balance` recalculation — only reachable through an internal accounting bug. |
| 63 | `EmptyCommissionBalance` | Commission withdrawal with no pending commission. |

## Error propagation model

- **Validation errors** fire *before* any token transfer or state change, so a rejected call is always a `no-op`.
- **Transfer errors** (`RecipientCannotReceivePayment`, `InvalidPaymentData`) map token-contract failures to stable business codes.
- **Accounting errors** (`BalanceUpdateOverflow`/`Underflow`/`BalanceEquationViolation`) fire *before* persisting a broken balance. The equation `assets = liabilities + equity` is re-checked inside every recalculation, so inconsistent accounting can never be persisted.
- The known code ranges are tested one-by-one in `error_tests.rs` (see [Testing](./tests.md)).
