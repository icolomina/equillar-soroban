#![allow(dead_code)]

use investment_income_based::contract::InvestmentContractClient;
pub use investment_income_based::{
    contract::InvestmentContract, shared::types::InvestmentContractParams, shared::types::Position,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
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
                reflector_id,
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

pub fn assert_contract_balance(
    test_data: &TestData,
    project: i128,
    reserve: i128,
    comission: i128,
    payments: i128,
) {
    let balance = test_data.client.get_contract_balance(&test_data.admin);
    assert_eq!(balance.project, project);
    assert_eq!(balance.reserve, reserve);
    assert_eq!(balance.comission, comission);
    assert_eq!(balance.payments, payments);
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

pub fn invest_as_operator(
    test_data: &TestData,
    addr: &Address,
    amount: &i128,
    token_id: &u32,
) -> Position {
    test_data.client.grant_operator(addr);
    test_data.client.invest(addr, amount, token_id)
}
