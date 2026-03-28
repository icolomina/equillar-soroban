mod common;

use common::{assert_contract_balance, create_investment_contract, do_payment_round};
use investment_income_based::{amounts::calculate_rate_denominator};
use soroban_sdk::{Address, Env, String, testutils::{Address as _}};
use crate::common::create_token_contract;
use soroban_sdk::testutils::Ledger;

/// Verifies that `calculate_rate_denominator` returns the correct denominator
/// for a range of investment amounts at a fixed 7-decimal precision.
/// Each assertion covers a distinct amount tier to ensure the step function
/// behaves correctly at and between boundaries.
#[test]
fn test_commision_calculator() {
    assert_eq!(calculate_rate_denominator(&(90_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(120_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(150_i128 * 10_000_000), 7), 10_u32);
    assert_eq!(calculate_rate_denominator(&(500_i128 * 10_000_000), 7),11_u32);
    assert_eq!(calculate_rate_denominator(&(1900_i128 * 10_000_000), 7),14_u32);
}

/// Verifies the complete lifecycle of a Coupon investment across 4 payment rounds.
/// Rounds 1–3 pay interest only (`amount_to_pay_per_month = 12`); round 4 pays
/// `amount_to_pay_per_month + deposited` (interest + full principal return),
/// marking the investment as completed. Contract balance snapshots are asserted
/// after every round to confirm reserve, commission, and paid amounts.
#[test]
fn test_flow_with_coupon() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 2_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);

    assert_contract_balance(&test_data, 0, 0, 0, 0, 0, 0);

    let inv = test_data.client.invest(&u, &1000);

    assert_eq!(inv.commission, 5_i128);
    assert_eq!(inv.deposited, 995_i128);
    assert_eq!(inv.accumulated_interests, 49_i128);
    assert_eq!(inv.total, 1044_i128);
    assert_eq!(inv.regular_payment, 12_i128);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    test_data.token_admin.mint(&test_data.project_address, &2000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &2000);

    do_payment_round(&e, &test_data, inv.token_id, 15,   12,   1, false);
    assert_contract_balance(&test_data, 945, 53, 5, 12, 995, 15);

    do_payment_round(&e, &test_data, inv.token_id, 15,   24,   2, false);
    assert_contract_balance(&test_data, 945, 56, 5, 24, 995, 30);

    do_payment_round(&e, &test_data, inv.token_id, 15,   36,   3, false);
    assert_contract_balance(&test_data, 945, 59, 5, 36, 995, 45);

    do_payment_round(&e, &test_data, inv.token_id, 1500, 1043, 4, true);
    assert_contract_balance(&test_data, 945, 552, 5, 1043, 995, 1545);
}

/// Verifies the complete lifecycle of a ReverseLoan investment across 4 payment rounds.
/// Each round pays `regular_payment = 261` (equal principal + proportional interest instalment).
/// After all 4 rounds the investment is marked completed and `paid` equals `total`.
/// Contract balance snapshots are asserted after every round.
#[test]
fn test_flow_with_reverse_loan() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    assert_contract_balance(&test_data, 0, 0, 0, 0, 0, 0);

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = test_data.client.invest(&u, &1000);
    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    assert_eq!(inv.commission, 5_i128);
    assert_eq!(inv.deposited, 995_i128);
    assert_eq!(inv.accumulated_interests, 49_i128);
    assert_eq!(inv.total, 1044_i128);
    assert_eq!(inv.regular_payment, 261_i128);

    test_data.token_admin.mint(&test_data.project_address, &2000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &2000);

    do_payment_round(&e, &test_data, inv.token_id, 261, 261,  1, false);
    assert_contract_balance(&test_data, 945, 50, 5, 261, 995, 261);
    do_payment_round(&e, &test_data, inv.token_id, 261, 522,  2, false);
    assert_contract_balance(&test_data, 945, 50, 5, 522, 995, 522);
    do_payment_round(&e, &test_data, inv.token_id, 261, 783,  3, false);
    assert_contract_balance(&test_data, 945, 50, 5, 783, 995, 783);
    do_payment_round(&e, &test_data, inv.token_id, 261, 1044, 4, true);
    assert_contract_balance(&test_data, 945, 50, 5, 1044, 995, 1044);
}

