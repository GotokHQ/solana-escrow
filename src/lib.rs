pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;


pub const PREFIX: &str = "escrow";

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

use solana_program::{
    declare_id, entrypoint::ProgramResult, pubkey::Pubkey,
};

declare_id!("escJ4uwy5ndByWNK2UpmHptAYCQahfKLXBbRVBR17fX");

/// Generates program authority
pub fn find_program_authority(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PREFIX.as_bytes(), program_id.as_ref()], program_id)
}

/// Checks that the supplied authority ID is the correct one for SPL-token
pub fn check_authority_account(escrow_authority_id: &Pubkey) -> ProgramResult {
    if escrow_authority_id != &id() {
        return Err(error::EscrowError::InvalidAuthorityId.into());
    }
    Ok(())
}
