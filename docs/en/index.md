---
sidebar_position: 1
title: Introduction
---

# What is Equillar?

Equillar is an **investment lifecycle engine** implemented as a modular Soroban smart contract on Stellar.

Its purpose is to help an integrator operate financing flows between:

- **Integrators**: fintechs, exchanges, or platform operators that integrate Equillar into their services
- **Companies**: companies raising financing for a specific project
- **Users**: users who invest through the integrator platform

## What Equillar does

At the contract level, Equillar provides:

- Role-based access control (`admin`, `operator`, `company`, `manager`)
- Capital contributions through `invest`, with a dynamic (size-dependent) commission model
- Positions management identified by an identifier (`position_id`) provided by the integrator
- Double-entry accounting (`Balance`), verified (assets = liabilities + equity) after every operation
- Scheduled payment rounds, with in-contract transfers or an out-of-contract payment mode
- Optional collateral handling with oracle-based valuation and proportional liquidation
- Emergency close and a final full sweep (`withdrawn_all`)
- Pause control and a two-step admin role transfer

## What Equillar does not do

Equillar does **not** provide:

- Regulatory status
- Compliance workflows
- KYC / AML onboarding
- Legal structuring for offerings
- Consumer-facing product UX by itself

Those responsibilities remain with the **integrator** and any off-chain systems it operates.

## Why the engine is role-based

Equillar is **not** a **fully permissionless** primitive: it is designed to operate in managed environments where the integrator is expected to control critical actions such as:

- Role assignment (admin)
- Capital inflows managed through approved operators
- Company fund transfers for payment rounds
- Collateral deposits
- Emergency activation
- Withdrawals, payments, and the final sweep

This matches the current product vision: **Equillar provides the underlying technology layer for integrators who are already regulated.**

## Table of contents

The documentation follows the protocol lifecycle.

| # | Section | What it covers |
|---|---|---|
| 1 | [Current status](./current-status.md) | Implementation state, what is tested, what is missing for production |
| 2 | [Deploying the contract](./deploying-the-contract.md) | Build, identities, upload, deploy, constructor parameters, lifecycle timeline |
| 3 | [The Balance model](./balance-model.md) | Double-entry accounting: the equation, every field, how each operation moves them |
| 4 | [Role setup and revocation](./roles.md) | The four roles, grant/revoke flows, single-holder rule |
| 5 | [Fundraising and positions](./fundraising-and-positions.md) | `invest`, the commission tier model, allocation examples, position states |
| 6 | [Refunds](./refunds.md) | `refund_investor` during the `fundraising` window, full commission reversal |
| 7 | [Payment rounds](./payment-rounds.md) | Company transfers, per-position payments, no-transfer mode, payment types: coupon/reverse-loan |
| 8 | [Withdrawals](./withdrawals.md) | Fund withdrawals, commission withdrawals, the final `withdrawn_all` sweep |
| 9 | [Collateral](./collateral.md) | Deposits, oracle-based valuation, proportional liquidation with a coverage cap |
| 10 | [Emergency close](./emergency-close.md) | The terminal activation gate and how it combines with `withdrawn_all` |
| 11 | [Pause and resume controls](./admin-controls.md) | The Pausable safety switch, per-function guards |
| 12 | [Testing and test coverage](./tests.md) | The three test binaries, property-based fuzzing, what is and is not covered |
| — | [Error codes appendix](./error-codes.md) | The business error codes and when each one fires |

Two sections are especially important as background for the rest: [Current status](./current-status.md) and [The Balance model](./balance-model.md) — the double-entry accounting that every later section references step by step.
