// Each test binary compiles this module; helpers used by only one of them
// would otherwise trigger dead-code warnings in the others.
#![allow(dead_code)]

use investment_income_based::contract::InvestmentContractClient;
pub use investment_income_based::{
    contract::InvestmentContract, shared::types::InvestmentContractParams, shared::types::Position,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use stellar_access::access_control;
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

pub mod reflector {
    use investment_income_based::shared::oracle::{Asset, PriceData, ReflectorOracle};
    use soroban_sdk::{contract, contractimpl, contracttype, Symbol};

    use super::*;

    #[contract]
    pub struct ReflectorMock;

    #[contracttype]
    enum DataKey {
        ContractToken,
        /// Per-asset price overrides (kept for future price-manipulation
        /// suites); absent keys fall back to the hardcoded defaults below.
        Price(Asset),
    }

    #[contractimpl]
    impl ReflectorMock {
        pub fn __constructor(env: Env, contract_token: Address) {
            env.storage()
                .instance()
                .set(&DataKey::ContractToken, &contract_token);
        }
    }

    #[contractimpl]
    impl ReflectorOracle for ReflectorMock {
        fn lastprice(env: &Env, asset: Asset) -> Option<PriceData> {
            // Overrides take precedence over the hardcoded defaults.
            if let Some(price_data) = env.storage().instance().get(&DataKey::Price(asset.clone())) {
                return Some(price_data);
            }

            let contract_token: Option<Address> =
                env.storage().instance().get(&DataKey::ContractToken);

            match asset {
                Asset::Stellar(addr) if contract_token.as_ref() == Some(&addr) => Some(PriceData {
                    price: 1_i128,
                    timestamp: 65_587_445_447_u64,
                }),
                Asset::Stellar(_) => Some(PriceData {
                    price: 60000_i128,
                    timestamp: 65_587_445_447_u64,
                }),
                Asset::Other(symbol) if symbol == Symbol::new(env, "BTC") => Some(PriceData {
                    price: 60000_i128,
                    timestamp: 65_587_445_447_u64,
                }),
                Asset::Other(_) => Some(PriceData {
                    price: 1_i128,
                    timestamp: 65_587_445_447_u64,
                }),
            }
        }

        fn decimals(_env: &Env) -> u32 {
            8_u32
        }

        fn base(env: &Env) -> Asset {
            Asset::Other(Symbol::new(env, "XXX"))
        }
    }
}

pub fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(env, &sac.address()),
        TokenAdminClient::new(env, &sac.address()),
    )
}

pub struct TestData<'a> {
    pub user: Address,
    pub project_address: Address,
    pub admin: Address,
    pub client: InvestmentContractClient<'a>,
    pub token: TokenClient<'a>,
    pub token_admin: TokenAdminClient<'a>,
}

/// Test constructor mirroring `InvestmentContractParams`; the argument list
/// matches the deployment parameters one by one on purpose.
#[allow(clippy::too_many_arguments)]
pub fn create_investment_contract(
    env: &Env,
    i_rate: u32,
    claim_block_days: u64,
    fundraising_days: u64,
    goal: i128,
    return_type: u32,
    return_months: u32,
    min_per_investment: i128,
    mock_auths: bool,
    cmr_upper_divisor: u32,
    cmr_lower_divisor: u32,
    cmr_reductor: i128,
) -> TestData<'_> {
    if mock_auths {
        env.mock_all_auths_allowing_non_root_auth();
    }

    let admin = Address::generate(env);
    let user = Address::generate(env);
    let project_address = Address::generate(env);
    let (token, token_admin) = create_token_contract(env, &admin);
    let reflector_id = env.register(reflector::ReflectorMock, (token.address.clone(),));
    let investment_params = InvestmentContractParams {
        i_rate,
        claim_block_days,
        fundraising_days,
        goal,
        return_type,
        return_months,
        min_per_investment,
        cmr_upper_divisor,
        cmr_lower_divisor,
        cmr_reductor,
    };

    let client = InvestmentContractClient::new(
        env,
        &env.register(
            InvestmentContract {},
            (
                admin.clone(),
                token.address.clone(),
                reflector_id.clone(),
                investment_params,
            ),
        ),
    );

    client.grant_company(&project_address);
    client.grant_manager(&project_address);

    TestData {
        user,
        project_address,
        admin,
        client,
        token,
        token_admin,
    }
}

/// Expected values for a `Balance` snapshot. Defaults to all zeros.
#[derive(Default, Debug, PartialEq)]
pub struct BalanceExpectation {
    pub payment_rigths: i128,
    pub contract_earnings: i128,
    pub managed_volume: i128,
    pub managed_collateral: i128,
    pub payment_obligations: i128,
    pub return_collateral_obligations: i128,
    pub contract_commissions: i128,
}

