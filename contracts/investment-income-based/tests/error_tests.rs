mod common;

use common::{create_investment_contract, invest_as_operator};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

use crate::common::create_token_contract;

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_interest_rate_zero() {
    let env = Env::default();
    create_investment_contract(&env, 0_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_goal_zero() {
    let env = Env::default();
    create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 0_i128, 1_u32, 4_u32, 100_i128, true);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_invalid_return_type() {
    let env = Env::default();
    create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 0_u32, 4_u32, 100_i128, true);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_return_months_zero() {
    let env = Env::default();
    create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 0_u32, 100_i128, true);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_min_investment_zero() {
    let env = Env::default();
    create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 0_i128, true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #43)")]
fn test_call_add_company_transfer_before_ts_next_payment() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 90_000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1_000_000);
    invest_as_operator(&test_data, &test_data.user, &1000);

    test_data.token_admin.mint(&test_data.project_address, &2000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &2000);

    env.ledger().set_timestamp(env.ledger().timestamp() + (12 * 86_400));
    test_data.client.add_company_transfer(&1000_i128, &test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #42)")]
fn test_call_process_investor_payment_without_previous_company_transfer() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 90_000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1_000_000);
    let inv = invest_as_operator(&test_data, &test_data.user, &1000);

    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 86_400));
    test_data.client.process_investor_payment(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_goal_reached() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 90_000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1_000_000);
    invest_as_operator(&test_data, &test_data.user, &89_000);
    invest_as_operator(&test_data, &test_data.user, &2200);
    invest_as_operator(&test_data, &test_data.user, &1600);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_invest_insufficient_balance() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &50_000);
    invest_as_operator(&test_data, &test_data.user, &100_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_invest_amount_less_than_minimum() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1_000_000);
    invest_as_operator(&test_data, &test_data.user, &50);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #40)")]
fn test_invest_after_fundraising_period() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.token_admin.mint(&test_data.user, &1_000_000);
    invest_as_operator(&test_data, &test_data.user, &1000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_add_company_transfer_insufficient_reserve_last_coupon_round() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 2_u32, 2_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = invest_as_operator(&test_data, &test_data.user, &1000);

    test_data.token_admin.mint(&test_data.project_address, &10);

    env.ledger().set_timestamp(15 * 86_400);

    test_data.client.add_company_transfer(&1_i128, &test_data.project_address);
    test_data.client.process_investor_payment(&inv.token_id);

    test_data.client.add_company_transfer(&1_i128, &test_data.project_address);
    test_data.client.process_investor_payment(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #38)")]
fn test_process_investor_payment_already_completed() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 1_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = invest_as_operator(&test_data, &test_data.user, &1000);

    test_data.token_admin.mint(&test_data.project_address, &2000);

    env.ledger().set_timestamp(15 * 86_400);
    test_data.client.add_company_transfer(&2000_i128, &test_data.project_address);
    test_data.client.process_investor_payment(&inv.token_id);
    test_data.client.process_investor_payment(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #39)")]
fn test_withdrawn_while_fundraising_ongoing() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000);
    invest_as_operator(&test_data, &test_data.user, &1000);

    env.ledger().set_timestamp(3 * 86_400);
    test_data.client.withdrawn(&500_i128, &test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_withdrawn_insufficient_project_balance() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000);
    invest_as_operator(&test_data, &test_data.user, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn(&946_i128, &test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #47)")]
fn test_add_company_transfer_insufficient_balance() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    env.ledger().set_timestamp(15 * 86_400);
    test_data.client.add_company_transfer(&100_000_i128, &test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_add_collateral_insufficient_balance() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let collateral_addr = Address::generate(&env);
    let (token_collateral, _token_collateral_admin) = create_token_contract(&env, &test_data.admin);
    test_data.client.grant_company(&collateral_addr);

    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&env, "TEST"),
        &collateral_addr,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2000)")]
fn test_add_collateral_without_company_role() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let collateral_addr = Address::generate(&env);
    let (token_collateral, token_collateral_admin) = create_token_contract(&env, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &200_i128);

    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&env, "TEST"),
        &collateral_addr,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_add_collateral_only_one_collateral_token_allowed() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let collateral_addr = Address::generate(&env);
    let (token_collateral, token_collateral_admin) = create_token_contract(&env, &test_data.admin);
    let (token_collateral_2, token_collateral_admin_2) = create_token_contract(&env, &test_data.admin);
    test_data.client.grant_company(&collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &200_i128);
    token_collateral_admin_2.mint(&collateral_addr, &200_i128);
    test_data.token_admin.mint(&test_data.user, &150_i128);
    invest_as_operator(&test_data, &test_data.user, &150_i128);
    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&env, "TEST"),
        &collateral_addr,
    );

    test_data.client.add_collateral(
        &token_collateral_2.address,
        &100_i128,
        &String::from_str(&env, "TEST2"),
        &collateral_addr,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #36)")]
fn test_pay_with_collateral_not_configured() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = invest_as_operator(&test_data, &test_data.user, &1000);

    test_data.client.pay_with_collateral(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #37)")]
fn test_return_collateral_when_empty() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let collateral_addr = Address::generate(&env);
    let (token_collateral, token_collateral_admin) = create_token_contract(&env, &test_data.admin);
    test_data.client.grant_company(&collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &200_i128);

    test_data.token_admin.mint(&test_data.user, &150_i128);
    let inv = invest_as_operator(&test_data, &test_data.user, &150_i128);

    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&env, "TEST"),
        &collateral_addr,
    );

    test_data.client.pay_with_collateral(&inv.token_id);

    test_data.client.return_collateral_to_company();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_refund_investor_address_has_not_invested() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    test_data.client.refund_investor(&999_u32);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #40)")]
