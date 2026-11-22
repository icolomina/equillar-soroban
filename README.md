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
| 🇪🇸 **Spanish** | [`docs/es/index.md`](docs/es/index.md) |

Start with **Current status** and **The Balance model** — the double-entry accounting every other section builds on.

**About the docs:** These docs were drafted with the assistance of AI models and verified line-by-line against the contract code; every numeric example is executable truth backed by the test suite. They may still contain errors, and both the docs and the code are under continuous improvement — issues and corrections are welcome.

## Contributing

Contributions are welcome. You can help in many ways beyond writing code:

- **Docs**: fix typos or errors, clarify explanations, or translate missing sections (English and Spanish).
- **Testing**: extend the directed or fuzz test suites, or report behavior that looks wrong.
- **Code**: bug fixes, hardening, and new features — open an issue first so we can discuss the approach.

Open an issue describing your idea, or fork and open a pull request. For anything touching payment, refund, or collateral math, explain your reasoning in the PR description.

Before running or testing the project locally, see the [docs](#documentation): the repo conventions (commands, test layout, gotchas) are described in [`AGENTS.md`](AGENTS.md).
