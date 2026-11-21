---
sidebar_position: 13
title: Testing and test coverage
---

# Testing and test coverage

The test suite lives in `contracts/investment-income-based/tests/` and is split into three types, each with a different protection goal. Running `cargo test` from the repo root executes all of them.

```text
tests/
├── common/mod.rs        # shared setup (contract deployment, mock oracle, token, helpers)
├── success_tests.rs     # 21 directed end-to-end flow tests
├── error_tests.rs       # 44 directed guard tests
└── fuzz_properties.rs   # 2 property-based fuzz tests (proptest)
```

## 1. `success_tests.rs` — exact lifecycle values (21 tests)

Full end-to-end flows with **exact, field-level assertions** of both the returned `Position` and the `Balance` snapshot after every step. They pin the truncation/rounding behavior bit by bit and cover:

- Coupon and ReverseLoan full lifecycles (every round, final capital return, completion)
- Payment rounds with liquidations **disabled** (accounting still updated without in-contract transfers)
- Multiple investors in the same payment round
- Company withdrawals, commission withdrawals, final sweeps etc.
- Collateral deposit/liquidation/return flows
- Pause/unpause, operator revocation blocking `invest`, grant flows
- The double-entry equation (`assets = liabilities + equity`) tested after every operation

The worked examples in this documentation ([Fundraising](./fundraising-and-positions.md#token-scale-example)) reproduce these exact values, so every figure you read here is executable truth.

## 2. `error_tests.rs` — every guard, one test per `callsite` (44 tests)

One test per business-error guard × entrypoint, asserting both the clean business error (no trap) and the expected panic code. Coverage:

- Constructor validation (interest rate, goal, months, minimum, cmr params, return type)
- `invest` guards (balance, minimum, window, goal reached, duplicate `token_id`, emergency)
- Payment round guards (no reserve, wrong round, enable/disable out of period)
- Refund guards (already refunded, fundraising ended, never invested)
- Withdrawal guards (no available commission, already withdrawn, ongoing fundraising)
- Collateral guards (not configured, empty, only one token)
- Emergency guards (already active, pending commission, blocked operations during emergency)
- Role guards (already holds role, role already exists, company/manager single-holder rule)

## 3. `fuzz_properties.rs` — property-based fuzzing of allocations (2 properties)

This suite **fuzzes the allocation math through the public `invest` entrypoint** using [proptest](https://proptest-rs.github.io/proptest/). It explores amounts from 1 raw unit up to `10^27` and interest rates from 1 to 10,000 basis points over randomized combinations of deployment parameters, and asserts, for every case:

**On success (`Ok`):**
- **P1** — `deposited = amount - commission`
- **P2** — commission belongs to the current tier: `0 ≤ commission ≤ floor(amount × i_rate / (denominator × 10_000))` with the tier resolved exactly as documented
- **P3** — `returns = floor(deposited × i_rate/10_000)` (tier-independent cap)
- **P4** — `total = deposited + returns`
- **P5** — `regular_payment` equals the type-correct schedule share (Coupon: `returns / return_months`; ReverseLoan: `total / return_months`)
- **INV-1** — the double-entry equation holds: `assets = liabilities + equity`
- clean rejection is a valid outcome: a clean `Err(TooLargeInvestment)` is asserted to be a **full no-op** (investor balance intact, contract balance unchanged, all Balance fields unchanged)

**Plus:**
- **P8** — same-tier commission monotonicity: for a fixed tier boundary, the commission never decreases when the amount increases within the tier (documents intended business semantics)

Anything that is **not** one of the two valid outcomes — a trap, an `InvokeError::Abort`, or any unexpected business error — fails the test.

### Run and scale

```bash
cd contracts/investment-income-based
cargo test --test fuzz_properties            # 128 randomized cases per property
PROPTEST_CASES=10000 cargo test --test fuzz_properties   # heavier local validation
```

Continuous integration runs `cargo test` with the default 128 cases per property on every push/PR to `main`.

### The regression-seed files

When a property fails, proptest persists a shrinking seed in `tests/*.proptest-regressions` and replays it automatically on later runs.

> **Important nuance**: seeds encode RNG state under the **current** strategy, so after a bug fix or a strategy reshaping they no longer reproduce their original counterexample; delete stale seed files deliberately (and note it in the commit). Never hand-edit a valid seed file.

## What the suite does not cover (yet)

Being explicit about the boundaries of the current protection:

- Directed tests exercise a fixed token configuration (7 decimals, mock SAC) and a mock oracle with hardcoded prices — cross-token or exotic-decimal divergences are only sampled by the fuzz suite in-wad.
- The fuzz suite drives `invest` end-to-end, so states reachable **after** acceptance (payment-round time boundaries, multi-round collateral interleavings) are covered by directed tests but not fuzzed.
- There is no ledger-churn / TTL-restoration simulation.

## Continuous integration

The CI pipeline (`.github/workflows/rust.yml`) runs `cargo fmt --check`, `cargo clippy`, `cargo build`, and the full `cargo test` (all three binaries, including the fuzz suite at its default case count).
