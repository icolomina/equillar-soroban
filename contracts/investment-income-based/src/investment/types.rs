use soroban_sdk::contracttype;

/// NFT-backed investment position state.
///
/// Tracks principal split, commission, accrued interests, payment progress,
/// completion status, and NFT token id.
#[contracttype]
#[derive(Copy, Clone)]
pub struct Investment {
    pub deposited: i128,
    pub amount_invested: i128,
    pub commission: i128,
    pub accumulated_interests: i128,
    pub total: i128,
    pub completed: bool,
    pub regular_payment: i128,
    pub paid: i128,
    pub payments_transferred: u32,
    pub token_id: u32,
}

#[derive(Copy, Clone)]
pub struct DepositAllocation {
    pub commission: i128,
    pub returns: i128,
    pub deposited: i128
}

impl DepositAllocation {
    pub fn get_total_claimable(self) -> i128 {
        let total = self.deposited + self.returns;
        total
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
#[contracttype]
/// Repayment profile used by investment positions.
pub enum InvestmentReturnType {
    ReverseLoan = 1,
    Coupon = 2,
}

impl InvestmentReturnType {
    /// Parses numeric return type code into enum variant.
    pub fn from_number<N>(value: N) -> Option<Self>
    where
        N: Into<u32>,
    {
        match value.into() {
            1 => Some(Self::ReverseLoan),
            2 => Some(Self::Coupon),
            _ => None,
        }
    }
}
