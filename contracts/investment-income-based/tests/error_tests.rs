mod common;

use common::{create_investment_contract};
use soroban_sdk::{Address, Env, String, testutils::{Address as _, Ledger}};

use crate::common::{create_token_contract};

/// Ensures the constructor panics when `i_rate` is 0.
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_interest_rate_zero() {
    let e = Env::default();
    create_investment_contract(&e, 0_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 100_i128, true);
}

/// Ensures the constructor panics when `goal` is 0.
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_goal_zero() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 7_u64, 0_i128, 1_u32, 4_u32, 100_i128, true);
}

/// Ensures the constructor panics when `return_type` is not a valid variant (0 is unsupported).
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_invalid_return_type() {
    let e = Env::default();
    create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        0_u32,
        4_u32,
        100_i128,
        true,
    );
}

/// Ensures the constructor panics when `return_months` is 0.
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_return_months_zero() {
    let e = Env::default();
    create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        0_u32,
        100_i128,
        true,
    );
}

/// Ensures the constructor panics when `min_per_investment` is 0.
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_constructor_min_investment_zero() {
    let e = Env::default();
    create_investment_contract(&e, 500_u32, 7_u64, 7_u64, 1000000_i128, 1_u32, 4_u32, 0_i128, true);
}

// ==================== Investment Error Tests ====================

/// Verifies that `add_company_transfer` is rejected with `NextPaymentCannotBeScheduledYet` (#43)
/// when called before `ts_payments_start` has been reached.
/// With fundraising_days=7 and claim_block_days=7 payments start at day 14;
/// advancing only to day 12 keeps the ledger before that threshold.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #43)")]
fn test_call_add_company_transfer_before_ts_next_payment() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 7_u64, 90000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &1000);

    test_data.token_admin.mint(&test_data.project_address, &2000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &2000);

    e.ledger().set_timestamp(e.ledger().timestamp() + (12 * 86400));
    test_data.client.add_company_transfer(&1000_i128);
}

/// Verifies that `process_investor_payment` is rejected with `PaymentAlreadyProcessedForThisPeriod` (#42)
/// when called without a prior `add_company_transfer` for the current round.
/// With no company transfer, `next_payment_round` stays at 0, which equals
/// `investment.payments_transferred` (0), triggering the already-processed guard.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #42)")]
fn test_call_process_investor_payment_without_previous_company_transfer() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 7_u64, 90000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000000);
    let inv = test_data.client.invest(&test_data.user, &1000);

    e.ledger().set_timestamp(e.ledger().timestamp() + (30 * 86400));
    test_data.client.process_investor_payment(&inv.token_id);
}   

/// Verifies that investing beyond the funding goal is rejected with `GoalAlreadyReached` (#31):
/// fills the goal nearly to capacity and then tries a third investment that would exceed it.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_goal_reached() {
    let e = Env::default();
    let test_data =
        create_investment_contract(&e, 500_u32, 7_u64, 7_u64, 90000_i128, 2_u32, 4_u32, 100_i128, true);

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &89000);
    test_data.client.invest(&test_data.user, &2200);
    test_data.client.invest(&test_data.user, &1600);
}

/// Verifies that `invest` is rejected with `AddressInsufficientBalance` (#1)
/// when the investor's token balance is lower than the requested investment amount.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_invest_insufficient_balance() {
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

    test_data.token_admin.mint(&test_data.user, &50000);
    test_data.client.invest(&test_data.user, &100000);
}

/// Verifies that `invest` is rejected with `AmountLessThanMinimum` (#5)
/// when the amount is below the configured `min_per_investment`.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_invest_amount_less_than_minimum() {
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

    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &50);
}

/// Verifies that `invest` is rejected with `FundrasingTimeExceeded` (#40)
/// when the ledger timestamp has passed the fundraising deadline.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #40)")]
fn test_invest_after_fundraising_period() {
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

    e.ledger().set_timestamp(8 * 86400);
    test_data.token_admin.mint(&test_data.user, &1000000);
    test_data.client.invest(&test_data.user, &1000);
}

// ==================== Payment Processing Error Tests ====================

/// Verifies that `add_company_transfer` is rejected with `ContractReserveInsufficientBalance` (#2)
/// on the final payment round when the reserve is insufficient to cover the full Coupon payout.
///
/// For Coupon investments, the last round pays `amount_to_pay_per_month + deposited`
/// (interest + full principal). `validate_company_transfer` detects the last round
/// (`next_payment_round == return_months - 1`) and requires `reserve + amount >= payment_obligations`.
///
/// Setup: return_type=2 (Coupon), return_months=2, invest 1000 → reserve=50,
/// amount_to_pay_per_month=24, payment_obligations=1019 (24+995). The admin receives only
/// 10 tokens — enough to pass the non-final check on round 1 (`1 > 24 - 50`),
/// but the final-round check fails on round 2 (`27 + 1 < 1019`).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_add_company_transfer_insufficient_reserve_last_coupon_round() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        2_u32,
        2_u32,
        100_i128,
        true,
    );

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = test_data.client.invest(&test_data.user, &1000);

    test_data.token_admin.mint(&test_data.admin, &10);

    e.ledger().set_timestamp(15 * 86400);

    test_data.client.add_company_transfer(&1_i128);
    test_data.client.process_investor_payment(&inv.token_id);

    test_data.client.add_company_transfer(&1_i128);
    test_data.client.process_investor_payment(&inv.token_id);
}

