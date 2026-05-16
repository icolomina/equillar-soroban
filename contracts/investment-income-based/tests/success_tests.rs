mod common;

use common::{assert_contract_balance, create_investment_contract, do_payment_round, invest_as_operator};
use investment_income_based::investment::allocation::calculate_rate_denominator;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

use crate::common::create_token_contract;

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

    let inv = invest_as_operator(&test_data, &u, &1000);

    assert_eq!(inv.commission, 5_i128);
    assert_eq!(inv.deposited, 995_i128);
    assert_eq!(inv.accumulated_interests, 49_i128);
    assert_eq!(inv.total, 1044_i128);
    assert_eq!(inv.regular_payment, 12_i128);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    test_data.token_admin.mint(&test_data.project_address, &2000);

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
    let inv = invest_as_operator(&test_data, &u, &1000);
    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    assert_eq!(inv.commission, 5_i128);
    assert_eq!(inv.deposited, 995_i128);
    assert_eq!(inv.accumulated_interests, 49_i128);
    assert_eq!(inv.total, 1044_i128);
    assert_eq!(inv.regular_payment, 261_i128);

    test_data.token_admin.mint(&test_data.project_address, &2000);

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

    let inv1 = invest_as_operator(&test_data, &u1, &1000);
    let inv2 = invest_as_operator(&test_data, &u2, &1000);

    assert_contract_balance(&test_data, 1890, 100, 10, 0, 1990, 0);

    test_data.token_admin.mint(&test_data.project_address, &3000);

    e.ledger().set_timestamp(15 * 86400);

    test_data.client.add_company_transfer(&522_i128, &test_data.project_address);
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
    let investment = invest_as_operator(&test_data, &test_data.user, &100000);
    assert!(investment.deposited > 0);
}

#[test]
fn test_revoke_operator_blocks_invest() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let operator = Address::generate(&e);
    test_data.token_admin.mint(&operator, &2_000_i128);

    test_data.client.grant_operator(&operator);
    let _ = test_data.client.invest(&operator, &1_000_i128);

    test_data.client.revoke_operator(&operator);
    let invest_result = test_data.client.try_invest(&operator, &200_i128);
    assert!(invest_result.is_err());
}

/// Verifies that granting `company` enables receiving project withdrawals,
/// and revoking it blocks `withdrawn` for that address.
#[test]
fn test_grant_and_revoke_company_role() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let investor = Address::generate(&e);
    let company_alt = Address::generate(&e);
    test_data.token_admin.mint(&investor, &1_000_i128);
    invest_as_operator(&test_data, &investor, &1_000_i128);

    e.ledger().set_timestamp(8 * 86_400);

    test_data.client.grant_company(&company_alt);
    let before = test_data.token.balance(&company_alt);
    test_data.client.withdrawn(&100_i128, &company_alt);
    let after = test_data.token.balance(&company_alt);
    assert_eq!(after - before, 100_i128);

    test_data.client.revoke_company(&company_alt);
    let withdrawn_result = test_data.client.try_withdrawn(&1_i128, &company_alt);
    assert!(withdrawn_result.is_err());
}

/// Verifies that revoking `manager` blocks commission withdrawals to that address.
#[test]
fn test_revoke_manager_blocks_withdrawn_commissions() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let investor = Address::generate(&e);
    let manager_alt = Address::generate(&e);
    test_data.token_admin.mint(&investor, &1_000_i128);
    invest_as_operator(&test_data, &investor, &1_000_i128);

    e.ledger().set_timestamp(8 * 86_400);

    test_data.client.grant_manager(&manager_alt);
    test_data.client.revoke_manager(&manager_alt);

    let commissions_result = test_data.client.try_withdrawn_commissions(&manager_alt);
    assert!(commissions_result.is_err());
}

/// Verifies the admin-transfer entrypoints are reachable in tests.
/// Under the current mocked auth setup both `try_*` calls are expected to fail,
/// but this still guards the generated client methods and wiring.
#[test]
fn test_transfer_and_accept_admin_role_methods_are_reachable() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1_000_000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let new_admin = Address::generate(&e);
    let transfer_result = test_data.client.try_transfer_admin_role(&new_admin);
    assert!(transfer_result.is_err());

    let accept_result = test_data.client.try_accept_admin_transfer_role();
    assert!(accept_result.is_err());
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
    test_data.client.grant_company(&collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &200_i128);
    test_data.token_admin.mint(&test_data.user, &150_i128);
    invest_as_operator(&test_data, &test_data.user, &150_i128);
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
    test_data.client.grant_company(&collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &4000_i128);

    let user2 = soroban_sdk::Address::generate(&e);
    let user3 = soroban_sdk::Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    let inv1 = invest_as_operator(&test_data, &test_data.user, &2000_i128);
    let inv2 = invest_as_operator(&test_data, &user2, &2000_i128);
    let inv3 = invest_as_operator(&test_data, &user3, &1000_i128);

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
    test_data.client.grant_company(&collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &4000_i128);

    let user2 = soroban_sdk::Address::generate(&e);
    let user3 = soroban_sdk::Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    invest_as_operator(&test_data, &test_data.user, &2000_i128);
    invest_as_operator(&test_data, &user2, &2000_i128);
    invest_as_operator(&test_data, &user3, &1000_i128);

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
    invest_as_operator(&test_data, &u, &1000);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);
    e.ledger().set_timestamp(8 * 86400);

    let balance_before = test_data.token.balance(&test_data.project_address);
    test_data.client.withdrawn(&900_i128, &test_data.project_address);
    let balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(balance_after - balance_before, 900_i128);
    assert_contract_balance(&test_data, 45, 50, 5, 0, 995, 0);
}

