mod common;

use common::{
    assert_balance, create_investment_contract, do_payment_round,
    do_payment_round_without_company_transfer, grant_company_exclusive, invest_as_operator,
    BalanceExpectation,
};
use investment_income_based::shared::types::LiquidateInvestmentsStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

use crate::common::create_token_contract;

/// Verifies the complete lifecycle of a Coupon investment across 4 payment rounds.
/// Rounds 1–3 pay interest only (`regular_payment = 6`); round 4 pays the
/// remaining interest + full principal (`504`), marking the investment as completed.
/// The accounting snapshot is asserted after every movement (investment, company
/// contribution, payment) and the balance equation `assets == liabilities + equity`
/// is verified each time.
///
/// Accounting trace (assets: payment_rigths/contract_earnings/managed_volume,
/// liabilities: payment_obligations, equity: contract_commissions):
/// - invest 500:        assets = 0 + 2 + 522, liabilities = 522, equity = 2
/// - r1 (transfer 7):   payment_rigths = -7, managed_volume = 523, obligations = 516
/// - r2 (transfer 7):   payment_rigths = -14, managed_volume = 524, obligations = 510
/// - r3 (transfer 7):   payment_rigths = -21, managed_volume = 525, obligations = 504
/// - r4 (transfer 501): payment_rigths = -522, managed_volume = 522, obligations = 0
#[test]
fn test_flow_with_coupon() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        2_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        400,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);

    assert_balance(&test_data, BalanceExpectation::default());

    let inv = invest_as_operator(&test_data, &u, &500, &1234_u32);

    assert_eq!(inv.commission, 2_i128);
    assert_eq!(inv.deposited, 498_i128);
    assert_eq!(inv.returns, 24_i128);
    assert_eq!(inv.total, 522_i128);
    assert_eq!(inv.regular_payment, 6_i128);

    // Investment: commission is both an asset (contract_earnings) and equity;
    // the full future return (total) is a managed_volume asset backed by the
    // payment_obligations liability.
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 2,
            managed_volume: 522,
            payment_obligations: 522,
            contract_commissions: 2,
            ..Default::default()
        },
    );

    test_data
        .token_admin
        .mint(&test_data.project_address, &2000);

    do_payment_round(&e, &test_data, inv.token_id, 7, 6, 1, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -7,
            contract_earnings: 2,
            managed_volume: 523,
            payment_obligations: 516,
            contract_commissions: 2,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 7, 12, 2, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -14,
            contract_earnings: 2,
            managed_volume: 524,
            payment_obligations: 510,
            contract_commissions: 2,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 7, 18, 3, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -21,
            contract_earnings: 2,
            managed_volume: 525,
            payment_obligations: 504,
            contract_commissions: 2,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 501, 522, 4, true);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -522,
            contract_earnings: 2,
            managed_volume: 522,
            payment_obligations: 0,
            contract_commissions: 2,
            ..Default::default()
        },
    );
}

/// Verifies the complete lifecycle of a ReverseLoan investment across 4 payment rounds.
/// Each round pays `regular_payment = 261` (equal principal + proportional interest instalment).
/// After all 4 rounds the investment is marked completed and `paid` equals `total`.
/// The accounting snapshot and the balance equation are asserted after every movement.
#[test]
fn test_flow_with_reverse_loan() {
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
        60,
        10,
        1200,
    );

    assert_balance(&test_data, BalanceExpectation::default());

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    assert_eq!(inv.commission, 5_i128);
    assert_eq!(inv.deposited, 995_i128);
    assert_eq!(inv.returns, 49_i128);
    assert_eq!(inv.total, 1044_i128);
    assert_eq!(inv.regular_payment, 261_i128);

    test_data
        .token_admin
        .mint(&test_data.project_address, &2000);

    do_payment_round(&e, &test_data, inv.token_id, 261, 261, 1, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -261,
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 783,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 261, 522, 2, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -522,
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 522,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 261, 783, 3, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -783,
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 261,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round(&e, &test_data, inv.token_id, 261, 1044, 4, true);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -1044,
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 0,
            contract_commissions: 5,
            ..Default::default()
        },
    );
}

