---
sidebar_position: 7
title: Refunds
---

# Refunds

Refunds are only allowed during the fundraising period, and only when the project's fundraising is cancelled. This can happen, for example, when the company determines that the capital raised is far from the target and unlikely to be reached within the remaining funding time.

## The refund_investor function

The admin can refund a position through `refund_investor(token_id)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  refund_investor \
  --token_id 1234
```

> **Access note:** despite closing a position created by an operator, this is an **admin-only** function.

Refunds are only valid if **all** of the following hold:

| Check | Error if not satisfied |
|---|---|
| The position exists | `AddressHasNotInvested` (14) |
| The current ledger timestamp is before `ts_fundraising_ends` | `FundrasingTimeExceeded` (40) |
| The position is not completed | `PositionCompleted` (38) |
| The refund amount is non-zero | `EmptyRefundAmount` (46) |
| The contract holds enough liquidity to pay it | `ContractInsufficientBalance` (3) |

## The refund amount

The refund returns exactly the capital the investor originally transferred in — the **gross amount**, not the allocation. Since the deposited amount is computed as:

```text
deposited = received amount (truncation-free) - commission
```

the total returned is always exactly what the investor contributed: `total to return = deposited + commission`.
The position is then marked as `completed` and the aggregate accounting adjustments are applied.

## Accounting: full commission reversal

`recalculate_from_refunded_to_investor` applies four movements and verifies the equation:

- `assets.managed_volume -= total` (the managed volume decreases since the deposit has been returned to the investor)
- `liabilities.payment_obligations -= total` (payment obligations decrease since the position disappears)
- `assets.contract_earnings -= commission` (commission also decreases since it is returned to the investor as well)
- `equity.contract_commissions -= commission` (equity decreases since the commission is returned to the investor)

> **Important consequence:** a refund fully reverses commission accrual. There is no "ghost" commission left in `contract_commissions` — the integrator cannot withdraw through `withdrawn_commissions` a commission belonging to a refunded position. This differs from the accounting model described in older documentation, where commission was kept accrued after a refund.

## Events and balance after a refund

**1. `BalanceUpdated`**: the updated balance snapshot:

```rust
BalanceUpdated {
    assets: BalanceAsset { payment_rigths, contract_earnings, managed_volume, managed_collateral },
    liabilities: BalanceLiabilities { payment_obligations, return_collateral_obligations },
    equity: BalanceEquity { contract_commissions },
}
```

**2. `InvestmentDepositRefunded`**: refund details

```rust
InvestmentDepositRefunded {
    addr,     // the address of the contract
    to,       // the address the deposit is refunded to
    amount,   // the total amount refunded (deposited + commission)
}
```

### Reading this event with stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "InvestmentDepositRefunded,*"
```

## Refunds vs. emergency close

> It is **not** the same as [emergency close](./emergency-close.md): `refund_investor` returns the deposited amount to the investor while the contract can still receive deposits (at that point the company has not started operating yet and therefore has not started spending the raised funds). Emergency close is the post-fundraising path and happens when the company cannot meet its payment obligations.
