//! Suite A — property-based fuzzing of the position allocation math
//! (`src/investment/allocation.rs`), exercised through the public `invest`
//! entrypoint.
//!
//! Covered invariants (see the fuzz-testing strategy doc):
//! - P1 sum-of-parts: `deposited + commission == amount` (no dust lost at entry)
//! - P2 `total == deposited + returns` (`get_total_claimable`)
//! - P3 commission never exceeds the exact tiered-rate floor
//! - P4 returns never exceed `i_rate`% of the invested principal
//! - P5 payment schedule: `regular_payment == base / months` (integer division)
//!   and the final payment closes the schedule within `base`
//! - P8 allocation outputs are monotonic non-decreasing in `amount`
//! - INV-1 accounting equation holds after each invest
//!
//! W-1/W-2 are FIXED (see `FUZZ_TESTING.md`): both `Wad` multiplications now
//! use `checked_mul` (I256 fallback) and `Wad::from_token_amount` overflow is
//! guarded. This suite therefore explores the FULL domain:
//! - `invest` returns `Ok`   => P1..P5 + INV-1 must hold exactly.
//! - `invest` returns `Err(TooLargeInvestment)` => valid outcome: the amount
//!   crossed the WAD arithmetic boundary (`amount * i_rate >~ 1.7e13`; see
//!   SAFE_AMOUNT_RATE). No further error (and no trap) is accepted.
//!
//! Accounting must be identical before and after a rejected invest: the guard
//! fires BEFORE the token transfer (fail-fast order in `invest`), so nothing
//! may move (atomicity, INV-19).
//!
//! `cmr_upper > cmr_lower` and all params > 0 are enforced by the constructor;
//! invalid constructor combos belong to directed Suite C tests.

mod common;

use common::*;
use investment_income_based::shared::types::Error;
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, InvokeError};

const GOAL_FAR_AWAY: i128 = i128::MAX;
/// Real success-domain boundary of the allocation math (approximate):
/// `amount * i_rate >~ 1.7e13` (i128::MAX / 1e25, 7-dec token) makes
/// `amount_wad * return_rate_wad` no longer fit in i128 and `invest` returns
/// `Err(TooLargeInvestment)` (W-2 guard). Kept as a constant only for the
/// monotonic test, which needs two *successful* invests to compare.
const SAFE_AMOUNT_RATE: i128 = 17_000_000_000_000; // 1.7e13

fn arb_i_rate() -> impl Strategy<Value = u32> {
    1u32..=10_000
}

/// Amounts spread uniformly across 27 orders of magnitude (1 ..= 1e27), so
/// both outcomes (Ok / TooLargeInvestment) get plenty of cases. Uniform
/// sampling would ignore small amounts almost completely.
fn arb_amount() -> impl Strategy<Value = i128> {
    (0u32..=26).prop_flat_map(|e| 1i128..=10i128.pow(e))
}

/// `(cmr_lower_divisor, cmr_upper_divisor)` with `upper > lower`.
fn arb_cmr_pair() -> impl Strategy<Value = (u32, u32)> {
    (1u32..=1_000, 1u32..=10_000)
        .prop_filter("cmr_upper must exceed cmr_lower", |(lower, upper)| {
            upper > lower
        })
}

fn arb_cmr_reductor() -> impl Strategy<Value = i128> {
    1i128..=1_000_000
}

fn arb_return_months() -> impl Strategy<Value = u32> {
    1u32..=120
}

/// return type can be 1 (Reverse Loan) or 2 (Coupon)
fn arb_return_type() -> impl Strategy<Value = u32> {
    1u32..=2
}

/// `(i_rate, small, big)` with both amounts inside the success domain
/// (`amount * i_rate <=~ SAFE_AMOUNT_RATE`, the W-2 boundary reinterpreted
/// after the fix: beyond it `invest` returns Err, leaving no positions to
/// compare) AND in the same commission tier (same `rate_denominator`), so
/// allocation outputs are directly comparable.
fn arb_same_tier_pair() -> impl Strategy<Value = (u32, (u32, u32), i128, i128, i128)> {
    (arb_i_rate(), arb_cmr_pair(), arb_cmr_reductor())
        .prop_flat_map(|(i_rate, cmr, reductor)| {
            (1i128..=(SAFE_AMOUNT_RATE / i_rate as i128)).prop_flat_map(move |a| {
                // Same tier: `b` must not cross the next `reductor` step.
                let tier_end = (a / reductor + 1) * reductor - 1;
                let b_max = tier_end.min(SAFE_AMOUNT_RATE / i_rate as i128);
                (a..=b_max, Just((a, i_rate, cmr, reductor)))
            })
        })
        .prop_map(|(b, (a, i_rate, cmr, reductor))| (i_rate, cmr, reductor, a, b))
}

