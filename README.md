# Equillar - Income-Based Investment Contract

> [!WARNING]
> This contract has not been audited. Do not use in production without a professional security review. See LICENSE for details.

## Overview

Equillar is an investment lifecycle engine implemented as a modular Soroban smart contract on Stellar. It lets a project raise funds from multiple investors, represent each position as an NFT, and execute scheduled repayments on-chain with optional liquidation mode, collateral settlement, and emergency close-out flows.

The current implementation follows a modular architecture with role-based access control:

- `admin`: governance and privileged operations.
- `operator`: allowed to create investments.
- `company`: approved source/target addresses for treasury, reserve, and collateral-provider flows.
- `manager`: approved recipients for commission withdrawals.

Core dependencies include OpenZeppelin Stellar ecosystem crates (`stellar-access`, `stellar-macros`, `stellar-tokens`) and Soroban SDK.

## Monorepo Structure

```text
.
├── Cargo.toml
├── README.md
└── contracts/
    └── investment-income-based/
        ├── Cargo.toml
        ├── Makefile
        ├── src/
        ├── tests/
        └── test_snapshots/
```

## Contract Module Architecture

Main crate: `contracts/investment-income-based`

```text
src/
├── lib.rs
├── contract.rs
├── constants.rs
├── investment/
│   ├── mod.rs
│   ├── allocation.rs
│   ├── events.rs
│   ├── types.rs
│   └── validation.rs
├── payments/
│   ├── mod.rs
│   ├── events.rs
│   ├── storage.rs
│   └── validation.rs
├── treasury/
│   ├── mod.rs
│   ├── events.rs
│   └── validation.rs
├── emergency/
│   ├── mod.rs
│   ├── events.rs
│   ├── types.rs
│   └── validation.rs
├── collateral/
│   ├── mod.rs
│   ├── events.rs
│   ├── liquidation.rs
│   ├── storage.rs
│   ├── types.rs
│   └── validation.rs
├── shared/
│   ├── mod.rs
│   ├── events.rs
│   ├── oracle.rs
│   ├── storage.rs
│   ├── storage_helper.rs
│   ├── token.rs
│   └── types.rs
└── validation/
    └── mod.rs
```

### Responsibilities by Module

- `contract.rs`: external entrypoints, access checks, pause guards, role management, liquidation toggles, upgrade flow, and orchestration.
- `investment/`: position creation, refunding, schedule logic, and allocation math.
- `payments/`: regular round payment processing and company reserve transfers.
- `treasury/`: project withdrawals and commission withdrawals.
- `emergency/`: emergency-close activation and proportional payout flow.
- `collateral/`: collateral deposit, valuation, liquidation, and return.
- `shared/`: cross-domain state types, storage helpers, token/oracle helpers, accounting state, and common events.
- `validation/`: reusable business-rule validations and the canonical `Error` enum.

## Constructor Parameters

`__constructor(admin_addr, token_addr, price_oracle, investment_params)` initializes the contract.

| Parameter | Type | Description |
|---|---|---|
| `token_addr` | `Address` | Token used for investments and payouts. |
| `price_oracle` | `Address` | Oracle used for collateral valuation. |
| `i_rate` | `u32` | Annual interest rate in basis points (for example, `500 = 5%`). Must be > 0. |
| `claim_block_days` | `u64` | Delay after fundraising before regular payments can start. |
| `fundraising_days` | `u64` | Fundraising window duration in days. |
| `goal` | `i128` | Maximum capital to raise. Must be > 0. |
| `return_type` | `u32` | `1 = ReverseLoan`, `2 = Coupon`. |
| `return_months` | `u32` | Number of payment rounds. Must be > 0. |
| `min_per_investment` | `i128` | Minimum amount per investment. Must be > 0. |
| `cmr_upper_divisor` | `u32` | Upper divisor used by the collateral/risk math. Must be > 0. |
| `cmr_lower_divisor` | `u32` | Lower divisor used by the collateral/risk math. Must be > 0 and lower than `cmr_upper_divisor`. |
| `cmr_reductor` | `i128` | Reduction factor used by the collateral/risk math. Must be > 0. |

