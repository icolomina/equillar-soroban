---
sidebar_position: 4
title: The Balance model
---

# The Balance model (double-entry accounting)

The `Balance` structure is the financial heart of Equillar. It is not a single number: it is a **double-entry bookkeeping snapshot** that tracks, at any moment, what the contract manages (assets), what it owes (liabilities), and what the integrator has earned (equity).

Every state-changing operation updates the Balance through a dedicated `recalculate_from_*` function, and each one **verifies the accounting equation before persisting**. If the equation breaks, the operation reverts with a business error instead of leaving inconsistent state on-chain.

## The accounting equation

```text
assets = liabilities + equity
```

This invariant holds after every operation. It is the property the test suite continuously asserts, and it is what makes the contract "accountably sane": at any point you can verify that everything the contract holds is exactly matched by something it owes or has earned.

## The structure

```rust
pub struct Balance {
    pub assets:      BalanceAsset,
    pub liabilities: BalanceLiabilities,
    pub equity:      BalanceEquity,
}

pub struct BalanceAsset {
    pub payment_rigths:    i128,
    pub contract_earnings: i128,
    pub managed_volume:    i128,
    pub managed_collateral: i128,
}

pub struct BalanceLiabilities {
    pub payment_obligations:          i128,
    pub return_collateral_obligations: i128,
}

pub struct BalanceEquity {
    pub contract_commissions: i128,
}
```

## What each field means

### Assets — what the contract manages

| Field | Meaning |
|---|---|
| `managed_volume` | Total value under management owed to investors: every active position's `total`, plus company transfers received for upcoming rounds. Decreases as payments are made to investors. |
| `contract_earnings` | Commission funds held by the contract (asset side). Decreases when commissions are withdrawn or reversed by a refund. |
| `payment_rigths` | The company's contribution counter for payment rounds, **stored with a negative sign**: it moves further into negative values as the company funds each round, and back toward zero when project funds are withdrawn. It keeps the total value equation balanced between custody reclassifications. |
| `managed_collateral` | Collateral held by the contract (in collateral-token units). Decreases when collateral is liquidated or returned. |

> **A note on `payment_rigths`:** when the company sends a transfer of `X` for a payment round, the contract receives `X` tokens *and* reclassifies `X` of `managed_volume` — the accounting entry adds `X` to `managed_volume` and subtracts `X` from `payment_rigths`. Net assets do not change (the tokens physically arrived, and the obligation to the company is now represented by a negative "right"). You can see this live in the `test_flow_with_reverse_loan` test, which asserts `payment_rigths = -261, -522, -783, -1044` after each payment round.

### Liabilities — what the contract owes

| Field | Meaning |
|---|---|
| `payment_obligations` | Total amount still owed to investors across all active positions. |
| `return_collateral_obligations` | The collateral the contract must return to the provider — it equals the collateral currently deposited. |

### Equity — what the integrator has earned

| Field | Meaning |
|---|---|
| `contract_commissions` | Pending commission available to withdraw by the `manager`. Decreases when commissions are withdrawn, or fully reversed when a position is refunded. |

## How operations move the fields

Every `recalculate_from_*` function applies two-to-four field movements and then re-checks the equation:

| Operation | Fields touched |
|---|---|
| `invest` (new position) | `assets.contract_earnings += commission` · `equity.contract_commissions += commission` · `assets.managed_volume += total` · `liabilities.payment_obligations += total` |
| `process_investor_payment` | `assets.managed_volume -= amount` · `liabilities.payment_obligations -= amount` |
| `add_company_transfer` | `assets.payment_rigths -= amount` · `assets.managed_volume += amount` |
| `withdrawn` (company) | `assets.payment_rigths += amount` · `assets.managed_volume -= amount` |
| `withdrawn_commissions` | `assets.contract_earnings -= amount` · `equity.contract_commissions -= amount` |
| `add_collateral` | `assets.managed_collateral += amount` · `liabilities.return_collateral_obligations += amount` |
| `pay_with_collateral` | `assets.managed_collateral -= collateral` · `assets.managed_volume -= remaining` · `liabilities.return_collateral_obligations -= collateral` · `liabilities.payment_obligations -= remaining` |
| `return_collateral_to_company` | `assets.managed_collateral -= amount` · `liabilities.return_collateral_obligations -= amount` |
| `withdrawn_all_collateral` | `assets.managed_collateral -= amount` · `liabilities.return_collateral_obligations -= amount` (remaining collateral is sent to a `manager` address) |
| `refund_investor` | `assets.managed_volume -= total` · `assets.contract_earnings -= commission` · `liabilities.payment_obligations -= total` · `equity.contract_commissions -= commission` |
| `withdrawn_all` | all fields reset to zero |

## Example

The following table shows the journal entries corresponding to these events:

- An investment of `500` units at `5%` (`i_rate = 500`) with commission `2`
- A company withdrawal of `300` units
- A company contribution of `60` units for payments
- A payment of `60` units to an investor

> The values are just examples to demonstrate how the equation `assets = liabilities + equity` holds.

| **Assets** | | | | **Liabilities** | | **Equity** |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `managed_volume` | `managed_collateral` | `payment_rights` | `contract_earnings` | `payment_obligations` | `return_collateral_obligations` | `contract_commissions` |
| 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 522 | 0 | 0 | 2 | 522 | 0 | 2 |
| 222 | 0 | 300 | 2 | 522 | 0 | 2 |
| 282 | 0 | 240 | 2 | 522 | 0 | 2 |
| 222 | 0 | 240 | 2 | 462 | 0 | 2 |

Let's walk through the journal entries one by one:

1. At the start every column of the balance is `0`.
2. When an investment of `500` units comes in, it generates a position with a gain of `22` units and `2` units of commission. This produces `524` units on the asset side: `522` as managed volume and `2` as contract earnings in the form of commission. On the liabilities side, payment obligations to investors increase by `522` units, and last, the equity increases by `2` units as commissions generated over the received investment. So: `522 + 2 = 522 + 2`.
3. When the company withdraws `300` units, the asset side reduces its managed volume by `300` units but, at the same time, the payment rights (an asset for the contract and a liability for the company) also increase by 300. Liabilities and equity stay untouched because the payment obligations to investors do not change and no new commissions were generated. So: `222 + 300 + 2 = 522 + 2`.
4. Now the company transfers `60` units of token to cover a payment round. As in the previous step, only the asset side changes. The managed volume increases by those `60` units (liquidity flowing back into the contract) but the company's payment obligations to the contract (an asset for the contract) decrease by those same `60` units. So: `282 + 240 + 2 = 522 + 2`.
5. Finally, those `60` units received are used to settle the payment of one position. The asset side reduces its managed volume by `60` units but the payment obligations to investors (a liability for the contract) also decrease by those same units. So: `222 + 240 + 2 = 462 + 2`.

The equation survives every movement. If any buggy code path updated the balance incorrectly, `check_balance()` would fail the operation with `Error::BalanceEquationViolation`.

## Reading the balance on-chain

The `get_contract_balance` function requires the caller to be the `admin` or hold any of the roles (`operator`, `company`, `manager`):

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  get_contract_balance \
  --caller "G...."
```

The response is the full `Balance` snapshot, grouped by `assets`, `liabilities`, and `equity`.

## The BalanceUpdated event

Operations that modify the balance emit the `BalanceUpdated` event so that integrators and indexers can follow the accounting off-chain:

```rust
BalanceUpdated {
    assets: BalanceAsset,
    liabilities: BalanceLiabilities,
    equity: BalanceEquity,
}
```