/// Verifies the ReverseLoan lifecycle when liquidations are disabled: payments
/// still update the accounting state (managed_volume and payment_obligations
/// decrease per round) even though no token transfer is performed in-contract.
#[test]
fn test_flow_payments_disabled() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    e.ledger()
        .set_timestamp(e.ledger().timestamp() + (8 * 86_401));
    test_data.client.disable_investment_liquidations();

    assert_eq!(
        test_data.client.check_investment_liquidations(&u),
        LiquidateInvestmentsStatus::Disabled
    );

    do_payment_round_without_company_transfer(&e, &test_data, inv.token_id, 261, 1, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 783,
            payment_obligations: 783,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round_without_company_transfer(&e, &test_data, inv.token_id, 522, 2, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 522,
            payment_obligations: 522,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round_without_company_transfer(&e, &test_data, inv.token_id, 783, 3, false);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 261,
            payment_obligations: 261,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    do_payment_round_without_company_transfer(&e, &test_data, inv.token_id, 1044, 4, true);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 0,
            payment_obligations: 0,
            contract_commissions: 5,
            ..Default::default()
        },
    );
}

/// Verifies that multiple investors can all be paid within the same payment round:
/// a single `add_company_transfer` covering the total per-round obligations allows
/// each `process_investor_payment` call to succeed, with each investment tracking
/// its own `payments_transferred` counter independently. Balance is asserted after
/// every movement.
#[test]
fn test_multiple_investors_same_payment_round() {
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
        60,
        10,
        1200,
    );

    let u1: Address = Address::generate(&e);
    let u2: Address = Address::generate(&e);
    test_data.token_admin.mint(&u1, &1000);
    test_data.token_admin.mint(&u2, &1000);

    let inv1 = invest_as_operator(&test_data, &u1, &1000, &1234_u32);
    let inv2 = invest_as_operator(&test_data, &u2, &1000, &1235_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 10,
            managed_volume: 2088,
            payment_obligations: 2088,
            contract_commissions: 10,
            ..Default::default()
        },
    );

    test_data
        .token_admin
        .mint(&test_data.project_address, &3000);

    e.ledger().set_timestamp(15 * 86400);

    test_data
        .client
        .add_company_transfer(&522_i128, &test_data.project_address);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -522,
            contract_earnings: 10,
            managed_volume: 2610,
            payment_obligations: 2088,
            contract_commissions: 10,
            ..Default::default()
        },
    );

    let inv1_paid = test_data.client.process_investor_payment(&inv1.token_id);
    let inv2_paid = test_data.client.process_investor_payment(&inv2.token_id);

    assert_eq!(inv1_paid.paid, 261_i128);
    assert_eq!(inv1_paid.payments_transferred, 1_u32);
    assert!(!inv1_paid.completed);

    assert_eq!(inv2_paid.paid, 261_i128);
    assert_eq!(inv2_paid.payments_transferred, 1_u32);
    assert!(!inv2_paid.completed);

    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -522,
            contract_earnings: 10,
            managed_volume: 2088,
            payment_obligations: 1566,
            contract_commissions: 10,
            ..Default::default()
        },
    );
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
        60,
        10,
        1000,
    );

    test_data.client.pause(&test_data.admin);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let invest_result = test_data
        .client
        .try_invest(&test_data.user, &100000, &1234_u32);
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
        60,
        10,
        1000,
    );

    test_data.client.pause(&test_data.admin);
    test_data.client.unpause(&test_data.admin);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let investment = invest_as_operator(&test_data, &test_data.user, &100000, &1234_u32);
    assert!(investment.deposited > 0);
}

