use soroban_sdk::Env;
use stellar_contract_utils::math::wad::Wad;

const LOWER_AMOUNT_FOR_COMMISSION_REDUCTION: i128 = 100;
const LOWER_DIVISOR: u32 = 10;
const UPPER_DIVISOR: u32 = 60;
const AMOUNT_PER_COMMISSION_REDUCTION: i128 = 400;

/// Calculates the dynamic commission denominator for an investment amount.
///
/// The denominator starts at `LOWER_DIVISOR` for small amounts and increases
/// by steps of 1 every `AMOUNT_PER_COMMISSION_REDUCTION` tokens above
/// `LOWER_AMOUNT_FOR_COMMISSION_REDUCTION`, capped at `UPPER_DIVISOR`.
pub fn calculate_rate_denominator(amount: &i128, decimals: u32) -> u32 {
    let scale_factor = 10_i128.pow(decimals);
    let token_amount = amount / scale_factor;

    if token_amount <= LOWER_AMOUNT_FOR_COMMISSION_REDUCTION {
        return LOWER_DIVISOR;
    }

    let a = (token_amount - LOWER_AMOUNT_FOR_COMMISSION_REDUCTION) / AMOUNT_PER_COMMISSION_REDUCTION;
    if a > UPPER_DIVISOR as i128 {
        return UPPER_DIVISOR;
    }

    LOWER_DIVISOR + a as u32
}

pub struct Amount {
    pub amount_to_invest: i128,
    pub amount_to_reserve_fund: i128,
    pub amount_to_commission: i128,
}

impl Amount {
    /// Returns amount effectively allocated to investment flow
    /// (`project + reserve`, excluding commission).
    pub fn get_invested_amount(&self) -> i128 {
        self.amount_to_invest + self.amount_to_reserve_fund
    }

    /// Splits an investment amount into commission, reserve, and project buckets.
    ///
    /// Uses OpenZeppelin `Wad` math helpers for fixed-point calculations:
    /// * commission uses `i_rate / (rate_denominator * 10_000)`
    /// * reserve is fixed at 5%
    /// * project receives the remainder
    ///
    /// # Panics
    ///
    /// Panics if token decimals do not fit into `u8` for `Wad` conversion.
    pub fn from_investment(e: &Env, amount: &i128, i_rate: &u32, decimals: u32) -> Amount {
        let rate_denominator: u32 = calculate_rate_denominator(amount, decimals);

        let decimals_for_wad: u8 = decimals
            .try_into()
            .expect("Token decimals must fit in u8")
        ;

        let amount_wad = Wad::from_token_amount(e, *amount, decimals_for_wad);
        let commission_rate =
            Wad::from_ratio(e, *i_rate as i128, (rate_denominator as i128) * 10_000);

        let reserve_rate = Wad::from_ratio(e, 5, 100);

        let amount_to_commission_wad = amount_wad * commission_rate;
        let amount_to_reserve_fund_wad = amount_wad * reserve_rate;
        let amount_to_invest_wad = amount_wad - amount_to_commission_wad - amount_to_reserve_fund_wad;

        Amount {
            amount_to_commission: amount_to_commission_wad.to_token_amount(e, decimals_for_wad),
            amount_to_reserve_fund: amount_to_reserve_fund_wad.to_token_amount(e, decimals_for_wad),
            amount_to_invest: amount_to_invest_wad.to_token_amount(e, decimals_for_wad),
        }
    }
}
