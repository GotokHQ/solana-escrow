// inside instruction.rs
use solana_program::program_error::ProgramError;
use std::convert::TryInto;

use crate::error::EscrowError::InvalidInstruction;

pub enum EscrowInstruction {
    /// Starts the trade by creating and populating an escrow account and transferring ownership of the given temp token account to the PDA
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the payer initializing the escrow
    /// 1. `[writable]`Temporary token account that should be created prior to this instruction and owned by the payer
    /// 2. `[signer]` The escrow authority responsible for approving / refunding payments due to some external conditions
    /// 3. `[writable]` The escrow account, it will hold all necessary info about the trade.
    /// 4. `[]` The payer token account that will receive the amount if the transaction is canceled
    /// 5. `[]` The payee token account that will receive the amount if the transaction is successful
    /// 6. `[]` The fee token account that will receive the fee if the transaction is successful
    /// 7. `[]` The rent sysvar
    /// 8. `[]` The token program
    InitEscrow {
        /// The total amount of token X to be paid by the payer
        amount: u64,
        /// The fee to collect
        fee: u64,
    },
    /// Settle the payment
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the authority
    /// 1. `[writable]` The taker's token account for the token they will receive should the trade go through
    /// 2. `[writable]` The fee taker's token account for the token they will receive should the trade go through
    /// 3. `[writable]` The PDA's temp token account to get tokens from and eventually close
    /// 4. `[writable]` The fee payer's main account to send their rent fees to
    /// 5. `[writable]` The escrow account holding the escrow info
    /// 6. `[]` The token program
    /// 7. `[]` The PDA account
    Settle,
    /// Cancel the escrow
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the authority
    /// 1. `[writable]` The escrow account holding the escrow info   
    /// 2. `[writable]` The token account of the payer that initialized the escrow  
    /// 3. `[writable]` The fee payer's main account to send their rent fees to
    /// 4. `[writable]` The PDA's temp token account to get tokens from and eventually close
    /// 5. `[]` The token program
    /// 6. `[]` The PDA account
    Cancel,
    /// Close the escrow
    ///
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the authority
    /// 1. `[writable]` The escrow account holding the escrow info     
    /// 2. `[writable]` The fee payer's main account to send their rent fees to
    Close,
}

impl EscrowInstruction {
    /// Unpacks a byte buffer into a [EscrowInstruction](enum.EscrowInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::InitEscrow {
                amount: Self::unpack_amount(rest)?,
                fee: Self::unpack_fee(rest)?,
            },
            1 => Self::Settle,
            2 => Self::Cancel,
            3 => Self::Close,
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction.into())
    }

    fn unpack_fee(input: &[u8]) -> Result<u64, ProgramError> {
        input
            .get(8..16)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction.into())
    }
}
