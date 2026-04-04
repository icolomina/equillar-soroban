// use soroban_sdk::storage::Storage;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env, IntoVal, String, contract, contractimpl, token, vec};
use stellar_access::ownable::{self as ownable};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::{only_owner, when_not_paused};
use stellar_tokens::non_fungible::Base;

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
    /// Initializes contract configuration and NFT metadata.
    ///
    /// Requires `owner_addr` auth, validates core parameters, stores derived
    /// `ContractData`, and sets collection metadata for investment NFTs.
    ///
    /// Uses OpenZeppelin Stellar libraries:
    /// * `Ownable` to set contract owner.
    /// * `stellar_tokens::non_fungible::Base` for NFT metadata.
    ///
    /// # Parameters
    ///
    /// * `owner_addr` - The contract owner/admin (requires authentication).
    /// * `project_address` - Recipient of withdrawn project funds and co-signer for company actions.
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
    ///
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

        // Set the owner using OpenZeppelin Ownable.
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

    /// Pays one scheduled installment to the holder of an investment NFT.
    ///
    /// Owner-only and paused-gated. Resolves NFT holder from `token_id`, computes
    /// the due amount, validates reserve sufficiency for the current round, and
    /// transfers funds from contract balance to the investor. For Coupon, final
    /// round includes principal return (`regular_payment + deposited`).
    ///
    /// # Returns
    ///
    /// * The updated `Investment` with incremented `payments_transferred` and `paid` fields.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `InvestmentCompleted` if all scheduled payments have already been made.
    /// * `PaymentAlreadyProcessedForThisPeriod` if this token was already paid for the current round.
    /// * `ContractReserveInsufficientBalance` if the reserve is insufficient for this payment.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if the token transfer fails.
    #[only_owner]
    #[when_not_paused]
    pub fn process_investor_payment(env: Env, token_id: u32) -> Result<Investment, Error> {
        let contract_data = storage::get_contract_data(&env);
        let addr = Base::owner_of(&env, token_id);
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

        storage::set_investment(&env, token_id, &investment);
        contract_balance.recalculate_from_payment_to_investor(&amount_to_transfer);
        storage::update_contract_balances(&env, &contract_balance);

        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_payment_sent(&env, amount_to_transfer);
        Ok(investment)
    }

    /// Registers a new investment and mints its NFT receipt.
    ///
    /// Investor-authenticated and paused-gated. Validates fundraising timing,
    /// goal/min constraints, and investor balance; then transfers tokens into the
    /// contract, updates accounting buckets (project/reserve/commission), stores
    /// investment state, and mints a sequential NFT.
    ///
    /// NFT minting and ownership tracking are provided by OpenZeppelin
    /// `stellar_tokens::non_fungible::Base`.
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
    ///
    /// # Notes
    ///
    /// * The call can push `received_so_far` above the goal if the current
    ///   investment is accepted and then crosses the threshold.
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

    /// Refunds an investor's original contribution during fundraising.
    ///
    /// Owner-only operation. Transfers `deposited + commission` to the current
    /// owner of `token_id`, marks the investment as completed, and reverses the
    /// corresponding accounting buckets from contract balances.
    ///
    /// # Returns
    ///
    /// * Refunded amount (`deposited + commission`).
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `FundrasingTimeExceeded` if fundraising has already ended.
    /// * `InvestmentCompleted` if the investment is already closed.
    /// * `EmptyRefundAmount` if there is nothing to refund.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if transfer fails.
    ///
    /// # Notes
    ///
    /// * `EmptyRefundAmount` is a defensive guard. Under normal flow, accepted
    ///   investments imply a strictly positive refund amount.
    #[only_owner]
    pub fn refund_investor(env: Env, token_id: u32) -> Result<i128, Error> {
        let mut investment = storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;
        let contract_data: ContractData = storage::get_contract_data(&env);
        let tk = get_token(&env, &contract_data);
        let mut contract_balance = storage::get_balances_or_new(&env);

        let amount_to_refund = investment.get_amount_to_refund();
        let investment_owner_addr = Base::owner_of(&env, token_id);
        validation::validate_refund_investor(&investment, &contract_data, amount_to_refund, env.ledger().timestamp())?;
        tk.try_transfer(&env.current_contract_address(), &investment_owner_addr,  &amount_to_refund)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        investment.completed = true;
        contract_balance.recalculate_from_refunded_to_investor(&investment);
        storage::set_investment(&env, token_id, &investment);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_investment_deposit_refunded(&env, investment_owner_addr, amount_to_refund);

        Ok(amount_to_refund)
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
    /// # Returns
    ///
    /// * `true` when transfer and accounting updates succeed.
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

    /// Withdraws accumulated commission to the contract owner.
    ///
    /// Owner-only and paused-gated. Computes pending commission as
    /// `comission - comission_withdrawal`, validates time window and available
    /// amount, then transfers pending commission and updates withdrawal counters.
    ///
    /// # Returns
    ///
    /// * Amount withdrawn in this call.
    ///
    /// # Errors
    ///
    /// * `FundrasingTimeOngoingYet` if fundraising has not ended.
    /// * `ContractInsufficientBalance` if no pending commission is available.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if transfer fails.
    #[when_not_paused]
    #[only_owner]
    pub fn withdrawn_commissions(env: Env) -> Result<i128, Error> {
        let contract_data =  storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();
        let mut contract_balance: ContractBalance = storage::get_balances_or_new(&env);

        let tk = get_token(&env, &contract_data);
        let amount_to_withdrawn = contract_balance.comission - contract_balance.comission_withdrawal;
        validation::validate_withdrawal_commission(amount_to_withdrawn, env.ledger().timestamp(), &contract_data)?;

        tk.try_transfer(
            &env.current_contract_address(),
            &owner,
            &amount_to_withdrawn,
        )
        .map_err(|_| Error::RecipientCannotReceivePayment)?
        .map_err(|_| Error::InvalidPaymentData)?;

        contract_balance.recalculate_from_comission_withdrawal(&amount_to_withdrawn);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_commission_withdrawn(&env, amount_to_withdrawn);

        Ok(amount_to_withdrawn)
    }

    /// Deposits company funds into reserve for the next payment round.
    ///
    /// Requires dual authorization: both the owner and `project_address` must sign the exact
    /// `amount`. Validates that `ts_payments_start` has been reached and that the transfer is
    /// sufficient for upcoming obligations. For final Coupon round, it enforces
    /// `reserve + amount >= payment_obligations` so principal return is coverable.
    /// Increments `next_payment_round` after a successful transfer.
    ///
    /// # Errors
    ///
    /// * `NextPaymentCannotBeScheduledYet` if `ts_payments_start` has not been reached.
    /// * `OwnerInsufficientBalance` if the owner does not hold enough tokens.
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

    /// Deposits collateral tokens to back investor obligations.
    ///
    /// Requires authentication from `collateral_addr`. Only one collateral token type is allowed
    /// per contract. Transfers `collateral_token_amount` to contract custody and
    /// computes updated collateral level via the oracle.
    ///
    /// # Returns
    ///
    /// * The updated `Collateral` record including the new `collateral_level`.
    ///
    /// # Errors
    ///
    /// * `OnlyOneCollateralTokenAllowed` if a different collateral token is already registered.
    /// * `AddressInsufficientBalance` if `collateral_addr` does not hold enough collateral tokens.
    /// * `CollateralLevelTooLow` if resulting oracle-priced coverage is insufficient.
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
        let total_cllateral_amount = collateral_token_amount + current_collateral_token_amount;

        if let Some(level) = calculate_collateral_level(
            &env,
            &contract_data.price_oracle,
            &collateral_token_addr,
            total_cllateral_amount,
            collateral_token_client.decimals(),
            &contract_data.token,
            contract_token_client.decimals(),
            contract_balances.payment_obligations,
        ) {
            let collateral = Collateral {
                token_collateral_address: collateral_token_addr,
                token_collateral_symbol: collateral_token_symbol,
                address_collateral_token: collateral_addr,
                collateral_amount: total_cllateral_amount,
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

    /// Liquidates collateral and sends a proportional share to the NFT holder.
    ///
    /// Requires dual authorization (owner + `project_address`). Computes the
    /// investor's pro-rata collateral entitlement from remaining obligations,
    /// transfers collateral tokens, and marks the investment as completed.
    ///
    /// # Returns
    ///
    /// * Collateral amount transferred.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `InvestmentCompleted` if the investment is already closed.
    /// * `CollateralNotConfigured` if no collateral has been registered.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if transfer fails.
    ///
    /// # Edge cases
    ///
    /// * Due to integer division, calculated payout may be 0 for very small shares;
    ///   the investment is still marked completed.
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

        let token_owner = Base::owner_of(&env, token_id);
        
        let mut contract_balance: ContractBalance = storage::get_balances_or_new(&env);
        let current_collateral_balance = collateral_token.balance(&env.current_contract_address());
        let collateral_amount = collateral::get_collateral_for_investment(
            &env, 
            &investment, 
            &contract_balance, 
            current_collateral_balance,
            collateral_token.decimals()
        );

        collateral_token
            .try_transfer(&env.current_contract_address(), &token_owner, &collateral_amount)
            .map_err(|_| Error::RecipientCannotReceivePayment)?
            .map_err(|_| Error::InvalidPaymentData)?;

        let remaining_obligations = investment.total - investment.paid;
        investment.completed = true;
        storage::set_investment(&env, token_id, &investment);  
        contract_balance.recalculate_from_collateral_liquidated(&collateral_amount, &remaining_obligations);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_collateral_sent(&env, token_owner, collateral_amount);
        Ok(collateral_amount)

    }

    /// Distributes an investor's proportional reserve share in default mode.
    ///
    /// Computes pro-rata entitlement as
    /// `remaining_obligations * reserve / payment_obligations`.
    /// Marks the investment as completed and reduces `payment_obligations` accordingly, ensuring
    /// every subsequent call receives a correct slice of the remaining reserve.
    ///
    /// # Returns
    ///
    /// * Amount transferred to the investor.
    ///
    /// # Errors
    ///
    /// * `AddressHasNotInvested` if no investment exists for `token_id`.
    /// * `FundrasingTimeOngoingYet` if the fundraising deadline has not passed yet.
    /// * `InvestmentCompleted` if the investment is already closed.
    /// * `EmptyReserve` if the reserve is empty.
    /// * `EmptyPaymentObligations` if there are no pending obligations.
    /// * `RecipientCannotReceivePayment` or `InvalidPaymentData` if transfer fails.
    ///
    /// # Edge cases
    ///
    /// * Integer division may round payout down to 0 for tiny shares. In that case,
    ///   no token transfer is executed, but the investment is still marked completed.
    #[when_not_paused]
    pub fn emergency_pay_investor(env: Env, token_id: u32) -> Result<i128, Error> {
        let contract_data = storage::get_contract_data(&env);
        let owner = ownable::get_owner(&env).unwrap();
        contract_data.project_address.require_auth_for_args(vec![&env, token_id.into_val(&env)]);
        owner.require_auth_for_args(vec![&env, token_id.into_val(&env)]);

        let mut investment = storage::get_investment(&env, token_id).ok_or(Error::AddressHasNotInvested)?;
        let mut contract_balance = storage::get_balances_or_new(&env);
        validation::validate_emergency_payment(&investment, &contract_balance, env.ledger().timestamp(), &contract_data)?;

        let remaining_obligations = investment.total - investment.paid;
        let amount_to_pay = remaining_obligations * contract_balance.reserve / contract_balance.payment_obligations;

        let token_owner = Base::owner_of(&env, token_id);
        let tk = get_token(&env, &contract_data);

        if amount_to_pay > 0 {
            tk.try_transfer(&env.current_contract_address(), &token_owner, &amount_to_pay)
                .map_err(|_| Error::RecipientCannotReceivePayment)?
                .map_err(|_| Error::InvalidPaymentData)?;
        }

        investment.completed = true;
        storage::set_investment(&env, token_id, &investment);
        contract_balance.recalculate_from_emergency_payment(&amount_to_pay, &remaining_obligations);
        storage::update_contract_balances(&env, &contract_balance);
        events::emit_balance_updated_event(&env, &contract_balance);
        events::emit_emergency_payment_sent(&env, token_owner, amount_to_pay);
        Ok(amount_to_pay)
    }

    /// Returns all remaining collateral balance to the configured collateral provider.
    ///
    /// Owner-only and paused-gated. Transfers full collateral-token custody from
    /// the contract to `address_collateral_token` recorded in `Collateral`.
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

#[contractimpl]
impl Pausable for InvestmentContract {
    /// Returns whether the contract is currently paused.
    ///
    /// Owner-only view helper exposed through OpenZeppelin's `Pausable` trait.
    #[only_owner]
    fn paused(e: &Env) -> bool {
        pausable::paused(e)
    }

    /// Pauses contract operations guarded by `#[when_not_paused]`.
    ///
    /// Owner-only state transition backed by OpenZeppelin `Pausable`.
    #[only_owner]
    fn pause(e: &Env, _caller: Address) {
        pausable::pause(e);
    }

    /// Unpauses the contract and re-enables `#[when_not_paused]` operations.
    ///
    /// Owner-only state transition backed by OpenZeppelin `Pausable`.
    #[only_owner]
    fn unpause(e: &Env, _caller: Address) {
        pausable::unpause(e);

    }
}
