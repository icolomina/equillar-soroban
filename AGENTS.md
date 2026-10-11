# AGENTS.md

Soroban (Stellar) smart contract repo, single crate in a Cargo workspace:
`contracts/investment-income-based`. The README is accurate and detailed — it
documents the module layout, public contract API, and constructor params.
Don't duplicate it; consult it before changing entrypoints or params.

## Commands

Run from the repo root unless noted:

```bash
cargo test                          # all tests (works with existing local Cargo.lock)
cargo test -q --no-run              # compile tests only, fast check
cargo test -p investment-income-based --test success_tests <test_name>   # single test
cargo fmt --all                     # CI enforces --check; run before committing
cargo clippy                        # CI runs it without -D warnings
```

Tests live in two binaries: `tests/success_tests.rs` and `tests/error_tests.rs`.
Use `--test success_tests` / `--test error_tests` to scope a run.

WASM build (run inside the contract dir):

```bash
cd contracts/investment-income-based
stellar contract build
```

## Gotchas

- **WASM build artifacts land in the workspace root `target/`, not in the
  contract's own `target/`** (`contracts/*` are workspace members). The final
  wasm is `target/wasm32v1-none/release/investment_income_based.wasm` from the
  repo root (`../../target/...` from the contract dir).
- **`stellar-contract-utils`, `stellar-access`, `stellar-macros` are git
  dependencies** pinned to a commit of `OpenZeppelin/stellar-contracts` (not
  crates.io), because the crates.io releases don't support soroban-sdk 28 yet.
  The workspace `soroban-sdk` must match the version those crates use (28.x).
  When OpenZeppelin publishes a release for soroban-sdk 28+, switch back to
  crates.io versions and bump the `rev` only deliberately.
- **`Cargo.lock` is gitignored** and resolved fresh in CI. The former
  `ed25519-dalek` pin workaround is no longer needed.
- **`test_snapshots/*.json` are auto-generated** by every `cargo test` run
  (deterministic, gitignored). Never hand-edit them; diffs there are noise.

## Testing conventions

`tests/common/mod.rs` holds all shared setup. `create_investment_contract(...)`
registers a mock SAC token and `ReflectorMock` (an in-test oracle with hardcoded
prices: contract token = 1, other assets = 60000). Payment rounds advance ledger
time by `30 * 86_401` seconds (~30 days) per round — copy this pattern in new
tests. Doctests are disabled (`doctest = false` in the crate manifest).

## Contract conventions

- `require!(cond, Error::...)` macro (defined in `src/lib.rs`) is the standard
  validation guard; the canonical `Error` enum lives in `src/validation/mod.rs`.
- `src/contract.rs` holds the public Soroban entrypoints (auth checks, pause
  guards, role management); logic lives in the domain modules
  (`investment/`, `payments/`, `treasury/`, `emergency/`, `collateral/`, `shared/`).
  Keep entrypoints thin and delegate.
- Contract is **unaudited** and manages real funds; be extra careful with
  payment, refund, and collateral math changes.

## Local reference

`.github/skill/` (gitignored, machine-local) contains a full Stellar/Soroban
development playbook — consult it for Stellar CLI, SDK, and ecosystem
questions.
