use soroban_sdk::{Address, Env, String, contractevent};

use crate::{balance::ContractBalance, collateral::Collateral};

#[contractevent]
pub struct ContractBalanceUpdated {
    pub reserve: i128,
    pub project: i128,
    pub comission: i128,
    pub comission_withdrawal: i128,
    pub received_so_far: i128,
    pub payments: i128,
    pub reserve_contributions: i128,
    pub project_withdrawals: i128,
    pub moved_from_project_to_reserve: i128,
    pub payment_obligations: i128,
    pub collateral_received: i128,
    pub collateral_liquidated: i128,
    pub collateral_returned: i128,
    pub refunded_to_investor: i128,
}

#[contractevent]
pub struct ContractDeployed {
    addr: Address,
    symbol: String,
    ts_fundraising_ends: u64,
    ts_payments_start: u64,
}

#[contractevent]
pub struct InvestmentReceived {
    #[topic]
    pub addr: Address,      
    pub deposited: i128,   
    pub capital_gains: i128, 
}

#[contractevent]
pub struct GoalReached {
    #[topic]
    pub addr: Address,
    pub total_received: i128,
    pub goal: i128,
}

#[contractevent]
pub struct CompanyTransferReceived {
    #[topic]
    pub addr: Address,
    pub total_received: i128
}

#[contractevent]
pub struct PaymentSent {
    #[topic]
    pub addr: Address,
    pub total_sent: i128
}

#[contractevent]
pub struct WithdrawalDone {
    #[topic]
    pub addr: Address,
    pub total_withdrawn: i128
}

#[contractevent]
pub struct CollateralDeposited<'a> {
     #[topic]
    pub addr: Address,
    pub current_collateral_amount: i128,
    pub total_deposited: i128,
    pub token_address: &'a Address,
    pub token_symbol: &'a String
}

#[contractevent]
pub struct CollateralSent {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_sent: i128
}

#[contractevent]
pub struct CollateralReturned {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_returned: i128
}

#[contractevent]
pub struct CommissionWithdrawn {
    #[topic]
    pub addr: Address,
    pub amount: i128
}

#[contractevent]
pub struct InvestmentDepositRefunded {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub amount: i128
}

#[contractevent]
pub struct EmergencyPaymentSent {
    #[topic]
    pub addr: Address,
    pub to: Address,
    pub total_sent: i128
}

/// Emits a full contract-balance snapshot.
pub fn emit_balance_updated_event(e: &Env, contract_balance: &ContractBalance) {
    
    ContractBalanceUpdated {
        reserve: contract_balance.reserve,
        project: contract_balance.project,
        comission: contract_balance.comission,
        comission_withdrawal: contract_balance.comission_withdrawal,
        received_so_far: contract_balance.received_so_far,
        payments: contract_balance.payments,
        reserve_contributions: contract_balance.reserve_contributions,
        project_withdrawals: contract_balance.project_withdrawals,
        moved_from_project_to_reserve: contract_balance.moved_from_project_to_reserve,
        payment_obligations: contract_balance.payment_obligations,
        collateral_received: contract_balance.collateral_received,
        collateral_liquidated: contract_balance.collateral_liquidated,
        collateral_returned: contract_balance.collateral_returned,
        refunded_to_investor: contract_balance.refunded_to_investor,
    }.publish(e);
}

/// Emits contract deployment metadata.
pub fn emit_contract_deployed_event(e: &Env, addr: Address, symbol: String, ts_fundraising_ends: u64, ts_payments_start: u64) {
    ContractDeployed {
        addr,
        symbol,
        ts_fundraising_ends,
        ts_payments_start
    }.publish(e);   
}

/// Emits a new-investment event with deposited amount and projected gains.
pub fn emit_investment_received_event(e: &Env, deposited: i128, capital_gains: i128) {
    InvestmentReceived {
        addr: e.current_contract_address(),
        deposited,
        capital_gains,
    }.publish(e);
}

/// Emits an event when fundraising goal is reached.
pub fn emit_goal_reached_event(e: &Env, total_received: i128, goal: i128) {
    GoalReached {
        addr: e.current_contract_address(),
        total_received,
        goal,
    }.publish(e);
}

/// Emits an event for company transfers into reserve.
pub fn emit_company_transfer_received(e: &Env, total_received: i128) {
    CompanyTransferReceived {
        addr: e.current_contract_address(),
        total_received,
    }.publish(e)
}

/// Emits an event for payments sent to investors.
pub fn emit_payment_sent(e: &Env, total_sent: i128) {
    PaymentSent {
        addr: e.current_contract_address(),
        total_sent,
    }.publish(e)
}

/// Emits an event for project withdrawals.
pub fn emit_withdrawal_done(e: &Env, total_withdrawn: i128) {
    WithdrawalDone {
        addr: e.current_contract_address(),
        total_withdrawn,
    }.publish(e)
}

/// Emits collateral deposit details and resulting tracked token metadata.
pub fn emit_collateral_deposited(
    e: &Env, 
    current_collateral_amount: i128, 
    total_deposited: i128, 
    collateral: &Collateral) {

        CollateralDeposited {
            addr: e.current_contract_address(),
            current_collateral_amount,
            total_deposited,
            token_address: &collateral.token_collateral_address,
            token_symbol: &collateral.token_collateral_symbol
        }.publish(e);
}

/// Emits collateral payout sent to an investor.
pub fn emit_collateral_sent(e: &Env, to: Address, total_sent: i128) {
    CollateralSent {
        addr: e.current_contract_address(),
        to,
        total_sent
    }.publish(e);
}

/// Emits collateral return sent back to collateral provider.
pub fn emit_collateral_returned(e: &Env, to: Address, total_returned: i128) {
    CollateralReturned {
        addr: e.current_contract_address(),
        to,
        total_returned
    }.publish(e);
}


/// Emits commission withdrawal amount sent to owner.
pub fn emit_commission_withdrawn(e: &Env, total_withdrawn: i128) {
    CommissionWithdrawn {
        addr: e.current_contract_address(),
        amount: total_withdrawn
    }.publish(e);
}

/// Emits investor refund amount for a refunded investment.
pub fn emit_investment_deposit_refunded(e: &Env, to: Address, total_refunded: i128) {
    InvestmentDepositRefunded {
        addr: e.current_contract_address(),
        to,
        amount: total_refunded
    }.publish(e);
}

/// Emits emergency payout amount sent to an investor.
pub fn emit_emergency_payment_sent(e: &Env, to: Address, total_sent: i128) {
    EmergencyPaymentSent {
        addr: e.current_contract_address(),
        to,
        total_sent,
    }.publish(e);
}





