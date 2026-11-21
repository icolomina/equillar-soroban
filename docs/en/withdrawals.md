---
sidebar_position: 9
title: Withdrawals
---

# Withdrawals

Withdrawals are allowed after the fundraising period ends. Equillar separates **project withdrawals** from **commission withdrawals**, and adds a final full-sweep operation.

> **TTL note — activity may drop during the claim block:** during the claim block window (after fundraising, before `ts_payments_start`) no new investments are accepted and payout rounds have not yet begun. The most notable activity are withdrawals (both company and comissions). This reduced activity can cause the ledger entries storing positions, balances, and related contract data to become archived due to TTL (time-to-live) expiration. Starting with Protocol 23, archived entries in Persistent and Instance storage can be automatically restored as part of a transaction's execution, provided the entry is included in the transaction's restore footprint — which, in practice, is populated automatically when the invocation is simulated via Stellar RPC (the standard flow used by stellar-cli and most SDKs/integrators).
>
> Equillar does not implement a proactive TTL-extension mechanism (e.g. a scheduled "keeper" call to renew TTLs during long inactivity windows). Instead, the contract relies on Protocol 23's automatic restoration during normal invocation. The practical implication for integrators is that the first call made after an extended period of inactivity may incur higher fees, due to the on-chain cost of restoring archived entries, but the call itself does not fail or require a separate manual restore step, as long as it goes through a client that simulates the transaction beforehand. If an integrator submits a transaction with a manually constructed footprint (bypassing RPC simulation) against an archived entry, that transaction will fail rather than auto-restore.

## Company project withdrawals

The admin can move project funds out of the contract through `withdrawn(amount, to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn \
  --amount "3000000000" \
  --to "G....."
```

Company withdrawals are allowed only if all of the following conditions are met:

| Check | Error if violated |
|---|---|
| Contract is not paused / no active emergency | paused guard / `OperationNotAllowedInEmergency` (50) |
| The financing period has ended | `FundrasingTimeOngoingYet` (39) |
| The address receiving the funds holds the `company` role | role error |
| The project balance is sufficient | `ContractInsufficientBalance` (3) |

The project balance is what the company may withdraw: the sum of all user investments **minus commissions**. Following the worked example from [Fundraising and positions](./fundraising-and-positions.md#token-scale-example) (one investment of `500` tokens), the amount available for the project is `497.7272728` tokens, with the remainder allocated to the commission bucket.

## Commission withdrawals

The admin can withdraw accrued commissions through `withdrawn_commissions(to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn_commissions \
  --to "G....."
```

Commission withdrawals are allowed only if all of the following hold:

| Check | Error if violated |
|---|---|
| Contract is not paused / no active emergency | paused guard / (50) |
| The financing period has ended | `FundrasingTimeOngoingYet` (39) |
| The address receiving the funds holds the `manager` role | role error |
| There is pending commission (`equity.contract_commissions > 0`) | `AmountLessThanMinimum` (5) |
| Contract liquidity covers the withdrawal | `ContractInsufficientBalance` (3) |

Notice that `withdrawn_commissions` does **not** require an amount: it withdraws the **full** pending commission (`equity.contract_commissions`). The integrator never needs to track how much is available. Calls to this function should be spaced out enough that commissions are actually available each time, avoiding unnecessary failed calls.

Accounting-wise, `recalculate_from_comission_withdrawal` applies the withdrawal symmetrically on both sides of the balance: `assets.contract_earnings -= amount` and `equity.contract_commissions -= amount`, so withdrawn commissions leave the accounting entirely (they are not "kept accrued", unlike the older model).

## Final sweep — withdrawn_all

The admin can sweep **all remaining token funds** out of the contract to an address holding the `manager` role, by calling `withdrawn_all(to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn_all \
  --to "G....."
```

This is the closing operation of an instance's lifecycle. Its only precondition is the emergency state:

| Check | Error if not satisfied |
|---|---|
| Emergency close is **active** | `EmergencyNotActive` (49) |

If the contract still holds tokens, the full token balance is transferred to `to`, and the `Balance` is **reset** (all fields to zero). Unlike the previous design, `withdrawn_all` does **not** require payment or collateral obligations to be zero: it can be executed with pending obligations and the integrator decides off-chain what to do with the received funds (for example, paying investors outside the contract).

> **Note the deliberate composition:** `withdrawn_all` requires the emergency gate to be active. See [Emergency close](./emergency-close.md) for how activation and the final sweep compose into the closing sequence of an instance. For the remaining collateral there is the equivalent operation [`withdrawn_all_collateral`](./collateral.md#collateral-final-sweep--withdrawn_all_collateral), which also sends the funds to a `manager` address instead of returning them to the provider.

## Events and balance after a company withdrawal

When the company withdraws project funds through `withdrawn`, the balance is updated via `recalculate_from_company_withdrawal`:

- `assets.payment_rigths += amount` (the company's payment obligations to the contract increase)
- `assets.managed_volume -= amount` (the managed volume decreases)

This update is published through the standard `BalanceUpdated` event. In addition, the contract emits a dedicated event with the withdrawal details:

**`WithdrawalDone`**:

```rust
WithdrawalDone {
    addr,              // the address of the contract
    total_withdrawn,   // the total amount withdrawn by the company
}
```

### Reading this event with stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "WithdrawalDone,*"
```

## Events and balance after a commission withdrawal

When the integrator withdraws accrued commissions through `withdrawn_commissions`, the balance update decreases both the commission asset and the pending equity.

**`CommissionWithdrawn`**:

```rust
CommissionWithdrawn {
    addr,     // the address of the contract
    amount,   // the total commission amount withdrawn
}
```

### Reading this event with stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "CommissionWithdrawn,*"
```

## Events and balance after withdrawing all funds

When the admin sweeps all remaining funds through `withdrawn_all`, the balance is fully reset via `balance.reset()` and every field of the equation goes back to `0`:

```text
assets:       payment_rigths, contract_earnings, managed_volume, managed_collateral = 0
liabilities:  payment_obligations, return_collateral_obligations                = 0
equity:       contract_commissions                                              = 0
```

This operation also emits the standard `BalanceUpdated` event (with the balance already reset); in addition, it emits the dedicated event that records the sweep:

**`AllWithdrawn`**:

```rust
AllWithdrawn {
    addr,     // the address of the contract
    amount,   // the total amount withdrawn
}
```

### Reading this event with stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "AllWithdrawn,*"
```
