#![allow(dead_code)]

pub use investment_income_based::{
    contract::{InvestmentContract, InvestmentContractClient},
    data::InvestmentContractParams,
    investment::Investment,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

pub mod reflector {
    use investment_income_based::collateral::{Asset, PriceData, ReflectorOracle};
    use soroban_sdk::{contract, contractimpl};

    use super::*;
    #[contract]
    pub struct ReflectorMock;
    #[contractimpl]
    impl ReflectorOracle for ReflectorMock {
        fn x_last_price(_env: &Env, _base_asset: Asset, _quote_asset: Asset) -> Option<PriceData>{
            Some(PriceData {
                price: 936_i128,
                timestamp: 65587445447_u64
            })
        }

        fn decimals(_env: &Env) -> u32 {
            3_u32
        }
    }
}

pub fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(e, &sac.address()),
        TokenAdminClient::new(e, &sac.address()),
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
    e: &Env,
    i_rate: u32,
    claim_block_days: u64,
    fundraising_days: u64,
    goal: i128,
    return_type: u32,
    return_months: u32,
    min_per_investment: i128,
    mock_auths: bool,
) -> TestData<'_> {
    if mock_auths {
        e.mock_all_auths();
    }
    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let project_address = Address::generate(&e);
    let reflector_id = e.register(reflector::ReflectorMock, ());
    let (token, token_admin) = create_token_contract(&e, &admin);
    let uri = String::from_str(&e, "https://example.com");
    let name = String::from_str(&e, "Test Token");
    let symbol = String::from_str(&e, "TT");

    let investment_params: InvestmentContractParams = InvestmentContractParams {
        i_rate,
        claim_block_days,
        fundraising_days,
        goal,
        return_type,
        return_months,
        min_per_investment,
    };

    let client = InvestmentContractClient::new(
        e,
        &e.register(
            InvestmentContract {},
            (
                admin.clone(),
                project_address.clone(),
                token.address.clone(),
                reflector_id,
                uri,
                name,
                symbol,
                investment_params,
            ),
        ),
    );

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
    received_so_far: i128,
    reserve_contributions: i128,
) {
    let b = test_data.client.get_contract_balance();
    assert_eq!(b.project, project);
    assert_eq!(b.reserve, reserve);
    assert_eq!(b.comission, comission);
    assert_eq!(b.payments, payments);
    assert_eq!(b.received_so_far, received_so_far);
    assert_eq!(b.reserve_contributions, reserve_contributions);
}

pub fn do_payment_round(
    e: &Env,
    test_data: &TestData,
    token_id: u32,
    transfer_amount: i128,
    expected_paid: i128,
    expected_transfers: u32,
    expected_completed: bool,
) -> Investment {
    e.ledger().set_timestamp(e.ledger().timestamp() + (30 * 86401));
    test_data.client.add_company_transfer(&transfer_amount);
    let inv = test_data.client.process_investor_payment(&token_id);
    assert_eq!(inv.paid, expected_paid);
    assert_eq!(inv.payments_transferred, expected_transfers);
    assert_eq!(inv.completed, expected_completed);
    
    inv
}


