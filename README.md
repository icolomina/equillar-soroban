# Equillar — Income-Based Investment Contract

> [!WARNING]
> This contract has not been audited. Do not use in production without a professional security review. See the LICENSE for more information.

## Overview

Equillar is an open-source Soroban smart contract that tokenizes income-based debt instruments on the Stellar network. It enables a project (the borrower) to raise capital from multiple investors, distribute scheduled repayments on-chain, and optionally back obligations with collateral.

**Key capabilities:**

- **Two return models** 
   -   *ReverseLoan*: equal principal + interest instalments each round. 
   -   *Coupon*: interest-only instalments with full principal returned on the final round.
- **NFT receipts** — each investment is represented as a Non-Fungible Token, making positions transferable and composable.
- **Dual-signing transfers** — `add_company_transfer` and `withdrawn` require both the owner and the project_address to sign the same amount, preventing unilateral fund movement.
- **Collateral support** — a third party can deposit a collateral token (priced via the Reflector oracle) to back investor obligations; collateral is liquidated proportionally per investor via `pay_with_collateral`.
- **Pausable** — the owner can pause and unpause all state-changing operations.

The contract is built on [OpenZeppelin's Stellar libraries](https://docs.openzeppelin.com/stellar-contracts) for ownership, pausability, and NFT base functionality.

## Repository Structure

```
.
├── Cargo.toml
├── README.md
└── contracts/
    └── investment-income-based/
        ├── Cargo.toml
        ├── Makefile
        ├── src/
        │   ├── lib.rs
        │   ├── contract.rs
        │   ├── investment.rs
        │   ├── balance.rs
        │   ├── collateral.rs
        │   ├── amounts.rs
        │   ├── data.rs
        │   ├── validation.rs
        │   ├── storage.rs
        │   ├── events.rs
        │   └── constants.rs
        └── tests/
            ├── common/
            │   └── mod.rs
            ├── success_tests.rs
            └── error_tests.rs
```

## Contract Parameters

| Parameter | Type | Description |
|---|---|---|
| `i_rate` | `u32` | Annual interest rate in basis points (e.g. 500 = 5%). Must be > 0. |
| `fundraising_days` | `u64` | Duration of the fundraising window in days. |
| `claim_block_days` | `u64` | Grace period after fundraising before payments can start. |
| `goal` | `i128` | Maximum capital to raise. Must be > 0. |
| `return_type` | `u32` | `1` = ReverseLoan, `2` = Coupon. |
| `return_months` | `u32` | Number of monthly payment rounds. Must be > 0. |
| `min_per_investment` | `i128` | Minimum amount per individual investment. Must be > 0. |

## Public Functions

### Constructor

| Function | Auth | Description |
|---|---|---|
| `__constructor` | owner | Initializes the contract, validates parameters, sets NFT metadata, and stores `ContractData`. |

### Investment

| Function | Auth | Description |
|---|---|---|
| `invest(addr, amount)` | investor | Transfers tokens from investor to contract, mints an NFT receipt, and records the investment with pre-calculated repayment schedule. |

### Payment Lifecycle

| Function | Auth | Description |
|---|---|---|
| `add_company_transfer(amount)` | owner + project_address | Deposits funds into the reserve for the upcoming payment round. Enforces final-round coverage for Coupon investments. Advances `next_payment_round`. |
| `process_investor_payment(token_id)` | owner | Pays the current round's instalment to the NFT holder. Marks investment completed after the last round. |

### Fund Management

| Function | Auth | Description |
|---|---|---|
| `withdrawn(amount)` | owner + project_address | Transfers project funds to `project_address` after the fundraising period ends. |
| `get_contract_balance()` | owner | Returns the current `ContractBalance` breakdown. |

### Collateral

| Function | Auth | Description |
|---|---|---|
| `add_collateral(token, amount, symbol, addr)` | collateral_addr | Deposits collateral tokens. Only one token type allowed per contract. Coverage level is computed via the Reflector oracle. |
| `pay_with_collateral(token_id)` | owner + project_address | Liquidates the investor's pro-rata collateral share and marks the investment completed. |
| `return_collateral_to_company()` | owner | Returns the entire remaining collateral balance to the collateral provider. |

### Pausable

| Function | Auth | Description |
|---|---|---|
| `pause(caller)` | owner | Pauses all state-changing operations. |
| `unpause(caller)` | owner | Resumes normal operation. |

## Modules

### `contract.rs`
Main contract entry point. Contains all public functions and orchestrates calls to validation, storage, balance accounting, and event emission.

### `investment.rs`
Defines the `Investment` struct and its payment logic. Handles both ReverseLoan and Coupon payment calculations, including the final-round principal return for Coupon investments.

### `balance.rs`
Tracks all on-chain accounting via `ContractBalance`: reserve, project, commission, payment_obligations, collateral positions, and historical totals. All mutations go through dedicated `recalculate_from_*` methods.

### `amounts.rs`
Computes the split of an investment into project funds, reserve fund, and commission using a progressive commission rate based on investment size.

### `collateral.rs`
Defines the `Collateral` struct, the `ReflectorOracle` interface, and helpers for computing collateral coverage level and per-investor collateral entitlements.

### `data.rs`
Core configuration struct `ContractData` (derived from constructor parameters) and `InvestmentContractParams` used during initialization.

### `validation.rs`
Centralizes all guard logic and defines the `Error` enum. Key validators: `validate_constructor_params`, `validate_investment`, `validate_withdrawal`, `validate_company_transfer` (including the final Coupon round check), `validate_reserve_balance`.

### `storage.rs`
Typed read/write wrappers over Soroban's persistent storage for `ContractData`, `ContractBalance`, individual `Investment` records, `Collateral`, and the `next_payment_round` counter.

### `events.rs`
Emits structured contract events for all significant state transitions (investment received, payment sent, withdrawal, collateral deposited/liquidated/returned, goal reached, contract deployed).

### `constants.rs`
Defines `SECONDS_IN_DAY` used for timestamp arithmetic.

## Tests

```
tests/
├── common/mod.rs       # Shared helpers: create_investment_contract, create_token_contract,
│                       # do_payment_round, assert_contract_balance, ReflectorMock
├── success_tests.rs    # 14 tests covering happy-path flows
└── error_tests.rs      # 20 tests covering all error conditions
```

Run all 34 tests:

```bash
cargo test
```

Run a specific suite:

```bash
cargo test --test success_tests
cargo test --test error_tests
```

## Building

```bash
# Development build
cargo build

# Optimised WASM for deployment
stellar contract build
```

For testnet deployment refer to the [Stellar deployment guide](https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-to-testnet).

## License

Apache 2.0. See [LICENSE](LICENSE).
