// use soroban_sdk::storage::Storage;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env, IntoVal, String, contract, contractimpl, log, token, vec};
use stellar_access::ownable::{self as ownable};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::non_fungible::{Base, NonFungibleToken};

use crate::amounts::Amount;
use crate::balance::ContractBalance;
use crate::collateral::{self, Collateral, calculate_collateral_level};
use crate::data::{ContractData, InvestmentContractParams};
use crate::investment::{Investment, InvestmentReturnType};
use crate::{events, storage};
use crate::validation::{self, Error};
use crate::require;


fn get_token<'a>(env: &'a Env, contract_data: &ContractData) -> TokenClient<'a> {
    token::Client::new(env, &contract_data.token)
}

fn get_collateral_token<'a>(env: &'a Env, collateral_token: &Address) -> TokenClient<'a> {
    token::Client::new(env, collateral_token)
}

#[contract]
pub struct InvestmentContract;

#[contractimpl]
impl InvestmentContract {
    /// Initializes the investment contract.
    ///
    /// Requires authentication from `owner_addr`. Validates all investment parameters,
    /// sets the contract owner, stores `ContractData` derived from `investment_params`,
    /// and initializes the NFT metadata (uri, name, symbol).
    ///
    /// # Parameters
    ///
    /// * `owner_addr` - The contract owner/admin (requires authentication).
    /// * `project_address` - Recipient of withdrawn project funds and co-signer of company transfers.
    /// * `token_addr` - The token used for all investment and payment operations.
    /// * `price_oracle` - Reflector oracle address used to price collateral.
    /// * `uri`, `name`, `symbol` - NFT collection metadata.
    /// * `investment_params` - Struct containing i_rate, fundraising_days, claim_block_days,
    ///   goal, return_type (1=ReverseLoan, 2=Coupon), return_months, min_per_investment.
    ///
    /// # Errors
    ///
    /// * `InterestRateMustBeGreaterThanZero` if `i_rate` is 0.
    /// * `GoalMustBeGreaterThanZero` if `goal` is 0.
    /// * `ReturnMonthsMustBeGreaterThanZero` if `return_months` is 0.
    /// * `MinPerInvestmentMustBeGreaterThanZero` if `min_per_investment` is 0.
    /// * `UnsupportedReturnType` if `return_type` is not 1 or 2.
    pub fn __constructor(
        env: Env,
        owner_addr: Address,
        project_address: Address,
        token_addr: Address,
        price_oracle: Address,
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
            &env,
            &investment_params,
            token_addr,
            project_address,
            price_oracle,
        );

