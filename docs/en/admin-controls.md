---
sidebar_position: 12
title: Pause and resume controls
---

# Pause and resume controls

## Introduction

Equillar uses the OpenZeppelin `Pausable` extension to provide a general-purpose safety switch. When the contract is paused, virtually all state-changing functions are blocked, protecting the contract while an incident (a suspected bug, unexpected behavior, etc.) is investigated.

Both `pause` and `unpause` can only be executed by the administrator.

### Guard behavior by function

| Function | Access (when not paused) | Behavior when paused |
|---|---|---|
| `invest` | `operator` (investing address) | Blocked |
| `enable_investment_liquidations` | admin | Blocked |
| `disable_investment_liquidations` | admin | Blocked |
| `check_investment_liquidations` | admin or any role | **Allowed** (read-only) |
| `process_investor_payment` | admin | Blocked |
| `refund_investor` | admin | Blocked |
| `get_contract_balance` | admin or any role | **Allowed** (read-only) |
| `grant_operator` / `revoke_operator` | admin | Blocked |
| `grant_company` / `revoke_company` | admin | Blocked |
| `grant_manager` / `revoke_manager` | admin | Blocked |
| `transfer_admin_role` / `accept_admin_transfer_role` | admin | Blocked |
| `withdrawn` | admin, `-to` holds `company` | Blocked |
| `withdrawn_commissions` | admin, `-to` holds `manager` | Blocked |
| `withdrawn_all` | admin, `-to` holds `manager` | Blocked |
| `withdrawn_all_collateral` | admin, `-to` holds `manager` | Blocked |
| `add_company_transfer` | `-from` holds `company` | Blocked |
| `activate_emergency_close` | admin | Blocked |
| `add_collateral` | `collateral_addr` holds `company` | Blocked |
| `pay_with_collateral` | admin | Blocked |
| `return_collateral_to_company` | admin | Blocked |
| `paused` (read) | anyone | **Allowed** (read-only) |
| `upgrade` | admin | **Only while paused** |

As a general rule, every function that moves funds, changes roles, or alters the contract's configuration is blocked while paused. The only exceptions are the read-only functions (`get_contract_balance`, `check_investment_liquidations`, `paused`), which remain available so the contract's state can still be inspected during an incident, and `upgrade`, which can **only** be called while the contract is paused — ensuring the contract's code cannot change mid-transaction or while other operations might still be in flight.

## Why nothing fund-moving is exempted

Unlike [emergency close](./emergency-close.md), a terminal gate scoped to pending settlement obligations, `pause` is meant to be a full stop covering *every* fund-moving operation, including investor-facing ones like `refund_investor`. This is intentional: if the reason for pausing is a suspected issue with the contract's logic or accounting, allowing any fund-moving function to execute would undermine the safety guarantee the pause is meant to provide. Once the contract is confirmed safe, the admin can resume normal operation, and any pending refunds, payments, or other operations can proceed as usual.

## Pause ↔ emergency interplay

Pause and emergency close are separate mechanisms with separate scopes:

- **Pause** is administrative and reversible (`unpause`), covers everything including the emergency gate itself, and is the tool for suspected contract-level incidents.
- **Emergency close** is business terminal: once activated it cannot be deactivated, and it freezes capital flows regardless of pause state.

While paused, `activate_emergency_close` is blocked; with the emergency active, all capital-moving entrypoints (including those under pause protection) reject with `OperationNotAllowedInEmergency` (50).
