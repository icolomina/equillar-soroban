# Equillar — Income-Based Investment Contract

> [!WARNING]
> This contract has not been audited. Do not use in production without a professional security review. See [LICENSE](LICENSE) for details.

A Soroban (Stellar) smart contract implementing an **investment financing engine**: an integrator operates financing flows between companies raising funds and users investing through its platform. The contract provides role management, fundraising acceptance with a dynamic commission model, scheduled payment rounds, optional collateral with oracle-based liquidation, an emergency close gate, and double-entry accounting verified on every operation.

OpenZeppelin Stellar crates (`stellar-contract-utils`, `stellar-access`, `stellar-macros`) and the Soroban SDK are used as core dependencies.

## Documentation

Full documentation lives in this repository, available in two languages. Each index lists every section of the protocol lifecycle:

| | Index |
|---|---|
| 🇬🇧 **English** | [`docs/en/index.md`](docs/en/index.md) |
| 🇪🇸 **Español** | [`docs/es/index.md`](docs/es/index.md) |

Start with **Current status** and **The Balance model** — the double-entry accounting every other section builds on.

## Repository

- Repository: <https://github.com/icolomina/equillar-soroban>
- Main website: <https://equillar.com>
