use soroban_sdk::Env;
use stellar_contract_utils::math::wad::Wad;

const LOWER_AMOUNT_FOR_COMMISSION_REDUCTION: i128 = 100;
const LOWER_DIVISOR: u32 = 10;
const UPPER_DIVISOR: u32 = 60;
const AMOUNT_PER_COMMISSION_REDUCTION: i128 = 400;

/// Computes commission denominator tier based on token amount.
///
/// Larger investments increase the denominator (thus reducing commission rate)
/// up to the configured upper bound.
pub fn calculate_rate_denominator(amount: &i128, decimals: u32) -> u32 {
    let scale_factor = 10_i128.pow(decimals);
    let token_amount = amount / scale_factor;

    if token_amount <= LOWER_AMOUNT_FOR_COMMISSION_REDUCTION {
        return LOWER_DIVISOR;
    }

    let step = (token_amount - LOWER_AMOUNT_FOR_COMMISSION_REDUCTION) / AMOUNT_PER_COMMISSION_REDUCTION;
    if step > UPPER_DIVISOR as i128 {
        return UPPER_DIVISOR;
    }

    LOWER_DIVISOR + step as u32
}

/// Allocation split of an investment amount.
pub struct InvestmentAllocation {
    pub amount_to_invest: i128,
    pub amount_to_reserve_fund: i128,
    pub amount_to_commission: i128,
}

impl InvestmentAllocation {
    /// Splits an investment into project, reserve and commission buckets.
    pub fn from_investment(env: &Env, amount: &i128, i_rate: &u32, decimals: u32) -> Self {
        let rate_denominator = calculate_rate_denominator(amount, decimals);
        let decimals_for_wad: u8 = decimals
            .try_into()
            .expect("Token decimals must fit in u8");

        let amount_wad = Wad::from_token_amount(env, *amount, decimals_for_wad);
        let commission_rate =
            Wad::from_ratio(env, *i_rate as i128, (rate_denominator as i128) * 10_000);
        let reserve_rate = Wad::from_ratio(env, 5, 100);

        let amount_to_commission_wad = amount_wad * commission_rate;
        let amount_to_reserve_fund_wad = amount_wad * reserve_rate;
        let amount_to_invest_wad = amount_wad - amount_to_commission_wad - amount_to_reserve_fund_wad;

        Self {
            amount_to_commission: amount_to_commission_wad.to_token_amount(env, decimals_for_wad),
            amount_to_reserve_fund: amount_to_reserve_fund_wad.to_token_amount(env, decimals_for_wad),
            amount_to_invest: amount_to_invest_wad.to_token_amount(env, decimals_for_wad),
        }
    }
}