#[test]
fn test_revoke_operator_blocks_invest() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1_000_000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        1000,
    );

    let operator = Address::generate(&e);
    test_data.token_admin.mint(&operator, &2_000_i128);

    test_data.client.grant_operator(&operator);
    let _ = test_data.client.invest(&operator, &1_000_i128, &1234_u32);

    test_data.client.revoke_operator(&operator);
    let invest_result = test_data.client.try_invest(&operator, &200_i128, &1235_u32);
    assert!(invest_result.is_err());
}

/// Verifies that granting `company` enables receiving project withdrawals,
/// and revoking it blocks `withdrawn` for that address. Since `company` is
/// single-holder, the initial holder (`project_address`) is rotated out first.
/// The withdrawal moves the amount from `managed_volume` into `payment_rigths`
/// (a company outflow), keeping the balance equation intact.
#[test]
fn test_grant_and_revoke_company_role() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1_000_000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        1000,
    );

    let investor = Address::generate(&e);
    let company_alt = Address::generate(&e);
    test_data.token_admin.mint(&investor, &1_000_i128);
    let inv = invest_as_operator(&test_data, &investor, &1_000_i128, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            payment_obligations: inv.total,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );

    e.ledger().set_timestamp(8 * 86_400);

    grant_company_exclusive(&test_data, &company_alt);
    let before = test_data.token.balance(&company_alt);
    test_data.client.withdrawn(&100_i128, &company_alt);
    let after = test_data.token.balance(&company_alt);
    assert_eq!(after - before, 100_i128);

    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: 100,
            contract_earnings: inv.commission,
            managed_volume: inv.total - 100,
            payment_obligations: inv.total,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );

    test_data.client.revoke_company(&company_alt);
    let withdrawn_result = test_data.client.try_withdrawn(&1_i128, &company_alt);
    assert!(withdrawn_result.is_err());
}

/// Verifies that revoking `manager` blocks commission withdrawals to that address.
/// Since `manager` is single-holder, the initial holder (`project_address`) is
/// revoked first to free the slot before granting the alternate address.
#[test]
fn test_revoke_manager_blocks_withdrawn_commissions() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1_000_000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        1000,
    );

    let investor = Address::generate(&e);
    let manager_alt = Address::generate(&e);
    test_data.token_admin.mint(&investor, &1_000_i128);
    let inv = invest_as_operator(&test_data, &investor, &1_000_i128, &1234_u32);

    e.ledger().set_timestamp(8 * 86_400);

    test_data.client.revoke_manager(&test_data.project_address);
    test_data.client.grant_manager(&manager_alt);
    test_data.client.revoke_manager(&manager_alt);

    let commissions_result = test_data.client.try_withdrawn_commissions(&manager_alt);
    assert!(commissions_result.is_err());

    // No movement happened: commission remains accrued as asset and equity.
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            payment_obligations: inv.total,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );
}

/// Verifies the admin-transfer entrypoints are reachable in tests.
/// Under the current mocked auth setup both `try_*` calls are expected to fail,
/// but this still guards the generated client methods and wiring.
#[test]
fn test_transfer_and_accept_admin_role_methods_are_reachable() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1_000_000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        1000,
    );

    let new_admin = Address::generate(&e);
    let transfer_result = test_data.client.try_transfer_admin_role(&new_admin);
    assert!(transfer_result.is_err());

    let accept_result = test_data.client.try_accept_admin_transfer_role();
    assert!(accept_result.is_err());
}

