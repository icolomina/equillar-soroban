use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Symbol};
use stellar_access::access_control::{self as access_control};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{has_role, only_admin, only_role, when_not_paused, when_paused};

use crate::collateral;
use crate::emergency;
use crate::investment;
use crate::payments;
use crate::require;
use crate::treasury;

use crate::shared::types::PositionReturnType;
use crate::shared::{
    self,
    types::{
        ContractBalance, ContractData, Error, InvestmentContractParams, LiquidateInvestmentsStatus,
        Position,
    },
};

#[contract]
pub struct InvestmentContract;

/// Returns the current top-level admin configured in access-control storage.
///
/// This helper is intentionally strict and panics if admin was not initialized,
/// because all sensitive flows depend on constructor initialization.
fn admin(env: &Env) -> Address {
    access_control::get_admin(env).unwrap()
}

/// Requires authentication for callers that are either:
/// - the contract admin, or
/// - holders of at least one configured role.
///
/// This is used by read-oriented endpoints where access should be broader than
/// `only_admin` but still restricted to known actors.
///
/// # Panics
/// Panics when `caller` is neither admin nor role holder.
fn require_admin_or_any_role(env: &Env, caller: &Address) {
    let is_admin = *caller == admin(env);

    if is_admin {
        caller.require_auth();
        return;
    }

    for role in access_control::get_existing_roles(env) {
        if access_control::has_role(env, caller, &role).is_some() {
            caller.require_auth();
            return;
        }
    }

    panic!("Caller is not admin or any role");
}

fn validate_constructor_params(
    i_rate: u32,
    goal: i128,
    return_months: u32,
    min_per_investment: i128,
    cmr_upper_divisor: u32,
    cmr_lower_divisor: u32,
    cmr_reductor: i128,
) -> Result<(), Error> {
    require!(
        i_rate > 0,
        Error::InterestRateMustBeGreaterThanZero,
        goal > 0,
        Error::GoalMustBeGreaterThanZero,
        return_months > 0,
        Error::ReturnMonthsMustBeGreaterThanZero,
        min_per_investment > 0,
        Error::MinPerInvestmentMustBeGreaterThanZero,
        cmr_upper_divisor > 0,
        Error::CmrUpperDividorMustBeGreaterThanZero,
        cmr_lower_divisor > 0,
        Error::CmrLowerDividorMustBeGreaterThanZero,
        cmr_reductor > 0,
        Error::CmrReductorMustBeGreaterThanZero,
        cmr_upper_divisor > cmr_lower_divisor,
        Error::CmrUpperDivisorMustBeGreaterTheCmrLowerDivisor
    );
    Ok(())
}

// Public Soroban entrypoints exported by the contract.
#[contractimpl]
impl InvestmentContract {
    /// Initializes contract configuration and metadata.
    ///
    /// # Access Control
    /// Requires auth from `admin_addr`.
    ///
    /// # Errors
    /// Returns validation errors for constructor params and unsupported return type.
    pub fn __constructor(
        env: Env,
        admin_addr: Address,
        token_addr: Address,
        price_oracle: Address,
        investment_params: InvestmentContractParams,
    ) -> Result<(), Error> {
        admin_addr.require_auth();
        validate_constructor_params(
            investment_params.i_rate,
            investment_params.goal,
            investment_params.return_months,
            investment_params.min_per_investment,
            investment_params.cmr_upper_divisor,
            investment_params.cmr_lower_divisor,
            investment_params.cmr_reductor,
        )?;
        PositionReturnType::from_number(investment_params.return_type)
            .ok_or(Error::UnsupportedReturnType)?;

        access_control::set_admin(&env, &admin_addr);

        let contract_data = ContractData::from_investment_contract_params(
            &env,
            &investment_params,
            token_addr,
            price_oracle,
        );

        shared::storage::update_contract_data(&env, &contract_data);
        shared::events::emit_contract_deployed_event(
            &env,
            env.current_contract_address(),
            contract_data.ts_fundraising_ends,
            contract_data.ts_payments_start,
        );

        Ok(())
    }

    /// Creates an investment position for `addr` and mints its NFT receipt.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - `addr` must hold the `operator` role and authenticate.
    ///
    /// # Errors
    /// Propagates investment validation and transfer errors.
    #[when_not_paused]
    #[only_role(addr, "operator")]
    pub fn invest(env: Env, addr: Address, amount: i128, token_id: u32) -> Result<Position, Error> {
        investment::invest(&env, &addr, amount, token_id)
    }