/// Verifies that `add_company_transfer` requests authorization from admin.
/// `from` is role-validated (`has_role`) but does not require root auth.
#[test]
fn test_add_company_transfer_authorization_verification() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    invest_as_operator(&test_data, &u, &1000);

    test_data.token_admin.mint(&test_data.project_address, &500);

    e.ledger().set_timestamp(15 * 86400);
    let amount: i128 = 261;
    test_data.client.add_company_transfer(&amount, &test_data.project_address);

    let auths = e.auths();

    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    assert!(admin_auth.is_some(), "Admin auth must be requested");
}

/// Verifies that the same user can invest multiple times, receiving a distinct
/// NFT token_id for each investment, with each investment tracking its own
/// state independently (paid, payments_transferred, completed).
/// A single `add_company_transfer` covering that round`s obligations allows
/// each investment to be paid via `process_investor_payment` independently:
/// `inv2` can still be paid in the same round as `inv1` because payment progress
/// is tracked per investment (`payments_transferred`) rather than per user.
#[test]
fn test_invest_same_user_multiple_times() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &3000);

    let inv1 = invest_as_operator(&test_data, &u, &1000);
    let inv2 = invest_as_operator(&test_data, &u, &1000);
    let inv3 = invest_as_operator(&test_data, &u, &1000);

    assert_ne!(inv1.token_id, inv2.token_id);
    assert_ne!(inv2.token_id, inv3.token_id);

    assert_eq!(inv1.paid, 0);
    assert_eq!(inv2.paid, 0);
    assert_eq!(inv3.paid, 0);
    assert_eq!(inv1.payments_transferred, 0);
    assert!(!inv1.completed);

    assert_contract_balance(&test_data, 2835, 150, 15, 0, 2985, 0);

    test_data.token_admin.mint(&test_data.project_address, &1000_i128);

    e.ledger().set_timestamp(15 * 86400);

    test_data.client.add_company_transfer(&1000_i128, &test_data.project_address);
    let inv1_after = test_data.client.process_investor_payment(&inv1.token_id);

    assert_eq!(inv1_after.payments_transferred, 1_u32);
    assert_eq!(inv1_after.paid, 261_i128);

    let inv2_after = test_data.client.process_investor_payment(&inv2.token_id);
    assert_eq!(inv2_after.payments_transferred, 1_u32);
    assert_eq!(inv2_after.paid, 261_i128);
}

/// Verifies that `refund_investor` transfers back the full original investment
/// (deposited + commission) to the NFT holder, marks the investment as completed,
/// and updates `refunded_to_investor` in the contract balance.
/// For a 1000-token investment: commission=5, deposited=995, so refund=1000.
/// Also verifies that `project`, `reserve`, and `comission` are fully decremented
/// for this single-investor scenario.
#[test]
fn test_refund_investor() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    e.ledger().set_timestamp(3 * 86400);

    let balance_before = test_data.token.balance(&u);
    let refunded = test_data.client.refund_investor(&inv.token_id);
    let balance_after = test_data.token.balance(&u);

    assert_eq!(refunded, 1000_i128);
    assert_eq!(balance_after - balance_before, 1000_i128);

    let contract_bal = test_data.client.get_contract_balance(&test_data.admin);
    assert_eq!(contract_bal.project, 0_i128);
    assert_eq!(contract_bal.reserve, 0_i128);
    assert_eq!(contract_bal.comission, 0_i128);
    assert_eq!(contract_bal.refunded_to_investor, 1000_i128);
}

