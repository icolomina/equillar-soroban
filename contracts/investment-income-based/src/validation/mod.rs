use crate::require;
use crate::emergency::EmergencyCloseState;
use crate::shared::types::Error;

pub fn validate_not_in_emergency(
    emergency_state: Option<EmergencyCloseState>,
) -> Result<(), Error> {
    require!(emergency_state.is_none(), Error::OperationNotAllowedInEmergency);
    Ok(())
}