/// Deploys a contract with fuzzable allocation params (goal unreachable so
/// `invest` is never capped) and mints `amount` tokens to a fresh investor,
/// but does NOT invest: the caller decides how (plain / try) since the relaxed
/// suite must handle both `Ok` and `Err(TooLargeInvestment)` outcomes.
fn setup_and_mint<'a>(
    env: &'a Env,
    i_rate: u32,
    (cmr_lower, cmr_upper): (u32, u32),
    cmr_reductor: i128,
    return_type: u32,
    return_months: u32,
    amount: i128,
) -> (TestData<'a>, Address) {
    let test_data = create_investment_contract(
        env,
        i_rate,
        7_u64, // claim_block_days
        7_u64, // fundraising_days: invest at ts 0 is inside the window
        GOAL_FAR_AWAY,
        return_type,
        return_months,
        1_i128, // min_per_investment
        true,   // mock auths
        cmr_upper,
        cmr_lower,
        cmr_reductor,
    );

    let investor = Address::generate(env);
    test_data.token_admin.mint(&investor, &amount);

    (test_data, investor)
}

/// Field-wise snapshot of the on-chain `Balance` (for unchanged-state checks).
fn balance_snapshot(test_data: &TestData) -> BalanceExpectation {
    let b = test_data.client.get_contract_balance(&test_data.admin);
    BalanceExpectation {
        payment_rigths: b.assets.payment_rigths,
        contract_earnings: b.assets.contract_earnings,
        managed_volume: b.assets.managed_volume,
        managed_collateral: b.assets.managed_collateral,
        payment_obligations: b.liabilities.payment_obligations,
        return_collateral_obligations: b.liabilities.return_collateral_obligations,
        contract_commissions: b.equity.contract_commissions,
    }
}