/// Asserts the on-chain `Balance` snapshot field by field and verifies the
/// accounting equation `assets == liabilities + equity` holds.
pub fn assert_balance(test_data: &TestData, expected: BalanceExpectation) {
    let balance = test_data.client.get_contract_balance(&test_data.admin);

    assert_eq!(
        balance.assets.payment_rigths, expected.payment_rigths,
        "assets.payment_rigths"
    );
    assert_eq!(
        balance.assets.contract_earnings, expected.contract_earnings,
        "assets.contract_earnings"
    );
    assert_eq!(
        balance.assets.managed_volume, expected.managed_volume,
        "assets.managed_volume"
    );
    assert_eq!(
        balance.assets.managed_collateral, expected.managed_collateral,
        "assets.managed_collateral"
    );
    assert_eq!(
        balance.liabilities.payment_obligations, expected.payment_obligations,
        "liabilities.payment_obligations"
    );
    assert_eq!(
        balance.liabilities.return_collateral_obligations, expected.return_collateral_obligations,
        "liabilities.return_collateral_obligations"
    );
    assert_eq!(
        balance.equity.contract_commissions, expected.contract_commissions,
        "equity.contract_commissions"
    );

    assert_balance_equation(test_data);
}

/// Asserts only that the accounting equation holds for the current state:
/// `assets == liabilities + equity`.
pub fn assert_balance_equation(test_data: &TestData) {
    let balance = test_data.client.get_contract_balance(&test_data.admin);

    let total_assets = balance.assets.contract_earnings
        + balance.assets.managed_volume
        + balance.assets.payment_rigths
        + balance.assets.managed_collateral;
    let total_liabilities =
        balance.liabilities.payment_obligations + balance.liabilities.return_collateral_obligations;

    assert_eq!(
        total_assets,
        total_liabilities + balance.equity.contract_commissions,
        "accounting equation must hold: assets == liabilities + equity"
    );
}

pub fn do_payment_round(
    env: &Env,
    test_data: &TestData,
    token_id: u32,
    transfer_amount: i128,
    expected_paid: i128,
    expected_transfers: u32,
    expected_completed: bool,
) -> Position {
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + (30 * 86_401));
    test_data
        .client
        .add_company_transfer(&transfer_amount, &test_data.project_address);
    let position = test_data.client.process_investor_payment(&token_id);
    assert_eq!(position.paid, expected_paid);
    assert_eq!(position.payments_transferred, expected_transfers);
    assert_eq!(position.completed, expected_completed);

    position
}

pub fn do_payment_round_without_company_transfer(
    env: &Env,
    test_data: &TestData,
    token_id: u32,
    expected_paid: i128,
    expected_transfers: u32,
    expected_completed: bool,
) -> Position {
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + (30 * 86_401));
    let position = test_data.client.process_investor_payment(&token_id);
    assert_eq!(position.paid, expected_paid);
    assert_eq!(position.payments_transferred, expected_transfers);
    assert_eq!(position.completed, expected_completed);

    position
}

/// Grants the `operator` role to `addr` unless it already holds it
/// (re-granting the same address fails with `AddressAlreadyHoldsRole`).
pub fn ensure_operator(test_data: &TestData, addr: &Address) {
    let env = &test_data.client.env;
    let operator_role = symbol_short!("operator");

    // Storage reads must run inside the contract context.
    let already_operator = env.as_contract(&test_data.client.address, || {
        access_control::has_role(env, addr, &operator_role).is_some()
    });

    if !already_operator {
        test_data.client.grant_operator(addr);
    }
}

/// Invests as `addr`, granting it the `operator` role first unless it already
/// holds it (re-granting the same address fails with `AddressAlreadyHoldsRole`).
pub fn invest_as_operator(
    test_data: &TestData,
    addr: &Address,
    amount: &i128,
    token_id: &u32,
) -> Position {
    ensure_operator(test_data, addr);
    test_data.client.invest(addr, amount, token_id)
}

/// Grants the single-holder `company` role to `addr`, revoking the current
/// holder first when the slot is already taken.
pub fn grant_company_exclusive(test_data: &TestData, addr: &Address) {
    let env = &test_data.client.env;
    let company_role = symbol_short!("company");

    // Storage reads must run inside the contract context.
    let current_holder = env.as_contract(&test_data.client.address, || {
        if access_control::get_role_member_count(env, &company_role) > 0 {
            Some(access_control::get_role_member(env, &company_role, 0))
        } else {
            None
        }
    });

    if let Some(holder) = current_holder {
        test_data.client.revoke_company(&holder);
    }
    test_data.client.grant_company(addr);
}
