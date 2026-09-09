use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The escrow has expired")]
    EscrowExpired,
    #[msg("The escrow has not expired yet")]
    EscrowNotExpired,
    #[msg("The expiration must be in the future")]
    InvalidExpiration,
}