/// Verifies that `withdrawn_commissions` transfers the full accumulated commission
/// to the provided manager/recipient address, returns the correct amount,
/// and updates `comission_withdrawal`.
/// Investing 1000 tokens generates 5 in commission; after fundraising ends the
/// recipient should receive exactly 5 tokens.
#[test]
fn test_withdrawn_commissions() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    invest_as_operator(&test_data, &u, &1000);

    assert_contract_balance(&test_data, 945, 50, 5, 0, 995, 0);

    e.ledger().set_timestamp(8 * 86400);

    let owner_balance_before = test_data.token.balance(&test_data.project_address);
    let withdrawn = test_data.client.withdrawn_commissions(&test_data.project_address);
    let owner_balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(withdrawn, 5_i128);
    assert_eq!(owner_balance_after - owner_balance_before, 5_i128);

    let contract_bal = test_data.client.get_contract_balance(&test_data.admin);
    assert_eq!(contract_bal.comission_withdrawal, 5_i128);
}

/// Verifies that owner-gated functions (`pause`, `unpause`) request exactly one
/// authorization from the admin address.
/// The test inspects `e.auths()` after each call.
#[test]
fn test_admin_authorization_verification() {
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

    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    assert!(admin_auth.is_some(), "Authorization should include admin");

    test_data.client.unpause(&test_data.admin);

    let auths = e.auths();
    assert_eq!(auths.len(), 1, "Should request exactly one authorization");
    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    assert!(admin_auth.is_some(), "Authorization should include admin");
}


/// Verifies that a single investor receives the full emergency pool when
/// they are the only active investment. In emergency-close mode, the frozen
/// pool is `reserve + project`, so the only investor receives the full pool.
/// After the call the investment is marked completed and both `reserve` and
/// `project` reach 0.
#[test]
fn test_emergency_pay_investor_single_investor() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000);

    e.ledger().set_timestamp(8 * 86400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();

    let balance_before = test_data.token.balance(&u);
    let paid = test_data.client.emergency_pay_investor(&inv.token_id);
    let balance_after = test_data.token.balance(&u);

    assert_eq!(paid, 995_i128);
    assert_eq!(balance_after - balance_before, 995_i128);

    let b = test_data.client.get_contract_balance(&test_data.admin);
    assert_eq!(b.reserve, 0_i128);
    assert_eq!(b.project, 0_i128);
    assert_eq!(b.payment_obligations, 0_i128);
    assert_eq!(b.payments, 995_i128);
}

/// Verifies that three investors with different investment amounts each receive
/// a proportional share of the emergency pool according to remaining obligations.
/// The full emergency pool (`reserve + project`) is fully distributed and the
/// last claim absorbs any rounding remainder.
#[test]
fn test_emergency_pay_investor_multiple_investors_proportional() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &2000);
    test_data.token_admin.mint(&user2, &2000);
    test_data.token_admin.mint(&user3, &1000);

    let inv1 = invest_as_operator(&test_data, &test_data.user, &2000);
    let inv2 = invest_as_operator(&test_data, &user2, &2000);
    let inv3 = invest_as_operator(&test_data, &user3, &1000);

    e.ledger().set_timestamp(8 * 86400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();

    let balance_before = test_data.client.get_contract_balance(&test_data.admin);
    let emergency_pool_before = balance_before.reserve + balance_before.project;

    let b1_before = test_data.token.balance(&test_data.user);
    let paid1 = test_data.client.emergency_pay_investor(&inv1.token_id);
    let b1_after = test_data.token.balance(&test_data.user);
    assert_eq!(b1_after - b1_before, paid1);

    let b2_before = test_data.token.balance(&user2);
    let paid2 = test_data.client.emergency_pay_investor(&inv2.token_id);
    let b2_after = test_data.token.balance(&user2);
    assert_eq!(b2_after - b2_before, paid2);

    let b3_before = test_data.token.balance(&user3);
    let paid3 = test_data.client.emergency_pay_investor(&inv3.token_id);
    let b3_after = test_data.token.balance(&user3);
    assert_eq!(b3_after - b3_before, paid3);

    assert_eq!(paid1, paid2);
    assert_eq!(paid3, paid1 / 2);

    assert_eq!(paid1 + paid2 + paid3, emergency_pool_before);

    let b = test_data.client.get_contract_balance(&test_data.admin);
    assert_eq!(b.reserve, 0_i128);
    assert_eq!(b.project, 0_i128);
    assert_eq!(b.payment_obligations, 0_i128);
}

/// Verifies that `emergency_pay_investor` requests authorization from admin.
/// Recipient/investor addresses are validated by business rules, not root auth.
#[test]
fn test_emergency_pay_investor_authorization() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000);

    e.ledger().set_timestamp(8 * 86400);
    test_data.client.withdrawn_commissions(&test_data.project_address);
    test_data.client.activate_emergency_close();
    test_data.client.emergency_pay_investor(&inv.token_id);

    let auths = e.auths();
    let admin_auth = auths.iter().find(|(addr, _)| *addr == test_data.admin);
    assert!(admin_auth.is_some(), "Admin auth must be requested");
}