/// Checks that `add_collateral` correctly computes and stores the collateral
/// level on first deposit, and that a second deposit from the same token
/// increases the reported collateral level proportionally. Each collateral
/// deposit is booked as a `managed_collateral` asset with a matching
/// `return_collateral_obligations` liability.
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
        true,
        60,
        10,
        1000,
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin) = create_token_contract(&e, &test_data.admin);
    grant_company_exclusive(&test_data, &collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &200_i128);
    test_data.token_admin.mint(&test_data.user, &150_i128);
    let inv = invest_as_operator(&test_data, &test_data.user, &150_i128, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            payment_obligations: inv.total,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );

    let collateral_level = test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&e, "TEST"),
        &collateral_addr,
    );

    assert!(collateral_level > 0_u32);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            managed_collateral: 100,
            payment_obligations: inv.total,
            return_collateral_obligations: 100,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );
    let last_collateral_level = collateral_level;

    let collateral_level = test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&e, "TEST"),
        &collateral_addr,
    );

    assert!(collateral_level > last_collateral_level);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            managed_collateral: 200,
            payment_obligations: inv.total,
            return_collateral_obligations: 200,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );
}

/// Validates proportional collateral distribution across investors:
/// two equal investments receive the same collateral payout while a
/// smaller investment receives a proportionally lower amount. Every
/// liquidation decreases `managed_volume`/`payment_obligations` by the
/// remaining obligation and `managed_collateral`/
/// `return_collateral_obligations` by the transferred collateral amount.
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
        true,
        60,
        10,
        2000,
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin) = create_token_contract(&e, &test_data.admin);
    grant_company_exclusive(&test_data, &collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &1_i128);

    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    let inv1 = invest_as_operator(&test_data, &test_data.user, &2000_i128, &1234_u32);
    let inv2 = invest_as_operator(&test_data, &user2, &2000_i128, &1235_u32);
    let inv3 = invest_as_operator(&test_data, &user3, &1000_i128, &1236_u32);

    let obligations_total = inv1.total + inv2.total + inv3.total;
    let commissions_total = inv1.commission + inv2.commission + inv3.commission;

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total,
            payment_obligations: obligations_total,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    test_data.client.add_collateral(
        &token_collateral.address,
        &1_i128,
        &String::from_str(&e, "BTC"),
        &collateral_addr,
    );

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total,
            managed_collateral: 1,
            payment_obligations: obligations_total,
            return_collateral_obligations: 1,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    let paid1 = test_data.client.pay_with_collateral(&inv1.token_id);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total - inv1.total,
            managed_collateral: 1 - paid1,
            payment_obligations: obligations_total - inv1.total,
            return_collateral_obligations: 1 - paid1,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    let paid2 = test_data.client.pay_with_collateral(&inv2.token_id);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total - inv1.total - inv2.total,
            managed_collateral: 1 - paid1 - paid2,
            payment_obligations: obligations_total - inv1.total - inv2.total,
            return_collateral_obligations: 1 - paid1 - paid2,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    let paid3 = test_data.client.pay_with_collateral(&inv3.token_id);
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: 0,
            managed_collateral: 1 - paid1 - paid2 - paid3,
            payment_obligations: 0,
            return_collateral_obligations: 1 - paid1 - paid2 - paid3,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    assert!(paid1 == paid2);
    assert!(paid3 <= paid1);

    // The remaining tracked collateral asset must match the actual
    // collateral token balance held by the contract.
    assert!(token_collateral.balance(&test_data.client.address) > 0);
    assert_eq!(
        token_collateral.balance(&test_data.client.address),
        1 - paid1 - paid2 - paid3
    );
}

