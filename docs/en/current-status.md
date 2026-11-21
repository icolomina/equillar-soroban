---
sidebar_position: 2
title: Current status
---

# Current status

Equillar should be understood as a **real technical foundation with visible gaps still to close** before production use.

## Status summary

- **Soroban contract**: implemented and tested
- **Audit**: not completed
- **Deployment target**: testnet only — functions are currently being tested there
- **Production readiness**: not ready

## What exists today

The current protocol already includes:

- Constructor-level configuration for financing terms, with full parameter validation at deployment
- Role management for `admin`, `operator`, `company`, and `manager`
- Investment intake and positions management, with checked (non-trapping) allocation arithmetic and a clean `TooLargeInvestment` rejection outside the representable range
- Double-entry accounting (`Balance`), verified after every accounting update
- Refunds during fundraising, with full commission reversal
- Payment rounds funded by the company's transfers, plus an out-of-contract payment mode
- Company and commission withdrawals
- Collateral support with oracle-based valuation, single-collateral-token enforcement, and proportional liquidation with a coverage cap
- Emergency close activation gate and a final full sweep (`withdrawn_all`)
- Pause and two-step admin transfer controls

## How it is tested

- `success_tests.rs` — 21 directed end-to-end lifecycle tests with exact per-field accounting assertions
- `error_tests.rs` — 44 tests, one per business-error guard
- `fuzz_properties.rs` — property-based fuzzing (proptest) of the allocation math through the public `invest` entrypoint, testing allocation invariants and the accounting equation over thousands of randomized cases

See [Testing and test coverage](./tests.md) for details.

## What is still missing for Equillar to be production-ready

At a minimum:

1. External security audit
2. Fixes from audit findings
3. Stronger operational runbooks
4. Clearer deployment and monitoring guidance
5. Production readiness validation beyond unit/integration-style and property-based tests

## Important framing

The protocol being **unaudited** is not something to hide. It is part of the current state and should be communicated explicitly to reviewers, contributors, and integrators.
