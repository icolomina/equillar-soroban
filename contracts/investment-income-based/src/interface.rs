use soroban_sdk::{contractclient, Address, Env, String};

use crate::collateral::Collateral;
use crate::investment::Investment;
use crate::shared::{ContractBalance, InvestmentContractParams};
use crate::validation::Error;

/// Public interface for the investment-income-based contract.
///
/// This trait is intended to be the stable, human-readable API surface for
/// integrators, auditors, and SDK authors.
///
/// The `contractclient` macro also generates a typed Soroban client that can be
/// used against deployed instances of this contract.
#[contractclient(name = "InvestmentContractClient")]
pub trait InvestmentContractInterface {
    /// Initializes contract configuration and NFT collection metadata.
    ///
    /// Expected caller is the administrative address provided in `owner_addr`
    /// (used as admin in implementation).
    fn __constructor(
        env: Env,
        owner_addr: Address,
        token_addr: Address,
        price_oracle: Address,
        uri: String,
        name: String,
        symbol: String,
        investment_params: InvestmentContractParams,
    ) -> Result<(), Error>;

    /// Creates a new investment position and mints its NFT receipt.
    ///
    /// Requires operator semantics on `addr` at the contract layer.
    fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error>;

    /// Processes one regular payment round for the investment identified by `token_id`.
    fn process_investor_payment(env: Env, token_id: u32) -> Result<Investment, Error>;

    /// Refunds the investor during fundraising and closes the position.
    fn refund_investor(env: Env, token_id: u32) -> Result<i128, Error>;

    /// Returns the current accounting snapshot of the contract balances.
    fn get_contract_balance(env: Env, caller: Address) -> Result<ContractBalance, Error>;

    /// Grants operator role to an address so it can call `invest`.
    fn grant_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// Revokes operator role from an address.
    fn revoke_operator(env: Env, operator: Address) -> Result<(), Error>;

    /// Grants company role to an address.
    ///
    /// Company addresses are accepted as `to`/`from` in treasury and collateral flows.
    fn grant_company(env: Env, company: Address) -> Result<(), Error>;

    /// Revokes company role from an address.
    fn revoke_company(env: Env, company: Address) -> Result<(), Error>;

    /// Grants manager role to an address.
    ///
    /// Manager addresses are valid recipients for commission withdrawals.
    fn grant_manager(env: Env, manager: Address) -> Result<(), Error>;

    /// Revokes manager role from an address.
    fn revoke_manager(env: Env, manager: Address) -> Result<(), Error>;

    /// Starts admin transfer using a pending acceptance window.
    ///
    /// Transfer is completed only after explicit acceptance.
    fn transfer_admin_role(env: Env, new_admin: Address) -> Result<(), Error>;

    /// Accepts a pending admin transfer.
    fn accept_admin_transfer_role(env: Env) -> Result<(), Error>;

    /// Withdraws project funds from the contract to a company-role address.
    fn withdrawn(env: Env, amount: i128, to: Address) -> Result<(), Error>;

    /// Withdraws accumulated commissions to a manager-role address.
    fn withdrawn_commissions(env: Env, to: Address) -> Result<i128, Error>;

    /// Deposits company funds into reserve for the next payment round.
    ///
    /// Returns `true` after successful transfer and accounting updates.
    fn add_company_transfer(env: Env, amount: i128, from: Address) -> Result<bool, Error>;

    /// Activates emergency close mode and snapshots the distributable pool.
    fn activate_emergency_close(env: Env) -> Result<bool, Error>;

    /// Pays an investor proportionally from the frozen emergency pool.
    fn emergency_pay_investor(env: Env, token_id: u32) -> Result<i128, Error>;

    /// Deposits collateral tokens and updates the tracked collateral position.
    ///
    /// `collateral_addr` is expected to be company-role at the contract layer.
    fn add_collateral(
        env: Env,
        collateral_token_addr: Address,
        collateral_token_amount: i128,
        collateral_token_symbol: String,
        collateral_addr: Address,
    ) -> Result<Collateral, Error>;

    /// Settles an investment position using the configured collateral pool.
    ///
    /// Returns the collateral amount transferred to the position owner.
    fn pay_with_collateral(env: Env, token_id: u32) -> Result<i128, Error>;

    /// Returns the remaining collateral balance to the collateral provider.
    fn return_collateral_to_company(env: Env) -> Result<i128, Error>;

    /// Returns whether the contract is currently paused.
    fn paused(env: Env, caller: Address) -> bool;

    /// Pauses operations guarded by the contract pause checks.
    fn pause(env: Env, caller: Address);

    /// Unpauses operations guarded by the contract pause checks.
    fn unpause(env: Env, caller: Address);
}