/// Confirms that `return_collateral_to_company` transfers the entire
/// collateral token balance held by the contract back to the collateral
/// provider address, leaving the contract with a zero balance and clearing
/// both the `managed_collateral` asset and the
/// `return_collateral_obligations` liability.
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
        true,
        60,
        10,
        2000,
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin) = create_token_contract(&e, &test_data.admin);
    grant_company_exclusive(&test_data, &collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &4000_i128);

    let user2 = soroban_sdk::Address::generate(&e);
    let user3 = soroban_sdk::Address::generate(&e);

    test_data.token_admin.mint(&test_data.user, &1000000_i128);
    test_data.token_admin.mint(&user2, &1000000_i128);
    test_data.token_admin.mint(&user3, &1000000_i128);

    let inv1 = invest_as_operator(&test_data, &test_data.user, &2000_i128, &1234_u32);
    let inv2 = invest_as_operator(&test_data, &user2, &2000_i128, &1235_u32);
    let inv3 = invest_as_operator(&test_data, &user3, &1000_i128, &1236_u32);

    let obligations_total = inv1.total + inv2.total + inv3.total;
    let commissions_total = inv1.commission + inv2.commission + inv3.commission;

    test_data.client.add_collateral(
        &token_collateral.address,
        &4000_i128,
        &String::from_str(&e, "TEST"),
        &collateral_addr,
    );

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total,
            managed_collateral: 4000,
            payment_obligations: obligations_total,
            return_collateral_obligations: 4000,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );

    assert_eq!(
        token_collateral_admin.balance(&test_data.client.address),
        4000_i128
    );
    test_data.client.return_collateral_to_company();
    assert_eq!(
        token_collateral_admin.balance(&test_data.client.address),
        0_i128
    );

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: commissions_total,
            managed_volume: obligations_total,
            payment_obligations: obligations_total,
            contract_commissions: commissions_total,
            ..Default::default()
        },
    );
}

/// Verifies that `withdrawn` transfers project funds to the `project_address`
/// once the fundraising period has ended. The withdrawn amount moves from
/// `managed_volume` into `payment_rigths` (company outflow) and the token
/// balance of the recipient increases accordingly.
#[test]
fn test_withdrawn() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    invest_as_operator(&test_data, &u, &1000, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    e.ledger().set_timestamp(8 * 86400);

    let balance_before = test_data.token.balance(&test_data.project_address);
    test_data
        .client
        .withdrawn(&900_i128, &test_data.project_address);
    let balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(balance_after - balance_before, 900_i128);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: 900,
            contract_earnings: 5,
            managed_volume: 144,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );
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
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        4_u32,
        100_i128,
        true,
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &3000);

    let inv1 = invest_as_operator(&test_data, &u, &1000, &1234_u32);
    let inv2 = invest_as_operator(&test_data, &u, &1000, &1235_u32);
    let inv3 = invest_as_operator(&test_data, &u, &1000, &1236_u32);

    assert_ne!(inv1.token_id, inv2.token_id);
    assert_ne!(inv2.token_id, inv3.token_id);

    assert_eq!(inv1.paid, 0);
    assert_eq!(inv2.paid, 0);
    assert_eq!(inv3.paid, 0);
    assert_eq!(inv1.payments_transferred, 0);
    assert!(!inv1.completed);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 15,
            managed_volume: 3132,
            payment_obligations: 3132,
            contract_commissions: 15,
            ..Default::default()
        },
    );

    test_data
        .token_admin
        .mint(&test_data.project_address, &1000_i128);

    e.ledger().set_timestamp(15 * 86400);

    test_data
        .client
        .add_company_transfer(&1000_i128, &test_data.project_address);
    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -1000,
            contract_earnings: 15,
            managed_volume: 4132,
            payment_obligations: 3132,
            contract_commissions: 15,
            ..Default::default()
        },
    );

    let inv1_after = test_data.client.process_investor_payment(&inv1.token_id);

    assert_eq!(inv1_after.payments_transferred, 1_u32);
    assert_eq!(inv1_after.paid, 261_i128);

    let inv2_after = test_data.client.process_investor_payment(&inv2.token_id);
    assert_eq!(inv2_after.payments_transferred, 1_u32);
    assert_eq!(inv2_after.paid, 261_i128);

    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -1000,
            contract_earnings: 15,
            managed_volume: 3610,
            payment_obligations: 2610,
            contract_commissions: 15,
            ..Default::default()
        },
    );
}

