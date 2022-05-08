use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

pub struct Escrow {
    pub is_initialized: bool,
    pub is_settled: bool,
    pub is_canceled: bool,
    pub payer: Pubkey,
    pub payer_token: Pubkey,
    pub payee_token: Pubkey,
    pub vault_token: Pubkey,
    pub fee_token: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

impl Escrow {
    pub fn is_settled(&self) -> bool {
        self.is_settled
    }
    pub fn is_canceled(&self) -> bool {
        self.is_canceled
    }
}

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

impl Pack for Escrow {
    const LEN: usize = 211;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Escrow::LEN];
        let (
            is_initialized,
            is_settled,
            is_canceled,
            payer,
            payer_token,
            payee_token,
            vault_token,
            authority,
            fee_token,
            amount,
            fee,
        ) = array_refs![src, 1, 1, 1, 32, 32, 32, 32, 32, 32, 8, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        let is_settled = match is_settled {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        let is_canceled = match is_canceled {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Escrow {
            is_initialized,
            is_settled,
            is_canceled,
            payer: Pubkey::new_from_array(*payer),
            payer_token: Pubkey::new_from_array(*payer_token),
            payee_token: Pubkey::new_from_array(
                *payee_token,
            ),
            vault_token: Pubkey::new_from_array(
                *vault_token,
            ),
            authority: Pubkey::new_from_array(*authority),
            fee_token: Pubkey::new_from_array(*fee_token),
            amount: u64::from_le_bytes(*amount),
            fee: u64::from_le_bytes(*fee),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Escrow::LEN];
        let (
            is_initialized_dst,
            is_settled_dst,
            is_canceled_dst,
            payer_pubkey_dst,
            payer_token_account_pubkey_dst,
            payee_token_account_pubkey_dst,
            payer_temp_token_account_pubkey_dst,
            authority_pubkey_dst,
            fee_taker_pubkey_dst,
            expected_amount_dst,
            expected_fees_dst,
        ) = mut_array_refs![dst, 1, 1, 1, 32, 32, 32, 32, 32, 32, 8, 8];

        let Escrow {
            is_initialized,
            is_settled,
            is_canceled,
            payer,
            payer_token,
            payee_token,
            vault_token,
            authority,
            fee_token,
            amount,
            fee,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        is_settled_dst[0] = *is_settled as u8;
        is_canceled_dst[0] = *is_canceled as u8;
        payer_pubkey_dst.copy_from_slice(payer.as_ref());
        payer_token_account_pubkey_dst
            .copy_from_slice(payer_token.as_ref());
        payee_token_account_pubkey_dst
            .copy_from_slice(payee_token.as_ref());
        payer_temp_token_account_pubkey_dst
            .copy_from_slice(vault_token.as_ref());
        authority_pubkey_dst.copy_from_slice(authority.as_ref());
        fee_taker_pubkey_dst.copy_from_slice(fee_token.as_ref());
        *expected_amount_dst = amount.to_le_bytes();
        *expected_fees_dst = fee.to_le_bytes();
    }
}

impl Sealed for Escrow {}

impl IsInitialized for Escrow {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