fn test_refund_investor_fundraising_ended() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    let inv = invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(8 * 86_400);

    test_data.client.refund_investor(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #38)")]
fn test_refund_investor_already_refunded() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    let inv = invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(3 * 86_400);

    test_data.client.refund_investor(&inv.token_id);
    test_data.client.refund_investor(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #39)")]
fn test_withdrawn_commissions_while_fundraising_ongoing() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(3 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_withdrawn_commissions_no_available_commission() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_withdrawn_commissions_already_fully_withdrawn() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.withdrawn_commissions(&test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_emergency_pay_investor_address_has_not_invested() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.emergency_pay_investor(&999_u32);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #39)")]
fn test_emergency_pay_investor_fundraising_ongoing() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    let inv = invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(3 * 86_400);
    test_data.client.emergency_pay_investor(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #38)")]
fn test_emergency_pay_investor_already_completed() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    let inv = invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();
    test_data.client.emergency_pay_investor(&inv.token_id);
    test_data.client.emergency_pay_investor(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #49)")]
fn test_emergency_pay_investor_without_activation() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 2_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    let inv = invest_as_operator(&test_data, &user, &1000);
    env.ledger().set_timestamp(8 * 86_400);

    test_data.client.emergency_pay_investor(&inv.token_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #51)")]
fn test_activate_emergency_close_with_pending_commission() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    invest_as_operator(&test_data, &user, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.activate_emergency_close();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #52)")]
fn test_activate_emergency_close_with_empty_pool() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.activate_emergency_close();
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #50)")]
fn test_add_company_transfer_blocked_in_emergency() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let user = Address::generate(&env);
    test_data.token_admin.mint(&user, &1000);
    invest_as_operator(&test_data, &user, &1000);
    test_data.token_admin.mint(&test_data.admin, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();
    test_data.client.add_company_transfer(&100_i128, &test_data.project_address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #50)")]
fn test_invest_blocked_in_emergency() {
    let env = Env::default();
    let test_data = create_investment_contract(&env, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true);

    let first_investor = Address::generate(&env);
    let late_investor = Address::generate(&env);
    test_data.token_admin.mint(&first_investor, &1000);
    test_data.token_admin.mint(&late_investor, &1000);

    invest_as_operator(&test_data, &first_investor, &1000);

    env.ledger().set_timestamp(8 * 86_400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();

    invest_as_operator(&test_data, &late_investor, &1000);
}
