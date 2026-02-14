use soroban_sdk::token::TokenClient;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Map, String};
use stellar_access::ownable::{self as ownable};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::non_fungible::{Base, NonFungibleToken};

use crate::balance::{Amount, CalculateAmounts, ContractBalance};
use crate::claim::{calculate_claimable_payments, Claim};
use crate::data::{ContractData, FromNumber, InvestmentContractParams, State};
use crate::investment::{Investment, InvestmentReturnType};
use crate::validation::{self, Error};

use crate::{require, storage as Storage};

fn get_token<'a>(env: &'a Env, contract_data: &ContractData) -> TokenClient<'a> {
    token::Client::new(env, &contract_data.token)
}

#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {
    /// Initializes the investment contract with configuration parameters.
    ///
    /// Sets up the contract with admin authentication, token configuration, investment rules,
    /// and return structure. The contract starts in 'Active' state.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment provided by Soroban.
    /// * `admin_addr` - The contract administrator's address (requires authentication).
    /// * `project_address` - The address that will receive withdrawn project funds.
    /// * `token_addr` - The token contract address used for all transactions.
    /// * `i_rate` - The interest rate percentage (must be > 0).
    /// * `claim_block_days` - Days investors must wait before claiming returns.
    /// * `goal` - The total funding goal (must be > 0).
    /// * `return_type` - The return model: 1=ReverseLoan, 2=Coupon.
    /// * `return_months` - Number of months for return payments (must be > 0).
    /// * `min_per_investment` - Minimum investment amount (must be > 0).
    ///
    /// # Errors
    ///
    /// * `InterestRateMustBeGreaterThanZero` if i_rate is 0.
    /// * `GoalMustBeGreaterThanZero` if goal is 0.
    /// * `ReturnMonthsMustBeGreaterThanZero` if return_months is 0.
    /// * `MinPerInvestmentMustBeGreaterThanZero` if min_per_investment is 0.
    /// * `UnsupportedReturnType` if return_type is not 1 or 2.
    pub fn __constructor(
        env: Env,
        owner_addr: Address,
        project_address: Address,
        token_addr: Address,
        uri: String,
        name: String,
        symbol: String,
        investment_params: InvestmentContractParams,
    ) -> Result<(), Error> {
        owner_addr.require_auth();
        validation::validate_constructor_params(
            investment_params.i_rate,
            investment_params.goal,
            investment_params.return_months,
            investment_params.min_per_investment,
        )?;
        InvestmentReturnType::from_number(investment_params.return_type).ok_or(Error::UnsupportedReturnType)?;

        // Set the owner using OpenZeppelin Ownable
        ownable::set_owner(&env, &owner_addr);
        let contract_data = ContractData::from_investment_contract_params(
            &investment_params,
            token_addr,
            project_address,
        );

        Base::set_metadata(&env, uri, name, symbol);
        Storage::update_contract_data(&env, &contract_data);
        Ok(())
    }

    /// Processes a scheduled payment to an investor (admin only).
    ///
    /// Transfers the regular payment amount from the contract's reserve balance to the investor.
    /// Updates investment status, payment tracking, and claim schedules. Validates timing constraints
    /// to ensure payments are made according to the investment schedule.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address receiving the payment.
    /// * `ts` - The claimable timestamp identifying the specific investment.
    ///
    /// # Returns
    ///
    /// * The updated `Investment` object with incremented payment counters.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for this address and timestamp.
    /// * `AddressInvestmentIsNotClaimableYet` if the claimable date hasn't been reached.
    /// * `AddressInvestmentIsFinished` if all payments have been completed.
    /// * `AddressInvestmentNextTransferNotClaimableYet` if less than a month has passed since last payment.
    /// * `ContractInsufficientBalance` if reserve balance is insufficient.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if token transfer fails.
    #[only_owner]
    #[when_not_paused]
    pub fn process_investor_payment(env: Env, token_id: u32) -> Result<Investment, Error> {
        let contract_data = Storage::get_contract_data(&env);
        let addr = Self::owner_of(&env, token_id);
        let mut investment =Storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;

        validation::validate_investment_payment(&env, &investment)?;

        let mut contract_balances: ContractBalance = Storage::get_balances_or_new(&env);
        let tk = get_token(&env, &contract_data);
        let amount_to_transfer: i128 = investment.process_investment_payment(&env, &contract_data);

        validation::validate_reserve_balance(amount_to_transfer, &contract_balances)?;
        tk.try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        Storage::update_investment_with_claim(&env, token_id, &investment);
        contract_balances.recalculate_from_payment_to_investor(&amount_to_transfer);
        Storage::update_contract_balances(&env, &contract_balances);

        contract_balances.emit_event(&env);
        Ok(investment)
    }

    //pub fn claim(end: Env, addr: Address)

    /// Allows an investor to make a new investment.
    ///
    /// Validates the investment amount, contract state, and funding goal constraints.
    /// Transfers tokens from the investor to the contract, splits them into project and reserve balances,
    /// creates the investment record with calculated returns, and updates the contract state.
    /// If the funding goal is reached, changes contract state to 'FundsReached'.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address (requires authentication).
    /// * `amount` - The investment amount in tokens.
    ///
    /// # Returns
    ///
    /// * The created `Investment` object with all calculated fields.
    ///
    /// # Errors
    ///
    /// * `AmountLessThanMinimum` if amount is below the minimum per investment.
    /// * `GoalAlreadyReached` if the funding goal has already been reached.
    /// * `AddressInsufficientBalance` if investor doesn't have enough tokens.
    /// * `WouldExceedGoal` if this investment would exceed the funding goal.
    ///
    /// # Note
    ///
    /// * The `#[when_not_paused]` macro automatically rejects calls if the contract is paused.
    #[when_not_paused]
    pub fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error> {
        addr.require_auth();
        let mut contract_data: ContractData = Storage::get_contract_data(&env);
        let tk = get_token(&env, &contract_data);

        validation::validate_investment(amount, &contract_data, tk.balance(&addr))?;

        let token_decimals: u8 = tk
            .decimals()
            .try_into()
            .expect("Token decimals must fit in u8")
        ;
        let amounts: Amount = Amount::from_investment(&env, &amount, &contract_data.interest_rate, token_decimals);

        // Validate goal before transfer
        let mut contract_balance = Storage::get_balances_or_new(&env);
        validation::validate_investment_goal(
            contract_balance.received_so_far,
            amounts.get_invested_amount(),
            contract_data.goal,
        )?;

        tk.try_transfer(&addr, env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        contract_balance.recalculate_from_investment(&amounts);
        Storage::update_contract_balances(&env, &contract_balance);

        let token_id = Base::sequential_mint(&env, &addr);
        let addr_investment =Investment::new(&env, &contract_data, &amount, token_decimals, token_id);
        Storage::update_investment_with_claim(&env, token_id, &addr_investment);

        if contract_balance.received_so_far >= contract_data.goal {
            contract_data.state = State::FundsReached;
            Storage::update_contract_data(&env, &contract_data);
            contract_data.state.emit_event(&env);
        }

        contract_balance.emit_event(&env);

        Ok(addr_investment)
    }

    /// Retrieves the current contract balances (admin only).
    ///
    /// Returns the breakdown of contract funds across different balance categories:
    /// project balance, reserve balance, commission, and total received.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * `ContractBalances` containing all balance information.
    #[only_owner]
    pub fn get_contract_balance(env: Env) -> Result<ContractBalance, Error> {
        let contract_balances: ContractBalance = Storage::get_balances_or_new(&env);

        Ok(contract_balances)
    }

    /// Withdraws funds from the project balance to the project address (admin only).
    ///
    /// Transfers the specified amount from the contract's project balance to the configured
    /// project address. Validates sufficient balance and updates internal accounting.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to withdraw from project balance.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `ContractInsufficientBalance` if project balance is less than the requested amount.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the transfer fails.
    #[only_owner]
    #[when_not_paused]
    pub fn single_withdrawn(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data = Storage::get_contract_data(&env);

        let mut contract_balances: ContractBalance = Storage::get_balances_or_new(&env);
        validation::validate_withdrawal(amount, contract_balances.project)?;

        let tk = get_token(&env, &contract_data);

        // Verify the transfer can be completed
        tk.try_transfer(
            &env.current_contract_address(),
            &contract_data.project_address,
            &amount,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

        //decrement_project_balance_from_company_withdrawal(&mut contract_balances, &amount);
        contract_balances.recalculate_from_company_withdrawal(&amount);
        Storage::update_contract_balances(&env, &contract_balances);
        contract_balances.emit_event(&env);

        Ok(true)
    }

    /// Calculates additional funds needed in reserve balance (admin only).
    ///
    /// Analyzes upcoming payment claims (within the next week) and compares them against
    /// the current reserve balance to determine if additional funds are needed.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    ///
    /// # Returns
    ///
    /// * The additional amount needed in reserve, or 0 if reserve is sufficient.
    #[only_owner]
    pub fn check_reserve_balance(env: Env) -> Result<i128, Error> {
        let claims_map: Map<u32, Claim> = Storage::get_claims_map_or_new(&env);
        let project_balances: ContractBalance = Storage::get_balances_or_new(&env);
        let mut min_funds: i128 = 0;

        for (_, next_claim) in claims_map.iter() {
            if next_claim.is_claim_next(&env) {
                min_funds += next_claim.amount_to_pay;
            }
        }

        if min_funds > 0 && project_balances.reserve < min_funds {
            let diff_to_contribute: i128 = min_funds - project_balances.reserve;
            return Ok(diff_to_contribute);
        }

        Ok(0_i128)
    }

    /// Adds funds from admin to the contract's reserve balance (admin only).
    ///
    /// Transfers tokens from the admin address to the contract and adds them to the reserve balance.
    /// This is used to replenish the reserve fund for upcoming investor payments.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to transfer to reserve.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `AddressInsufficientBalance` if admin doesn't have enough tokens.
    #[only_owner]
    pub fn add_company_transfer(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data = Storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();

        let tk = get_token(&env, &contract_data);
        validation::validate_company_transfer(&tk, &owner, amount)?;
        tk.try_transfer(&owner, env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let mut contract_balances = Storage::get_balances_or_new(&env);
        contract_balances.recalculate_from_company_contribution(&amount);
        Storage::update_contract_balances(&env, &contract_balances);
        contract_balances.emit_event(&env);

        Ok(true)
    }

    /// Moves funds from project balance to reserve balance (admin only).
    ///
    /// Transfers the specified amount internally from the project balance to the reserve balance.
    /// This is used to ensure sufficient reserve funds for upcoming investor payments.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `amount` - The amount to move from project to reserve.
    ///
    /// # Returns
    ///
    /// * `true` on success.
    ///
    /// # Errors
    ///
    /// * `ProjectBalanceInsufficientAmount` if project balance is less than the requested amount.
    #[only_owner]
    pub fn move_funds_to_the_reserve(env: Env, amount: i128) -> Result<bool, Error> {
        let mut contract_balances = Storage::get_balances_or_new(&env);
        validation::validate_move_to_reserve(amount, contract_balances.project)?;

        contract_balances.recalculate_from_project_to_reserver_movement(&amount);
        Storage::update_contract_balances(&env, &contract_balances);
        contract_balances.emit_event(&env);

        Ok(true)
    }

    /// Allows an investor to claim all their pending payment periods at once.
    ///
    /// Unlike `process_investor_payment` (admin-only, single payment), this function is
    /// called by the investor themselves. It calculates how many monthly payments have
    /// become available since the last claim (or since the claimable date for first-time
    /// claims) and transfers the accumulated amount in a single operation.
    ///
    /// For example, if an investor hasn't claimed for 3 months, they receive 3 × regular_payment.
    ///
    /// # Parameters
    ///
    /// * `env` - The execution environment.
    /// * `addr` - The investor's address (requires authentication).
    /// * `ts` - The claimable timestamp identifying the specific investment.
    ///
    /// # Returns
    ///
    /// * The updated `Investment` object with incremented payment counters.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for this address and timestamp.
    /// * `AddressInvestmentIsNotClaimableYet` if the claimable date hasn't been reached.
    /// * `AddressInvestmentIsFinished` if all payments have been completed.
    /// * `AddressInvestmentNextTransferNotClaimableYet` if no full payment periods have elapsed.
    /// * `ContractInsufficientBalance` if reserve balance is insufficient.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if token transfer fails.
    #[when_not_paused]
    pub fn claim(env: Env, token_id: u32) -> Result<Investment, Error> {
        let addr: Address = Self::owner_of(&env, token_id);
        addr.require_auth();

        let contract_data = Storage::get_contract_data(&env);
        let mut investment =Storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;

        validation::validate_claim(&env, &investment)?;

        let num_payments =calculate_claimable_payments(&env, &investment, contract_data.return_months);
        require!(
            num_payments > 0,
            Error::AddressInvestmentNextTransferNotClaimableYet
        );

        let mut contract_balances = Storage::get_balances_or_new(&env);
        let amount_to_transfer = investment.process_multiple_payments(&env, &contract_data, num_payments);

        validation::validate_reserve_balance(amount_to_transfer, &contract_balances)?;

        let tk = get_token(&env, &contract_data);
        tk.try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        Storage::update_investment_with_claim(&env, token_id, &investment);
        contract_balances.recalculate_from_payment_to_investor(&amount_to_transfer);
        Storage::update_contract_balances(&env, &contract_balances);

        contract_balances.emit_event(&env);
        Ok(investment)
    }
}

#[contractimpl(contracttrait)]
impl NonFungibleToken for InvestmentContract {
    type ContractType = Base;
}

#[contractimpl]
impl Pausable for InvestmentContract {
    #[only_owner]
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    #[only_owner]
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    #[only_owner]
    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);
    }
}
