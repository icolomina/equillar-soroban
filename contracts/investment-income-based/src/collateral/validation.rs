use super::Collateral;
use crate::require;
use crate::shared::types::Error;

/// Validates collateral deposit preconditions.
///
/// When collateral is already configured, only the same collateral token is
/// accepted. Also enforces source balance sufficiency.
pub(super) fn validate_add_collateral(
    existing_collateral: Option<Collateral>,
    collateral_token_addr_matches: bool,
    collateral_owner_has_balance: bool,
) -> Result<(), Error> {
    if existing_collateral.is_some() {
        require!(
            collateral_token_addr_matches,
            Error::OnlyOneCollateralTokenAllowed
        );
    }

    require!(
        collateral_owner_has_balance,
        Error::AddressInsufficientBalance
    );
    Ok(())
}

/// Validates that there is collateral balance to return.
pub(super) fn validate_collateral_return(collateral_contract_balance: i128) -> Result<(), Error> {
    require!(
        collateral_contract_balance > 0,
        Error::CollateralBalanceIsEmpty
    );
    Ok(())
}