/// Assertions for the rejected branch: a `TooLargeInvestment` invest must be a
/// clean no-op (W-2 guard fires BEFORE the token transfer, so nothing moves).
fn assert_rejected_invest(
    test_data: &TestData,
    investor: &Address,
    amount: i128,
    balance_before: &BalanceExpectation,
) {
    // The investor did not pay anything.
    assert_eq!(
        test_data.token.balance(investor),
        amount,
        "investor paid a rejected invest"
    );
    // The contract did not receive anything.
    assert_eq!(
        test_data.token.balance(&test_data.client.address),
        0,
        "contract received funds from a rejected invest"
    );
    // Every accounting field is bit-for-bit unchanged (INV-19).
    assert_eq!(
        &balance_snapshot(test_data),
        balance_before,
        "Balance changed on rejected invest"
    );
}

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        // CI-friendly default; scale up locally with `PROPTEST_CASES=10000`.
        cases: 128,
        max_shrink_iters: 5_000,
        ..proptest::test_runner::Config::default()
    })]

    /// P1, P2, P3, P4, P5 + INV-1 for a single arbitrary investment across the
    /// FULL domain: `Ok` must satisfy every invariant, `Err(TooLargeInvestment)`
    /// must be a clean no-op (W-1/W-2 fixed), anything else (including traps)
    /// is a failure.
    #[test]
    fn allocation_single_invest_invariants(
        i_rate in arb_i_rate(),
        amount in arb_amount(),
        cmr in arb_cmr_pair(),
        cmr_reductor in arb_cmr_reductor(),
        return_type in arb_return_type(),
        return_months in arb_return_months(),
        token_id in any::<u32>(),
    ) {
        let env = Env::default();
        let (test_data, investor) = setup_and_mint(
            &env, i_rate, cmr, cmr_reductor, return_type, return_months, amount,
        );
        let balance_before = balance_snapshot(&test_data);
        ensure_operator(&test_data, &investor);

        match test_data.client.try_invest(&investor, &amount, &token_id) {
            Ok(Ok(position)) => {
                // P1: the investor's tokens are split exactly.
                prop_assert_eq!(position.deposited + position.commission, amount);

                // P2: total claimable == principal + returns.
                prop_assert_eq!(position.total, position.deposited + position.returns);

                // P3: commission never exceeds the exact floor of the tiered rate.
                // Recomputes the integer tier logic of `calculate_rate_denominator`.
                let step = amount / cmr_reductor;
                let denominator: i128 = if step > cmr.1 as i128 {
                    cmr.1 as i128
                } else {
                    cmr.0 as i128 + step
                };
                prop_assert!(
                    position.commission * (denominator * 10_000) <= amount * i_rate as i128,
                    "commission {} exceeds tiered-rate floor (den={denominator})",
                    position.commission
                );

                // P4: returns never exceed i_rate% of the invested principal.
                prop_assert!(
                    position.returns * 10_000 <= position.deposited * i_rate as i128,
                    "returns {} exceed i_rate% of deposited {}",
                    position.returns,
                    position.deposited
                );

                // P5: schedule closes within `base` (total for ReverseLoan, returns
                // for Coupon); regular_payment is the exact integer floor.
                let base = if return_type == 1 {
                    position.total
                } else {
                    position.returns
                };
                prop_assert_eq!(position.regular_payment, base / return_months as i128);
                prop_assert!(
                    position.regular_payment * return_months as i128 <= base,
                    "scheduled regular payments exceed base"
                );
                prop_assert!(
                    base - position.regular_payment * (return_months as i128 - 1)
                        >= position.regular_payment,
                    "final payment falls below the regular payment"
                );

                // INV-1: accounting equation holds after the invest.
                assert_balance_equation(&test_data);
            }
            // W-2 guard fired: the amount crossed the WAD boundary. This is a
            // valid outcome, but nothing may change.
            Err(Ok(Error::TooLargeInvestment)) => {
                assert_rejected_invest(&test_data, &investor, amount, &balance_before);
            }
            // Test failure: the contract returned a business error (its Error
            // enum), but not the one we expect here (only #61 is valid).
            Err(Ok(e)) => prop_assert!(false, "invest failed with unexpected error {e:?}"),
            // Test failure: the value returned by the contract could not be
            // deserialized into the Position struct that the client expects.
            Ok(Err(cnv)) => prop_assert!(false, "invest returned a conversion error {cnv:?}"),
            // Test failure: the contract call trapped (Rust panic,
            // panic_with_error!, ...). INV-18: no invocation may trap.
            Err(Err(InvokeError::Abort)) => {
                prop_assert!(false, "INV-18: invest trapped instead of returning a contract error")
            }
            // Test failure: any other host-level error (not a contract Error),
            // e.g. failed auth or exhausted instruction budget.
            Err(Err(e)) => prop_assert!(false, "invest failed with a host error {e:?}"),
        }
    }

    /// P8: investing more money must never give you a worse deal — as long as
    /// both investments land in the SAME commission tier.
    ///
    /// How it checks: invest `a` first, then `b > a`, and assert that every
    /// output (commission, deposited, returns, total, regular_payment) is
    /// >= a's. Within one tier the only rounding is integer truncation, which
    /// can flatten outputs (make them equal) but never invert the order.
    ///
    /// Cross-tier pairs are intentionally NOT compared: crossing a tier
    /// boundary lowers the commission rate by design, so slightly more money
    /// CAN yield a smaller commission. That is business semantics, not a bug
    /// (fuzz counterexample: i_rate=5616, cmr=(1,2278), reductor=492112,
    /// a=1119554039, b=1121418804).
    #[test]
    fn allocation_monotonic_within_tier(
        same_tier in arb_same_tier_pair(),
        return_type in arb_return_type(),
        return_months in arb_return_months(),
    ) {
        let (i_rate, cmr, cmr_reductor, a, b) = same_tier;
        // Defensive: the strategy already guarantees a <= b and same tier.
        let a = a.min(b);
        let b = b.max(a);

        let env = Env::default();
        let test_data = create_investment_contract(
            &env,
            i_rate,
            7_u64,
            7_u64,
            GOAL_FAR_AWAY,
            return_type,
            return_months,
            1_i128,
            true,
            cmr.1,
            cmr.0,
            cmr_reductor,
        );

        let investor_a = Address::generate(&env);
        let investor_b = Address::generate(&env);
        test_data.token_admin.mint(&investor_a, &a);
        test_data.token_admin.mint(&investor_b, &b);
        let pos_a = invest_as_operator(&test_data, &investor_a, &a, &1_u32);
        let pos_b = invest_as_operator(&test_data, &investor_b, &b, &2_u32);

        prop_assert!(pos_a.commission <= pos_b.commission, "commission not monotonic within tier");
        prop_assert!(pos_a.deposited <= pos_b.deposited, "deposited not monotonic within tier");
        prop_assert!(pos_a.returns <= pos_b.returns, "returns not monotonic within tier");
        prop_assert!(pos_a.total <= pos_b.total, "total not monotonic within tier");
        prop_assert!(
            pos_a.regular_payment <= pos_b.regular_payment,
            "regular_payment not monotonic within tier"
        );

        // INV-1: accounting equation holds after both invests.
        assert_balance_equation(&test_data);
    }
}