    /// Enable liquidation payments. It means positions will be liquidated within the contract foreach
    /// payment round
    ///
    /// # Access Control
    /// - Admin only.
    ///
    /// # Errors
    /// Propagates period validations (only within the block period)
    #[only_admin]
    #[when_not_paused]
    pub fn enable_investment_liquidations(env: Env) -> Result<(), Error> {
        payments::enable_liquidate_investments(&env)
    }

    /// Disable liquidation payments. It means positions won't be liquidated within the contract foreach
    /// payment round
    ///
    /// # Access Control
    /// - Admin only.
    ///
    /// # Errors
    /// Propagates period validations (only within the block period)
    #[only_admin]
    #[when_not_paused]
    pub fn disable_investment_liquidations(env: Env) -> Result<(), Error> {
        payments::disable_liquidate_investments(&env)
    }

    /// Check liquidation payments status
    /// payment round
    ///
    /// # Access Control
    /// - Admin only.
    ///
    pub fn check_investment_liquidations(env: Env, caller: Address) -> LiquidateInvestmentsStatus {
        require_admin_or_any_role(&env, &caller);
        payments::check_investment_liquidations(&env)
    }

    /// Processes one scheduled payment for the investment identified by `token_id`.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    #[when_not_paused]
    #[only_admin]
    pub fn process_investor_payment(env: Env, token_id: u32) -> Result<Position, Error> {
        payments::process_investor_payment(&env, token_id)
    }

