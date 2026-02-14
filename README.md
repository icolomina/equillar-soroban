# The Equillar Soroban Contract

> [!WARNING]
> This contract is useful for learning and should not be used in production without professional auditing. Please refer to the license for more information.

## Structure

This repository uses the [recommended structure](https://developers.stellar.org/docs/build/smart-contracts/getting-started/hello-world#create-a-new-project) for a Soroban project:

```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- 
## Overview

The Equillar Investment Contract is a Soroban smart contract designed for managing investments on the Stellar network. It enables:

- **Capital contributions**: Investors can contribute funds toward a project's funding goal
- **Time-based Returns**: Investors receive periodic payments (monthly) over a defined period
- **Flexible Return Models**: Supports both Reverse Loan and Coupon return types
- **NFT Representation**: Each investment is represented as a Non-Fungible Token (NFT)
- **Automated Payment Management**: Tracks and processes investor payments with claim mechanisms
- **Admin Controls**: Owner-controlled operations for payment processing, fund management, and contract pausing

The contract uses [OpenZeppelin's Stellar libraries](https://docs.openzeppelin.com/stellar-contracts) for access control (Ownable), pausability, and NFT functionality.

## Core Functions

### Initialization

- **`__constructor`**: Initializes the contract with investment parameters (interest rate, funding goal, return type, minimum investment, etc.)

### Investment Functions

- **`invest`**: Allows users to invest funds. Mints an NFT token ID representing the investment and calculates returns based on the configured parameters
- **`claim`**: Allows investors to claim all their accumulated pending payments at once (self-service)

### Admin Functions (Owner Only)

- **`process_investor_payment`**: Processes a single monthly payment to an investor (admin-driven)
- **`single_withdrawn`**: Withdraws funds from the project balance to the project address
- **`add_company_transfer`**: Adds funds from the admin to the reserve balance for upcoming payments
- **`move_funds_to_the_reserve`**: Internally moves funds from project balance to reserve balance
- **`get_contract_balance`**: Retrieves current contract balance breakdown (project, reserve, commission)
- **`check_reserve_balance`**: Calculates additional funds needed for upcoming payments (within next week)

### Pausable Functions

- **`pause`**: Pauses the contract, preventing investments and payments
- **`unpause`**: Resumes contract operations

## Modules

### `contract.rs`
The main contract implementation containing all public functions and business logic for the `InvestmentContract`.

### `investment.rs`
Defines the `Investment` struct and related logic for individual investments, including:
- Investment creation
- Payment processing
- Return calculations
- Support for two return types: **Reverse Loan** (principal + interest distributed evenly) and **Coupon** (interest-only payments with final principal payment)

### `balance.rs`
Manages contract balance accounting with the `ContractBalance` struct:
- Tracks reserve balance (for investor payments)
- Tracks project balance (for company withdrawal)
- Tracks commission balance
- Implements progressive commission rates based on investment amount
- Provides balance recalculation methods for various operations
- Uses OpenZeppelin's `Wad` library for high-precision fixed-point arithmetic (18 decimals) to accurately calculate commission splits and reserve allocations without rounding errors

### `claim.rs`
Handles payment claim scheduling and calculations:
- `Claim` struct stores next payment timestamp and amount
- Calculates how many payment periods have elapsed since the last claim
- Determines when payments become available

### `data.rs`
Core data structures and configuration:
- `ContractData`: Stores contract-level configuration (interest rate, goal, return type, token address, etc.)
- `State` enum: Tracks contract state (Active, FundsReached)
- `InvestmentContractParams`: Constructor parameters

### `validation.rs`
Centralized validation logic and error definitions:
- Validates investment amounts, balances, and timing constraints
- Defines the `Error` enum with all possible contract errors
- Ensures business rules are enforced (e.g., minimum investment, payment timing, goal limits)

### `storage.rs`
Storage management layer providing read/write operations for:
- Contract data
- Individual investments
- Claims map
- Contract balances
- Uses Soroban's persistent storage primitives

### `constants.rs`
Defines time constants used throughout the contract:
- `SECONDS_IN_DAY`, `SECONDS_IN_WEEK`, `SECONDS_IN_MONTH`

### `lib.rs`
The crate root that exports the contract and serves as the entry point for the Soroban WebAssembly module.

## Tests

The test suite is organized into multiple files for better maintainability:

### Test Structure

```
tests/
├── common/
│   └── mod.rs           # Shared test utilities and helper functions
├── error_tests.rs       # Tests for error conditions and validation
└── success_tests.rs     # Tests for successful operations
```

### `common/mod.rs`
Contains shared test utilities used across all test files:
- **`create_investment_contract`**: Sets up a test environment with contract, token, and addresses
- **`create_token_contract`**: Creates a Stellar Asset Contract for testing
- **`TestData` struct**: Encapsulates all test context (addresses, clients, tokens)
- Helper functions for common test scenarios like minting tokens and making investments

### `error_tests.rs` (19 tests)
Tests that verify the contract properly handles error conditions:
- **Constructor validation errors**: Invalid parameters (zero interest rate, zero goal, invalid return type, etc.)
- **Investment errors**: Amount below minimum, insufficient balance, goal exceeded, contract paused
- **Payment processing errors**: Invalid token IDs, insufficient reserve, payment timing violations
- **Authorization errors**: Unauthorized pause/unpause, unauthorized withdrawals
- **Withdrawal errors**: Insufficient balances for various operations

Each test uses `#[should_panic]` to verify the contract panics with the expected error.

### `success_tests.rs` (18 tests)
Tests that verify successful contract operations:
- **Commission calculation**: Tests the progressive commission rate algorithm
- **Investment flows**: Both Reverse Loan and Coupon return types
- **Balance management**: Contract balance tracking, reserve calculations, fund movements
- **Payment processing**: Single and multiple payment claims
- **Pausable functionality**: Pause and unpause operations
- **Admin operations**: Withdrawals, company transfers, fund movements
- **Multi-investor scenarios**: Multiple investments from the same user, goal limits

## Building and Testing

### Prerequisites

Ensure you have the Stellar/Soroban development environment set up:
- Rust toolchain
- Soroban CLI
- Required dependencies from `Cargo.toml`

For detailed setup instructions, refer to the [Stellar documentation](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup).

### Running Tests

Run all tests (37 total):
```bash
cargo test
```

Run specific test files:
```bash
cargo test --test error_tests
cargo test --test success_tests
```

Run a specific test:
```bash
cargo test test_investment_reverse_loan
```

### Building the Contract

Build for development (unoptimized):
```bash
cargo build
```

And then generate the wasm file:
```bash
stellar contract build
```

Deploy to Stellar testnet using the Soroban CLI. Refer to the [official deployment guide](https://developers.stellar.org/docs/build/smart-contracts/getting-started/deploy-to-testnet) for detailed instructions.