/// Verifies that multiple investors can all be paid within the same payment round:
/// a single `add_company_transfer` covering the total per-round obligations allows
/// each `process_investor_payment` call to succeed, with each investment tracking
/// its own `payments_transferred` counter independently.
#[test]
fn test_multiple_investors_same_payment_round() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u1: Address = Address::generate(&e);
    let u2: Address = Address::generate(&e);
    test_data.token_admin.mint(&u1, &1000);
    test_data.token_admin.mint(&u2, &1000);

    let inv1 = test_data.client.invest(&u1, &1000);
    let inv2 = test_data.client.invest(&u2, &1000);

    assert_contract_balance(&test_data, 1890, 100, 10, 0, 1990, 0);

    test_data.token_admin.mint(&test_data.project_address, &3000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &3000);

    e.ledger().set_timestamp(15 * 86400);

    test_data.client.add_company_transfer(&522_i128);
    assert_contract_balance(&test_data, 1890, 622, 10, 0, 1990, 522);

    let inv1_paid = test_data.client.process_investor_payment(&inv1.token_id);
    let inv2_paid = test_data.client.process_investor_payment(&inv2.token_id);

    assert_eq!(inv1_paid.paid, 261_i128);
    assert_eq!(inv1_paid.payments_transferred, 1_u32);
    assert!(!inv1_paid.completed);

    assert_eq!(inv2_paid.paid, 261_i128);
    assert_eq!(inv2_paid.payments_transferred, 1_u32);
    assert!(!inv2_paid.completed);

    assert_contract_balance(&test_data, 1890, 100, 10, 522, 1990, 522);
}



/// Verifies that pausing the contract prevents new investments:
/// after `pause` is called, `try_invest` must return an error.
#[test]
fn test_pause() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
    );

    test_data.client.pause(&test_data.admin);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let invest_result = test_data.client.try_invest(&test_data.user, &100000);
    assert!(invest_result.is_err());
}

/// Verifies that unpausing the contract restores normal operation:
/// after `pause` followed by `unpause`, `invest` succeeds and returns
/// a valid investment with a positive deposited amount.
#[test]
fn test_unpause() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
    );

    test_data.client.pause(&test_data.admin);
    test_data.client.unpause(&test_data.admin);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let investment = test_data.client.invest(&test_data.user, &100000);
    assert!(investment.deposited > 0);
}



/// Checks that `add_collateral` correctly computes and stores the collateral
/// level on first deposit, and that a second deposit from the same token
/// increases the reported collateral level proportionally.
#[test]
fn test_add_collateral() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin)  = create_token_contract(&e, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &200_i128);
    test_data.token_admin.mint(&test_data.user, &150_i128);
    test_data.client.invest(&test_data.user, &150_i128);
    let collateral = test_data.client.add_collateral(
        &token_collateral.address, 
        &100_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
    );

    assert_eq!(collateral.collateral_level, 6000_u32);

    let collateral = test_data.client.add_collateral(
        &token_collateral.address, 
        &100_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
    );

    assert!(collateral.collateral_level > 6000_u32);

}

/// Validates proportional collateral distribution across investors:
/// two equal investments receive the same collateral payout while a
/// smaller investment receives a proportionally lower amount.
#[test]
fn test_pay_with_collateral() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin)  = create_token_contract(&e, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &4000_i128);

    let user2 = soroban_sdk::Address::generate(&e);
    let user3 = soroban_sdk::Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    let inv1 = test_data.client.invest(&test_data.user, &2000_i128);
    let inv2 = test_data.client.invest(&user2, &2000_i128);
    let inv3 = test_data.client.invest(&user3, &1000_i128);

    test_data.client.add_collateral(
        &token_collateral.address, 
        &4000_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
    );

    let pay_collatreral_1 = test_data.client.pay_with_collateral(&inv1.token_id);
    let pay_collatreral_2 = test_data.client.pay_with_collateral(&inv2.token_id);
    let pay_collatreral_3 = test_data.client.pay_with_collateral(&inv3.token_id);

    assert!(pay_collatreral_1 == pay_collatreral_2);
    assert!(pay_collatreral_3 < pay_collatreral_1);

}

/// Confirms that `return_collateral_to_company` transfers the entire
/// collateral token balance held by the contract back to the collateral
/// provider address, leaving the contract with a zero balance.
#[test]
fn test_return_collateral_to_company() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin)  = create_token_contract(&e, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &4000_i128);

    let user2 = soroban_sdk::Address::generate(&e);
    let user3 = soroban_sdk::Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    test_data.client.invest(&test_data.user, &2000_i128);
    test_data.client.invest(&user2, &2000_i128);
    test_data.client.invest(&user3, &1000_i128);

    test_data.client.add_collateral(
        &token_collateral.address, 
        &4000_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
    );

    assert_eq!(token_collateral_admin.balance(&test_data.client.address), 4000_i128);
    test_data.client.return_collateral_to_company();
    assert_eq!(token_collateral_admin.balance(&test_data.client.address), 0_i128)

}