    /// Refunds an investor during the refund window and closes that position.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn refund_investor(env: Env, token_id: u32) -> Result<i128, Error> {
        investment::refund_investor(&env, token_id)
    }

    /// Returns the current accounting snapshot of contract balances.
    ///
    /// # Access Control
    /// `caller` must authenticate and be either admin or any existing role holder.
    pub fn get_contract_balance(env: Env, caller: Address) -> Result<ContractBalance, Error> {
        require_admin_or_any_role(&env, &caller);
        Ok(shared::storage::get_balances_or_new(&env))
    }

    /// Grants `operator` role to an address.
    ///
    /// Operators are allowed to call investment entrypoints.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn grant_operator(env: Env, operator: Address) -> Result<(), Error> {
        let operator_role = Symbol::new(&env, "operator");
        access_control::grant_role_no_auth(&env, &operator, &operator_role, &admin(&env));
        Ok(())
    }

    /// Revokes `operator` role from an address.
    ///
    /// # Access Control
    /// - Admin only.
    /// - `operator` must currently hold `operator` role.
    #[only_admin]
    #[when_not_paused]
    #[has_role(operator, "operator")]
    pub fn revoke_operator(env: Env, operator: Address) -> Result<(), Error> {
        let operator_role = Symbol::new(&env, "operator");
        access_control::revoke_role_no_auth(&env, &operator, &operator_role, &admin(&env));
        Ok(())
    }

    /// Grants `company` role to an address.
    ///
    /// Company role gates treasury and collateral source addresses.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn grant_company(env: Env, company: Address) -> Result<(), Error> {
        let company_role = Symbol::new(&env, "company");
        access_control::grant_role_no_auth(&env, &company, &company_role, &admin(&env));
        Ok(())
    }

    /// Revokes `company` role from an address.
    ///
    /// # Access Control
    /// - Admin only.
    /// - `company` must currently hold `company` role.
    #[only_admin]
    #[when_not_paused]
    #[has_role(company, "company")]
    pub fn revoke_company(env: Env, company: Address) -> Result<(), Error> {
        let company_role = Symbol::new(&env, "company");
        access_control::revoke_role_no_auth(&env, &company, &company_role, &admin(&env));
        Ok(())
    }

    /// Grants `manager` role to an address.
    ///
    /// Manager role gates commission recipients.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn grant_manager(env: Env, manager: Address) -> Result<(), Error> {
        let manager_role = Symbol::new(&env, "manager");
        access_control::grant_role_no_auth(&env, &manager, &manager_role, &admin(&env));
        Ok(())
    }

    /// Revokes `manager` role from an address.
    ///
    /// # Access Control
    /// - Admin only.
    /// - `manager` must currently hold `manager` role.
    #[only_admin]
    #[when_not_paused]
    #[has_role(manager, "manager")]
    pub fn revoke_manager(env: Env, manager: Address) -> Result<(), Error> {
        let manager_role = Symbol::new(&env, "manager");
        access_control::revoke_role_no_auth(&env, &manager, &manager_role, &admin(&env));
        Ok(())
    }

    /// Initiates the two-step admin transfer process.
    ///
    /// The pending transfer is configured with a one-day acceptance window.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn transfer_admin_role(env: Env, new_admin: Address) -> Result<(), Error> {
        let live_until_ledger = env.ledger().sequence() + 17_280_u32; // 1 day in ledgers (assuming 5s ledger close time)
        access_control::transfer_admin_role(&env, &new_admin, live_until_ledger);
        Ok(())
    }

    /// Accepts an existing pending admin transfer.
    ///
    /// # Access Control
    /// Admin-gated endpoint in this contract facade.
    #[only_admin]
    #[when_not_paused]
    pub fn accept_admin_transfer_role(env: Env) -> Result<(), Error> {
        access_control::accept_admin_transfer(&env);
        Ok(())
    }

    /// Withdraws project funds to `to`.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    /// - `to` must hold `company` role.
    #[when_not_paused]
    #[only_admin]
    #[has_role(to, "company")]
    pub fn withdrawn(env: Env, amount: i128, to: Address) -> Result<(), Error> {
        treasury::withdrawn(&env, amount, &to)
    }

    /// Withdraws accumulated commissions to `to`.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    /// - `to` must hold `manager` role.
    #[when_not_paused]
    #[only_admin]
    #[has_role(to, "manager")]
    pub fn withdrawn_commissions(env: Env, to: Address) -> Result<i128, Error> {
        treasury::withdrawn_commissions(&env, &to)
    }

    /// Withdraws accumulated commissions to `to`.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    /// - `to` must hold `manager` role.
    #[when_not_paused]
    #[only_admin]
    #[has_role(to, "manager")]
    pub fn withdrawn_all(env: Env, to: Address) -> Result<i128, Error> {
        treasury::withdrawn_all(&env, &to)
    }

    /// Registers an inbound transfer from a company address for next payment round.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - `from` must sign the trx.
    /// - `from` must hold `company` role.
    #[when_not_paused]
    #[only_role(from, "company")]
    pub fn add_company_transfer(env: Env, amount: i128, from: Address) -> Result<bool, Error> {
        payments::add_company_transfer(&env, &from, amount)
    }

    /// Freezes contract into emergency mode and snapshots distributable pool.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn activate_emergency_close(env: Env) -> Result<bool, Error> {
        emergency::activate_emergency_close(&env)
    }

    /// Pays one investor from emergency pool according to emergency rules.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    #[only_admin]
    #[when_not_paused]
    pub fn emergency_pay_investor(env: Env, token_id: u32) -> Result<i128, Error> {
        emergency::emergency_pay_investor(&env, token_id)
    }

    /// Deposits collateral token amount and recalculates collateral state.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - `collateral_addr` must sign the trx
    /// - `collateral_addr` must hold `company` role.
    #[when_not_paused]
    #[only_role(collateral_addr, "company")]
    pub fn add_collateral(
        env: Env,
        collateral_token_addr: Address,
        collateral_token_amount: i128,
        collateral_token_symbol: String,
        collateral_addr: Address,
    ) -> Result<u32, Error> {
        collateral::add_collateral(
            &env,
            collateral_token_addr,
            collateral_token_amount,
            collateral_token_symbol,
            collateral_addr,
        )
    }

    /// Settles an investment using available collateral.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    #[when_not_paused]
    #[only_admin]
    pub fn pay_with_collateral(env: Env, token_id: u32) -> Result<i128, Error> {
        collateral::pay_with_collateral(&env, token_id)
    }

    /// Returns remaining collateral balance to configured collateral provider.
    ///
    /// # Access Control
    /// - Contract must not be paused.
    /// - Admin only.
    #[when_not_paused]
    #[only_admin]
    pub fn return_collateral_to_company(env: Env) -> Result<i128, Error> {
        collateral::return_collateral_to_company(&env)
    }

    /// Returns whether the contract is paused.
    ///
    /// # Access Control
    /// Caller must be admin or any role holder and must authenticate.
    pub fn paused(env: &Env, caller: Address) -> bool {
        require_admin_or_any_role(env, &caller);
        pausable::paused(env)
    }

    /// Upgrades the contract code (with new features) without changing the address
    ///
    /// # Access Control
    /// Caller must be admin and contract must be paused
    #[only_admin]
    #[when_paused]
    pub fn upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}

// Pausable extension entrypoints exposed by the contract.
#[contractimpl]
impl Pausable for InvestmentContract {
    /// Pauses guarded operations.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    fn pause(env: &Env, _caller: Address) {
        pausable::pause(env);
    }

    /// Unpauses guarded operations.
    ///
    /// # Access Control
    /// Admin only.
    #[only_admin]
    fn unpause(env: &Env, _caller: Address) {
        pausable::unpause(env);
    }
}
