use crate::constants::{SECONDS_IN_MONTH, SECONDS_IN_WEEK};
use crate::investment::Investment;
use soroban_sdk::{contracttype, Env};

#[contracttype]
#[derive(Copy, Clone)]
pub struct Claim {
    pub next_transfer_ts: u64,
    pub amount_to_pay: i128,
}

impl Claim {
    pub fn is_claim_next(&self, env: &Env) -> bool {
        self.next_transfer_ts <= env.ledger().timestamp() + SECONDS_IN_WEEK
    }
}

pub fn calculate_next_claim(e: &Env, investment: &Investment) -> Claim {
    Claim {
        next_transfer_ts: match investment.last_transfer_ts {
            lts if lts > 0 => lts + SECONDS_IN_MONTH,
            _ => e.ledger().timestamp() + SECONDS_IN_MONTH,
        },
        amount_to_pay: investment.regular_payment,
    }
}


pub fn calculate_claimable_payments(env: &Env, investment: &Investment, return_months: u32) -> u32 {
    let now = env.ledger().timestamp();
    let remaining = return_months - investment.payments_transferred;

    let eligible = if investment.last_transfer_ts == 0 {
        let elapsed = now - investment.claimable_ts;
        (elapsed / SECONDS_IN_MONTH) as u32 + 1
    } else {
        let elapsed = now - investment.last_transfer_ts;
        (elapsed / SECONDS_IN_MONTH) as u32
    };

    eligible.min(remaining)
}