/// Verifies that `process_investor_payment` is rejected with `InvestmentCompleted` (#38)
/// when called on an investment that has already received all its scheduled payments.
/// Uses return_months=1 so a single company transfer and payment round fully completes
/// the investment. Payments become available after `fundraising_days + claim_block_days` = 14 days.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #38)")]
fn test_process_investor_payment_already_completed() {
    let e = Env::default();
    let test_data = create_investment_contract(
        &e,
        500_u32,
        7_u64,
        7_u64,
        1000000_i128,
        1_u32,
        1_u32,
        100_i128,
        true,
    );

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = test_data.client.invest(&test_data.user, &1000);

    test_data.token_admin.mint(&test_data.project_address, &2000);
    test_data.token.transfer(&test_data.project_address, &test_data.admin, &2000);

    e.ledger().set_timestamp(15 * 86400);
    test_data.client.add_company_transfer(&2000_i128);
    test_data.client.process_investor_payment(&inv.token_id);

    test_data.client.process_investor_payment(&inv.token_id);
}







// ==================== Withdrawal Error Tests ====================

/// Verifies that `withdrawn` is rejected with `FundrasingTimeOngoingYet` (#39)
/// when called before the fundraising deadline has passed.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #39)")]
fn test_withdrawn_while_fundraising_ongoing() {
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

    test_data.token_admin.mint(&test_data.user, &1000);
    test_data.client.invest(&test_data.user, &1000);

    e.ledger().set_timestamp(3 * 86400);
    test_data.client.withdrawn(&500_i128);
}

/// Verifies that `withdrawn` is rejected with `ContractInsufficientBalance` (#3)
/// when the requested amount exceeds the available project balance.
/// Investing 1000 with a 5% rate leaves a project balance of 945 (reserve=50, commission=5),
/// so attempting to withdraw 946 must fail.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_withdrawn_insufficient_project_balance() {
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

    test_data.token_admin.mint(&test_data.user, &1000);
    test_data.client.invest(&test_data.user, &1000);

    e.ledger().set_timestamp(8 * 86400);
    test_data.client.withdrawn(&946_i128);
}







// ==================== Transfer Error Tests ====================

/// Verifies that `add_company_transfer` is rejected with `AddressInsufficientBalance` (#1)
/// when the admin has no tokens to transfer.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_add_company_transfer_insufficient_balance() {
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

    e.ledger().set_timestamp(15 * 86400);
    test_data.client.add_company_transfer(&100000_i128);
}



/// Verifies that `add_collateral` is rejected with `AddressInsufficientBalance` (#1)
/// when the collateral provider has no collateral tokens.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_add_collateral_insufficient_balance() {
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

    let collateral_addr = Address::generate(&e);
    let (token_collateral, _token_collateral_admin)  = create_token_contract(&e, &test_data.admin);

    test_data.client.add_collateral(
        &token_collateral.address, 
        &100_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
        
    );
}

/// Verifies that `add_collateral` is rejected with `OnlyOneCollateralTokenAllowed` (#34)
/// when attempting to register a second, different collateral token.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_add_collateral_only_one_collateral_token_allowed() {
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
    let (token_collateral_2, token_collateral_admin_2)  = create_token_contract(&e, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &200_i128);
    token_collateral_admin_2.mint(&collateral_addr, &200_i128);
    test_data.token_admin.mint(&test_data.user, &150_i128);
    test_data.client.invest(&test_data.user, &150_i128);
    test_data.client.add_collateral(
        &token_collateral.address, 
        &100_i128, 
        &String::from_str(&e,"TEST"), 
        &collateral_addr
    );

    test_data.client.add_collateral(
        &token_collateral_2.address, 
        &100_i128, 
        &String::from_str(&e,"TEST2"), 
        &collateral_addr
    );


}

/// Verifies that `pay_with_collateral` is rejected with `CollateralNotConfigured` (#36)
/// when no collateral has been registered for the contract.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #36)")]
fn test_pay_with_collateral_not_configured() {
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

    test_data.token_admin.mint(&test_data.user, &1000);
    let inv = test_data.client.invest(&test_data.user, &1000);

    test_data.client.pay_with_collateral(&inv.token_id);
}

/// Verifies that `return_collateral_to_company` is rejected with `CollateralBalanceIsEmpty` (#37)
/// when collateral has been configured but its balance in the contract is zero.
/// A single investor's `pay_with_collateral` drains the full collateral balance
/// (they represent 100% of payment_obligations), leaving nothing to return.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #37)")]
fn test_return_collateral_when_empty() {
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

    let collateral_addr = Address::generate(&e);
    let (token_collateral, token_collateral_admin) = create_token_contract(&e, &test_data.admin);
    token_collateral_admin.mint(&collateral_addr, &200_i128);

    test_data.token_admin.mint(&test_data.user, &150_i128);
    let inv = test_data.client.invest(&test_data.user, &150_i128);

    test_data.client.add_collateral(
        &token_collateral.address,
        &100_i128,
        &String::from_str(&e, "TEST"),
        &collateral_addr,
    );

    test_data.client.pay_with_collateral(&inv.token_id);

    test_data.client.return_collateral_to_company();
}