## Public API (Current)

### Governance and Roles

| Function | Access | Purpose |
|---|---|---|
| `__constructor(...)` | `admin_addr` auth | Initializes metadata, validates params, stores config, and sets admin. |
| `grant_operator(operator)` | admin | Grants operator role. |
| `revoke_operator(operator)` | admin | Revokes operator role. |
| `grant_company(company)` | admin | Grants company role. |
| `revoke_company(company)` | admin | Revokes company role. |
| `grant_manager(manager)` | admin | Grants manager role. |
| `revoke_manager(manager)` | admin | Revokes manager role. |
| `transfer_admin_role(new_admin)` | admin | Starts two-step admin transfer (time-limited acceptance). |
| `accept_admin_transfer_role()` | admin-gated endpoint | Accepts pending admin transfer. |
| `upgrade(new_wasm_hash)` | admin + paused | Replaces the contract WASM after pausing. |

### Investment Lifecycle

| Function | Access | Purpose |
|---|---|---|
| `invest(addr, amount, token_id)` | role `operator` on `addr` + not paused | Accepts investment and mints NFT position. |
| `refund_investor(token_id)` | admin | Refunds investment during valid refund window. |
| `process_investor_payment(token_id)` | admin | Executes one regular payment round for a position. |
| `enable_investment_liquidations()` | admin | Enables liquidation-based processing during the allowed window. |
| `disable_investment_liquidations()` | admin | Disables liquidation-based processing during the allowed window. |
| `check_investment_liquidations(caller)` | admin or role holder | Returns the current liquidation mode. |

### Treasury and Balances

| Function | Access | Purpose |
|---|---|---|
| `add_company_transfer(amount, from)` | role `company` on `from` + not paused | Deposits company funds into reserve for the next round. |
| `withdrawn(amount, to)` | admin + role `company` on `to` + not paused | Withdraws project funds to company address. |
| `withdrawn_commissions(to)` | admin + role `manager` on `to` + not paused | Withdraws accumulated commissions to manager address. |
| `withdrawn_all(to)` | admin + role `manager` on `to` + not paused | Withdraws all remaining token balance when obligations are zero. |
| `get_contract_balance(caller)` | caller must be admin or hold any role | Returns `ContractBalance` snapshot. |

### Emergency and Collateral

| Function | Access | Purpose |
|---|---|---|
| `activate_emergency_close()` | admin + not paused | Freezes emergency pool and transitions to emergency settlement. |
| `emergency_pay_investor(token_id)` | admin + not paused | Pays one investor proportionally from emergency pool. |
| `add_collateral(token, amount, symbol, collateral_addr)` | role `company` on `collateral_addr` + not paused | Deposits collateral and updates tracked collateral state. |
| `pay_with_collateral(token_id)` | admin + not paused | Settles a position using collateral pool. |
| `return_collateral_to_company()` | admin + not paused | Returns remaining collateral to provider. |

### Pause Control

| Function | Access | Purpose |
|---|---|---|
| `paused(caller)` | caller must be admin or hold any role | Returns pause flag. |
| `pause(caller)` | admin | Pauses guarded operations. |
| `unpause(caller)` | admin | Unpauses guarded operations. |

## Tests

```text
tests/
├── common/mod.rs
├── success_tests.rs
└── error_tests.rs
```

Run all tests:

```bash
cd contracts/investment-income-based
cargo test
```

Compile tests only (fast validation):

```bash
cd contracts/investment-income-based
cargo test -q --no-run
```

## Build

```bash
cd contracts/investment-income-based

# Dev build
cargo build

# Optimized WASM build
stellar contract build
```

For deployment on testnet, see Equillar docs:
https://docs.equillar.com

## License

Apache 2.0. See [LICENSE](LICENSE).