/// Verifies that `refund_investor` transfers back the full original investment
/// (deposited + commission) to the NFT holder, marks the investment as completed,
/// and clears `managed_volume` and `payment_obligations` by the position total.
/// Since the commission cash leaves with the refund, the accrued commission is
/// also unbooked from `contract_earnings`/equity.
/// For a 1000-token investment: commission=5, deposited=995, so refund=1000.
#[test]
fn test_refund_investor() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    e.ledger().set_timestamp(3 * 86400);

    let balance_before = test_data.token.balance(&u);
    let refunded = test_data.client.refund_investor(&inv.token_id);
    let balance_after = test_data.token.balance(&u);

    assert_eq!(refunded, 1000_i128);
    assert_eq!(balance_after - balance_before, 1000_i128);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 0,
            managed_volume: 0,
            payment_obligations: 0,
            contract_commissions: 0,
            ..Default::default()
        },
    );
}

/// Verifies that `withdrawn_commissions` transfers the full accumulated commission
/// to the provided manager/recipient address, returns the correct amount, and
/// clears both the `contract_earnings` asset and the `contract_commissions` equity.
/// Investing 1000 tokens generates 5 in commission; after fundraising ends the
/// recipient should receive exactly 5 tokens.
#[test]
fn test_withdrawn_commissions() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    invest_as_operator(&test_data, &u, &1000, &1234_u32);

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 1044,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    e.ledger().set_timestamp(8 * 86400);

    let owner_balance_before = test_data.token.balance(&test_data.project_address);
    let withdrawn = test_data
        .client
        .withdrawn_commissions(&test_data.project_address);
    let owner_balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(withdrawn, 5_i128);
    assert_eq!(owner_balance_after - owner_balance_before, 5_i128);

    assert_balance(
        &test_data,
        BalanceExpectation {
            managed_volume: 1044,
            payment_obligations: 1044,
            ..Default::default()
        },
    );
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
        60,
        10,
        1000,
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

/// Verifies that emergency close can be activated after fundraising ends even
/// when the contract holds no funds and no obligations: the activation only
/// requires post-fundraising timing and zero pending commissions.
#[test]
fn test_activate_emergency_close_with_no_positions() {
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
        60,
        10,
        1200,
    );

    e.ledger().set_timestamp(8 * 86400);

    assert!(test_data.client.activate_emergency_close());
    assert_balance(&test_data, BalanceExpectation::default());
}

/// Verifies the new emergency flow: after every obligation is settled and all
/// commissions are withdrawn, the admin activates emergency close and sweeps
/// every remaining token out of the contract with `withdrawn_all`. The balance
/// snapshot is reset to zeros afterwards.
///
/// Token trace: investor deposits 1000, company contributes 261*4 = 1044,
/// payments out 1044, commissions out 5 => contract sweeps 995.
#[test]
fn test_emergency_close_and_withdrawn_all() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000, &1234_u32);

    test_data
        .token_admin
        .mint(&test_data.project_address, &2000);

    do_payment_round(&e, &test_data, inv.token_id, 261, 261, 1, false);
    do_payment_round(&e, &test_data, inv.token_id, 261, 522, 2, false);
    do_payment_round(&e, &test_data, inv.token_id, 261, 783, 3, false);
    do_payment_round(&e, &test_data, inv.token_id, 261, 1044, 4, true);

    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -1044,
            contract_earnings: 5,
            managed_volume: 1044,
            payment_obligations: 0,
            contract_commissions: 5,
            ..Default::default()
        },
    );

    let withdrawn_commission = test_data
        .client
        .withdrawn_commissions(&test_data.project_address);
    assert_eq!(withdrawn_commission, 5_i128);

    assert_balance(
        &test_data,
        BalanceExpectation {
            payment_rigths: -1044,
            managed_volume: 1044,
            payment_obligations: 0,
            ..Default::default()
        },
    );

    assert!(test_data.client.activate_emergency_close());

    let manager_balance_before = test_data.token.balance(&test_data.project_address);
    let swept = test_data.client.withdrawn_all(&test_data.project_address);
    let manager_balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(swept, 995_i128);
    assert_eq!(manager_balance_after - manager_balance_before, 995_i128);

    // Full reset: the balance snapshot is zeroed and the equation holds.
    assert_balance(&test_data, BalanceExpectation::default());
}