        Base::set_metadata(&env, uri, name, symbol.clone());
        storage::update_contract_data(&env, &contract_data);
        events::emit_contract_deployed_event(
            &env, 
            env.current_contract_address(), 
            symbol.clone(), 
            contract_data.ts_fundraising_ends, 
            contract_data.ts_payments_start
        );
        Ok(())
    }

    /// Pays the current scheduled instalment to the holder of a given investment NFT (owner only).
    ///
    /// Resolves the NFT owner from `token_id`, computes the amount due via
    /// `Investment::process_investment_payment`, validates that the reserve covers the transfer,
    /// and sends tokens from the contract to the investor. For Coupon investments the final round
    /// pays `regular_payment + deposited` (interest + full principal). Marks the investment as
    /// completed when all rounds have been paid.
    ///
    /// # Returns
    ///
    /// * The updated `Investment` with incremented `payments_transferred` and `paid` fields.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `InvestmentCompleted` if all scheduled payments have already been made.
    /// * `PaymentAlreadyProcessedForThisPeriod` if the investor was already paid in the current round.
    /// * `ContractReserveInsufficientBalance` if the reserve is insufficient for this payment.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[only_owner]
    #[when_not_paused]
    pub fn process_investor_payment(env: Env, token_id: u32) -> Result<Investment, Error> {
        let contract_data = storage::get_contract_data(&env);
        let addr = Self::owner_of(&env, token_id);
        let mut investment = storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;
        let mut contract_balance: ContractBalance = storage::get_balances_or_new(&env);

        let tk = get_token(&env, &contract_data);
        let next_payment_round = storage::get_next_payment_round(&env);

        if investment.completed {
            return Err(Error::InvestmentCompleted);
        }
        let amount_to_transfer: i128 = investment.process_investment_payment(&contract_data);

        validation::validate_reserve_balance(amount_to_transfer, &investment, &contract_balance, next_payment_round)?;
        tk.try_transfer(&env.current_contract_address(), &addr, &amount_to_transfer)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        storage::update_contract_data(&env, &contract_data);
        storage::set_investment(&env, token_id, &investment);
        contract_balance.recalculate_from_payment_to_investor(&amount_to_transfer);
        storage::update_contract_balances(&env, &contract_balance);

        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_payment_sent(&env, amount_to_transfer);
        Ok(investment)
    }

    /// Records a new investment from `addr` (requires investor authentication).
    ///
    /// Validates that the fundraising period has not expired, the funding goal has not been reached,
    /// the amount meets the minimum, and the investor has sufficient balance. Transfers `amount`
    /// tokens from the investor to the contract, splits them into project, reserve, and commission
    /// buckets, mints an NFT receipt, and creates an `Investment` record with pre-calculated
    /// interest and payment schedule. Emits a goal-reached event if the goal is met.
    ///
    /// # Returns
    ///
    /// * The newly created `Investment` with all calculated fields (deposited, accumulated_interests,
    ///   total, regular_payment, commission).
    ///
    /// # Errors
    ///
    /// * `FundrasingTimeExceeded` if the fundraising deadline has passed.
    /// * `GoalReached` if the funding goal has already been met.
    /// * `AmountLessThanMinimum` if `amount` is below `min_per_investment`.
    /// * `AddressInsufficientBalance` if the investor does not have enough tokens.
    #[when_not_paused]
    pub fn invest(env: Env, addr: Address, amount: i128) -> Result<Investment, Error> {
        addr.require_auth();
        let mut contract_data: ContractData = storage::get_contract_data(&env);
        let tk = get_token(&env, &contract_data);
        let mut contract_balance = storage::get_balances_or_new(&env);

        validation::validate_investment(amount, &contract_data, tk.balance(&addr), env.ledger().timestamp(), &contract_balance)?;
        let amounts: Amount = Amount::from_investment(&env, &amount, &contract_data.interest_rate, tk.decimals());
        
        tk.try_transfer(&addr, env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let token_id = Base::sequential_mint(&env, &addr);
        let addr_investment = Investment::new(&contract_data, &amounts, token_id);

        storage::set_investment(&env, token_id, &addr_investment);
        contract_balance.recalculate_from_investment(&amounts, &addr_investment);
        contract_data.amount_to_pay_per_month += addr_investment.regular_payment;

        storage::update_contract_data(&env, &contract_data);
        storage::update_contract_balances(&env, &contract_balance);

        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_investment_received_event(&env, amounts.amount_to_invest, addr_investment.accumulated_interests);

        if contract_balance.received_so_far >= contract_data.goal {
            events::emit_goal_reached_event(&env, contract_balance.received_so_far, contract_data.goal);
        }

        Ok(addr_investment)
    }

    /// Returns the current `ContractBalance` snapshot (owner only).
    ///
    /// Exposes the internal balance breakdown: project funds, reserve, commission,
    /// paid-out amount, received_so_far, and payment_obligations.
    #[only_owner]
    pub fn get_contract_balance(env: Env) -> Result<ContractBalance, Error> {
        let contract_balances: ContractBalance = storage::get_balances_or_new(&env);

        Ok(contract_balances)
    }

    /// Transfers project funds from the contract to `project_address`.
    ///
    /// Requires dual authorization: both the owner and `project_address` must sign the exact
    /// `amount`. Validates that the fundraising period has ended and the project balance covers
    /// the withdrawal before executing the transfer.
    ///
    /// # Errors
    ///
    /// * `FundrasingTimeOngoingYet` if the fundraising deadline has not passed yet.
    /// * `ContractInsufficientBalance` if the project balance is less than `amount`.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[when_not_paused]
    pub fn withdrawn(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data =  storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();
        contract_data.project_address.require_auth_for_args(vec![&env, amount.into_val(&env)]);
        owner.require_auth_for_args(vec![&env, amount.into_val(&env)]);

        let mut contract_balance: ContractBalance = storage::get_balances_or_new(&env);
        validation::validate_withdrawal(amount, contract_balance.project, env.ledger().timestamp(), &contract_data)?;

        let tk = get_token(&env, &contract_data);

        tk.try_transfer(
            &env.current_contract_address(),
            &contract_data.project_address,
            &amount,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

        contract_balance.recalculate_from_company_withdrawal(&amount);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_withdrawal_done(&env, amount);

        Ok(true)
    }

    /// Deposits funds into the contract reserve in preparation for the next payment round.
    ///
    /// Requires dual authorization: both the owner and `project_address` must sign the exact
    /// `amount`. Validates that `ts_payments_start` has been reached and that the transfer is
    /// sufficient to cover the upcoming payment obligations. For the final round of a Coupon
    /// investment, the validation requires `reserve + amount >= payment_obligations` to ensure
    /// the full principal return can be funded. Increments `next_payment_round` on success.
    ///
    /// # Errors
    ///
    /// * `NextPaymentCannotBeScheduledYet` if `ts_payments_start` has not been reached.
    /// * `AddressInsufficientBalance` if the owner does not hold enough tokens.
    /// * `ContractReserveInsufficientBalance` if the final Coupon round cannot be fully funded.
    #[when_not_paused]
    pub fn add_company_transfer(env: Env, amount: i128) -> Result<bool, Error> {
        let contract_data =  storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();
        contract_data.project_address.require_auth_for_args(vec![&env, amount.into_val(&env)]);
        owner.require_auth_for_args(vec![&env, amount.into_val(&env)]);

        let mut contract_balance = storage::get_balances_or_new(&env);
        
        let tk = get_token(&env, &contract_data);
        let next_payment_round = storage::get_next_payment_round(&env);
        validation::validate_company_transfer(&env, &tk, &owner, &contract_data, &contract_balance, amount, next_payment_round)?;
        tk.try_transfer(&owner, env.current_contract_address(), &amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        
        contract_balance.recalculate_from_company_contribution(&amount);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_company_transfer_received(&env, amount);
        storage::incr_next_payment_round(&env);

        Ok(true)
    }

    /// Deposits collateral tokens into the contract to back investor obligations.
    ///
    /// Requires authentication from `collateral_addr`. Only one collateral token type is allowed
    /// per contract — attempting to register a different token fails. Transfers `collateral_token_amount`
    /// from `collateral_addr` to the contract and computes the resulting collateral coverage level
    /// via the Reflector price oracle. Rejects the deposit if the resulting level is too low.
    ///
    /// # Returns
    ///
    /// * The updated `Collateral` record including the new `collateral_level`.
    ///
    /// # Errors
    ///
    /// * `OnlyOneCollateralTokenAllowed` if a different collateral token is already registered.
    /// * `AddressInsufficientBalance` if `collateral_addr` does not hold enough collateral tokens.
    /// * `CollateralLevelTooLow` if the oracle-priced coverage is insufficient.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[when_not_paused]
    pub fn add_collateral(
        env: Env,
        collateral_token_addr: Address,
        collateral_token_amount: i128,
        collateral_token_symbol: String,
        collateral_addr: Address,
    ) -> Result<Collateral, Error> {
        collateral_addr.require_auth();

        if let Some(coll) = storage::get_collateral(&env) {
            if coll.token_collateral_address != collateral_token_addr {
                return Err(Error::OnlyOneCollateralTokenAllowed);
            }
        }

        let collateral_token_client = get_collateral_token(&env, &collateral_token_addr);

        if collateral_token_client.balance(&collateral_addr) < collateral_token_amount {
            return Err(Error::AddressInsufficientBalance);
        }

        let current_collateral_token_amount = collateral_token_client.balance(&env.current_contract_address());

        collateral_token_client
            .try_transfer(&collateral_addr, &env.current_contract_address(), &collateral_token_amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let mut contract_balances = storage::get_balances_or_new(&env);
        contract_balances.recalculate_from_collateral_received(&collateral_token_amount);
        storage::update_contract_balances(&env, &contract_balances);

        let contract_data = storage::get_contract_data(&env);
        let contract_token_client = get_token(&env, &contract_data);

        if let Some(level) = calculate_collateral_level(
            &env,
            &contract_data.price_oracle,
            &collateral_token_addr,
            collateral_token_amount + current_collateral_token_amount,
            collateral_token_client.decimals(),
            &contract_data.token,
            contract_token_client.decimals(),
            contract_balances.payment_obligations,
        ) {
            let collateral = Collateral {
                token_collateral_address: collateral_token_addr,
                token_collateral_symbol: collateral_token_symbol,
                address_collateral_token: collateral_addr,
                collateral_amount: collateral_token_amount,
                collateral_level: level
            };
            storage::update_collateral(&env, &collateral);
            events::emit_collateral_deposited(
                &env,
                current_collateral_token_amount,
                collateral_token_amount,
                &collateral
            );
            Ok(collateral)
        } else {
            return Err(Error::CollateralLevelTooLow);
        }
    }

    /// Liquidates collateral and sends the investor's proportional share to the NFT holder.
    ///
    /// Requires dual authorization: both the owner and `project_address` must sign the exact
    /// `token_id`. Computes the investor's pro-rata collateral entitlement based on their
    /// remaining `payment_obligations` share, transfers that amount from the contract to the
    /// NFT owner, and marks the investment as completed.
    ///
    /// # Returns
    ///
    /// * The collateral amount transferred.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `InvestmentCompleted` if the investment is already closed.
    /// * `CollateralNotConfigured` if no collateral has been registered.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[when_not_paused]
    pub fn pay_with_collateral(env: Env, token_id: u32) -> Result<i128, Error> { 
        let contract_data =  storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();
        contract_data.project_address.require_auth_for_args(vec![&env, token_id.into_val(&env)]);
        owner.require_auth_for_args(vec![&env, token_id.into_val(&env)]);

        let mut investment = storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;
        require!(!investment.completed, Error::InvestmentCompleted);

        let collateral = storage::get_collateral(&env).ok_or(Error::CollateralNotConfigured)?;
        let collateral_token = get_collateral_token(&env, &collateral.token_collateral_address);

        let token_owner = Self::owner_of(&env, token_id);
        
        let mut contract_balance: ContractBalance = storage::get_balances_or_new(&env);
        let collateral_amount = collateral::get_collateral_for_investment(
            &env, 
            &investment, 
            &contract_balance, 
            collateral.collateral_amount,
            collateral_token.decimals()
        );

        collateral_token
            .try_transfer(&env.current_contract_address(), &token_owner, &collateral_amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        investment.completed = true;
        storage::set_investment(&env, token_id, &investment);  
        contract_balance.recalculate_from_collateral_liquidated(&collateral_amount);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_collateral_sent(&env, token_owner, collateral_amount);
        Ok(collateral_amount)

    }

    /// Returns the entire remaining collateral balance to the collateral provider (owner only).
    ///
    /// Transfers all collateral tokens held by the contract back to `address_collateral_token`
    /// as recorded in the stored `Collateral`. Intended for use once all investor obligations
    /// have been settled and no collateral is needed any longer.
    ///
    /// # Returns
    ///
    /// * The amount of collateral tokens returned.
    ///
    /// # Errors
    ///
    /// * `CollateralNotConfigured` if no collateral has been registered.
    /// * `CollateralBalanceIsEmpty` if the contract holds no collateral tokens.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[only_owner]
    #[when_not_paused]
    pub fn return_collateral_to_company(env: Env) -> Result<i128, Error> {
        
        if let Some(coll) = storage::get_collateral(&env) {
            let collateral_token = get_collateral_token(&env, &coll.token_collateral_address);

            let collateral_contract_balance = collateral_token.balance(&env.current_contract_address());
            require!(collateral_contract_balance > 0, Error::CollateralBalanceIsEmpty);

            collateral_token.try_transfer(
                &env.current_contract_address(),
                &coll.address_collateral_token,
                &collateral_contract_balance
            ).map_err(|_| Error::RecipientCannotReceivePayment)?
             .map_err(|_| Error::InvalidPaymentData)?;

            let mut contract_balance = storage::get_balances_or_new(&env);
            contract_balance.recalculate_from_collateral_returned(&collateral_contract_balance);
            storage::update_contract_balances(&env, &contract_balance);
            events::emit_collateral_returned(&env, coll.address_collateral_token, collateral_contract_balance);
            Ok(collateral_contract_balance)
        } else {
            return Err(Error::CollateralNotConfigured);
        } 
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