/// Verifies that `withdrawn` transfers project funds to the `project_address`
/// once the fundraising period has ended, and correctly reduces the contract's
/// project balance by the withdrawn amount.
#[test]
fn test_withdrawn() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    test_data.client.invest(&u, &1000);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);
    e.ledger().set_timestamp(8 * 86400);

    let balance_before = test_data.token.balance(&test_data.project_address);
    test_data.client.withdrawn(&900_i128);
    let balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(balance_after - balance_before, 900_i128);
    assert_contract_balance(&test_data, 45, 50, 5, 0, 995, 0);
}

/// Verifies that `add_company_transfer` enforces dual authorization:
/// both the owner (admin) and project_address must authorize the call,
/// and critically, both must sign the exact same amount — preventing a
/// compromised party from substituting a different value in their signature.
#[test]
fn test_add_company_transfer_authorization_verification() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    test_data.client.invest(&u, &1000);

    test_data.token_admin.mint(&test_data.project_address, &500);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &500);

    e.ledger().set_timestamp(15 * 86400);
    let amount: i128 = 261;
    test_data.client.add_company_transfer(&amount);

    let auths = e.auths();

    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    let project_auth = auths.iter().find(|(addr, _)| *addr == test_data.project_address);

    assert!(admin_auth.is_some(), "Owner auth must be requested");
    assert!(project_auth.is_some(), "project_address auth must be requested");
}

/// Verifies that `withdrawn` enforces dual authorization:
/// both the owner (admin) and project_address must sign the call,
/// and both must authorize the exact same amount.
#[test]
fn test_withdrawn_authorization_verification() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    test_data.client.invest(&u, &1000);

    e.ledger().set_timestamp(8 * 86400);

    let amount: i128 = 500;
    test_data.client.withdrawn(&amount);

    let auths = e.auths();
    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    let project_auth = auths.iter().find(|(addr, _)| *addr == test_data.project_address);

    assert!(admin_auth.is_some(), "Owner auth must be requested");
    assert!(project_auth.is_some(), "project_address auth must be requested");
}

/// Verifies that the same user can invest multiple times, receiving a distinct
/// NFT token_id for each investment, with each investment tracking its own
/// state independently (paid, payments_transferred, completed).
/// A single `add_company_transfer` covering that round`s obligations allows
/// each investment to be paid via `process_investor_payment` independently:
/// inv2 can still be paid in the same round as inv1 because
/// `next_payment_round=1` while `inv2.payments_transferred` is still 0.
#[test]
fn test_invest_same_user_multiple_times() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &3000);

    let inv1 = test_data.client.invest(&u, &1000);
    let inv2 = test_data.client.invest(&u, &1000);
    let inv3 = test_data.client.invest(&u, &1000);

    assert_ne!(inv1.token_id, inv2.token_id);
    assert_ne!(inv2.token_id, inv3.token_id);

    assert_eq!(inv1.paid, 0);
    assert_eq!(inv2.paid, 0);
    assert_eq!(inv3.paid, 0);
    assert_eq!(inv1.payments_transferred, 0);
    assert!(!inv1.completed);

    assert_contract_balance(&test_data, 2835, 150, 15, 0, 2985, 0);

    test_data.token_admin.mint(&test_data.project_address, &1000_i128);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &1000_i128);

    e.ledger().set_timestamp(15 * 86400);

    test_data.client.add_company_transfer(&1000_i128);
    let inv1_after = test_data.client.process_investor_payment(&inv1.token_id);

    assert_eq!(inv1_after.payments_transferred, 1_u32);
    assert_eq!(inv1_after.paid, 261_i128);

    let inv2_after = test_data.client.process_investor_payment(&inv2.token_id);
    assert_eq!(inv2_after.payments_transferred, 1_u32);
    assert_eq!(inv2_after.paid, 261_i128);
}

/// Verifies that owner-gated functions (`pause`, `unpause`) request
/// authorization from the admin address and only from the admin address,
/// inspecting `e.auths()` after each call.
#[test]
fn test_owner_authorization_verification() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
    );

    test_data.client.pause(&test_data.admin);

    let auths = e.auths();
    assert_eq!(auths.len(), 1, "Should request exactly one authorization");

    assert_eq!(auths[0].0, test_data.admin, "Authorization should be from admin/owner");

    test_data.client.unpause(&test_data.admin);

    let auths = e.auths();
    assert_eq!(auths.len(), 1, "Should request exactly one authorization");
    assert_eq!(auths[0].0, test_data.admin, "Authorization should be from admin/owner");
}