/// Verifies the emergency sweep order with **pending payment obligations**:
///
/// 1. An investment is created and collateral is deposited while obligations
///    are still outstanding.
/// 2. Commissions are withdrawn and emergency close is activated with the
///    position still unpaid (`payment_obligations = 1044`).
/// 3. `withdrawn_all_collateral` runs **first**: it sweeps the remaining
///    collateral to the manager address and lowers `managed_collateral` /
///    `return_collateral_obligations` symmetrically (no full reset —
///    payment obligations stay on the books).
/// 4. `withdrawn_all` then sweeps the whole contract token balance (investor
///    principal included) to the manager address — the integrator decides
///    what to do with those funds off-chain — and fully resets the balance.
///
/// Token trace: investor deposits 1000, commissions out 5 => `withdrawn_all`
/// sweeps 995; collateral held 100 => `withdrawn_all_collateral` sweeps 100.
#[test]
fn test_withdrawn_all_and_withdrawn_all_collateral_in_emergency_with_pending_obligations() {
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
        60,
        10,
        1200,
    );

    let u: Address = Address::generate(&e);
    test_data.token_admin.mint(&u, &1000);
    let inv = invest_as_operator(&test_data, &u, &1000, &1234_u32);

    // Obligations stay pending on purpose: no payment rounds are run.
    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            payment_obligations: inv.total,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin) = create_token_contract(&e, &test_data.admin);
    grant_company_exclusive(&test_data, &collateral_addr);
    token_collateral_admin.mint(&collateral_addr, &200_i128);

    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&e, "TEST"),
        &collateral_addr,
    );

    assert_balance(
        &test_data,
        BalanceExpectation {
            contract_earnings: inv.commission,
            managed_volume: inv.total,
            managed_collateral: 100,
            payment_obligations: inv.total,
            return_collateral_obligations: 100,
            contract_commissions: inv.commission,
            ..Default::default()
        },
    );

    // Emergency gate requires commissions to be fully withdrawn first.
    e.ledger().set_timestamp(8 * 86400);
    let withdrawn_commission = test_data
        .client
        .withdrawn_commissions(&test_data.project_address);
    assert_eq!(withdrawn_commission, inv.commission);
    assert!(test_data.client.activate_emergency_close());

    // Sweep the remaining collateral first: withdrawn_all requires the
    // contract to hold no collateral at sweep time (CollateralStillRemaining-
    // OnContract, #60). The collateral sweep lowers managed_collateral and
    // return_collateral_obligations symmetrically; payment obligations stay
    // on the books (1044).
    let manager_collateral_before = token_collateral.balance(&test_data.project_address);
    let swept_collateral = test_data
        .client
        .withdrawn_all_collateral(&test_data.project_address);
    let manager_collateral_after = token_collateral.balance(&test_data.project_address);

    assert_eq!(swept_collateral, 100_i128);
    assert_eq!(
        manager_collateral_after - manager_collateral_before,
        100_i128
    );
    assert_balance(
        &test_data,
        BalanceExpectation {
            managed_volume: inv.total,
            payment_obligations: inv.total,
            ..Default::default()
        },
    );

    // Now sweep the contract token balance to the manager even though
    // payment_obligations is still 1044; the balance ends fully reset.
    let manager_balance_before = test_data.token.balance(&test_data.project_address);
    let swept = test_data.client.withdrawn_all(&test_data.project_address);
    let manager_balance_after = test_data.token.balance(&test_data.project_address);

    assert_eq!(swept, 995_i128);
    assert_eq!(manager_balance_after - manager_balance_before, 995_i128);
    assert_balance(&test_data, BalanceExpectation::default());
}
