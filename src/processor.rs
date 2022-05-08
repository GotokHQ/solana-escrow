use crate::{
    error::EscrowError::{
        AccountAlreadyCanceled, AccountAlreadySettled, AccountNotSettledOrCanceled, AmountOverflow,
        ExpectedAmountMismatch, FeeOverflow,
    },
    PREFIX,
    find_program_authority,
    instruction::EscrowInstruction,
    state::Escrow,
    utils::{assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_initialized},
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::state::Account as TokenAccount;

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = EscrowInstruction::unpack(instruction_data)?;

        match instruction {
            EscrowInstruction::InitEscrow { amount, fee } => {
                msg!("Instruction: InitEscrow");
                Self::process_init_escrow(accounts, amount, fee, program_id)
            }
            EscrowInstruction::Settle => {
                msg!("Instruction: Settle");
                Self::process_settlement(accounts, program_id)
            }
            EscrowInstruction::Cancel => {
                msg!("Instruction: Cancel");
                Self::process_cancel(accounts, program_id)
            }
            EscrowInstruction::Close => {
                msg!("Instruction: Close");
                Self::process_close(accounts, program_id)
            }
        }
    }

    fn process_init_escrow(
        accounts: &[AccountInfo],
        amount: u64,
        fee: u64,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_info = next_account_info(account_info_iter)?;
        assert_signer(payer_info)?;
        let vault_token_info = next_account_info(account_info_iter)?;
        assert_owned_by(vault_token_info, &spl_token::id())?;
        let vault_token =
            TokenAccount::unpack(&vault_token_info.data.borrow())?;
        if vault_token.amount != amount {
            msg!(
                "Got Mismatched amount..., got: {} , expected {}",
                amount,
                vault_token.amount
            );
            return Err(ExpectedAmountMismatch.into());
        }

        let authority_info = next_account_info(account_info_iter)?;
        assert_signer(authority_info)?;

        let escrow_info = next_account_info(account_info_iter)?;
        let payer_token_info = next_account_info(account_info_iter)?;
        let payee_token_info = next_account_info(account_info_iter)?;
        let fee_token_info = next_account_info(account_info_iter)?;
        if vault_token.is_native() {
            assert_account_key(payer_token_info, payer_info.key)?;
        } else {
            assert_owned_by(payer_token_info, &spl_token::id())?;
            assert_owned_by(payee_token_info, &spl_token::id())?;
            assert_owned_by(fee_token_info, &spl_token::id())?;
            let _: TokenAccount = assert_initialized(payer_token_info)?;
            let _: TokenAccount = assert_initialized(payee_token_info)?;
            let _: TokenAccount = assert_initialized(fee_token_info)?;
        }

        let rent_info = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        assert_rent_exempt(rent_info, escrow_info)?;

        let mut escrow = Escrow::unpack_unchecked(&escrow_info.data.borrow())?;
        if escrow.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if fee > amount {
            return Err(FeeOverflow.into());
        }
        escrow.is_initialized = true;
        escrow.is_settled = false;
        escrow.is_canceled = false;
        escrow.fee = fee;
        escrow.payer = *payer_info.key;
        escrow.payer_token = *payer_token_info.key;
        escrow.payee_token = *payee_token_info.key;
        escrow.vault_token = *vault_token_info.key;
        escrow.fee_token = *fee_token_info.key;
        escrow.authority = *authority_info.key;
        escrow.amount = amount;

        Escrow::pack(escrow, &mut escrow_info.data.borrow_mut())?;

        let (pda, _bump_seed) = find_program_authority(program_id);

        let token_program_info = next_account_info(account_info_iter)?;
        assert_account_key(token_program_info, &spl_token::id())?;
        let owner_change_ix = spl_token::instruction::set_authority(
            token_program_info.key,
            vault_token_info.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            payer_info.key,
            &[&payer_info.key],
        )?;

        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                vault_token_info.clone(),
                payer_info.clone(),
                token_program_info.clone(),
            ],
        )?;
        Ok(())
    }

    //inside: impl Processor {}
    fn process_settlement(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Process settlement with fee");
        let account_info_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_info_iter)?;

        assert_signer(authority_info)?;

        let payee_token_info = next_account_info(account_info_iter)?;
        let fee_token_info = next_account_info(account_info_iter)?;

        let vault_token_info = next_account_info(account_info_iter)?;
        assert_owned_by(vault_token_info, &spl_token::id())?;

        let vault_token =
            TokenAccount::unpack(&vault_token_info.data.borrow())?;

        let escrow_info = next_account_info(account_info_iter)?;
        let mut escrow = Escrow::unpack(&escrow_info.data.borrow())?;

        if escrow.is_canceled() {
            return Err(AccountAlreadyCanceled.into());
        }
        if escrow.is_settled() {
            return Err(AccountAlreadySettled.into());
        }

        assert_account_key(authority_info, &escrow.authority)?;
        assert_account_key(payee_token_info, &escrow.payee_token)?;
        assert_account_key(fee_token_info, &escrow.fee_token)?;
        assert_account_key(vault_token_info, &escrow.vault_token)?;

        let fee_payer_info = next_account_info(account_info_iter)?;
        
        let token_program_info = next_account_info(account_info_iter)?;
        assert_account_key(token_program_info, &spl_token::id())?;

        let (vault, bump_seed) = find_program_authority(program_id);

        let vault_info = next_account_info(account_info_iter)?;
        assert_account_key(vault_info, &vault)?;

        let vault_signer_seeds = [
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &[bump_seed],
        ];

        let fee = escrow.fee;

        if fee > vault_token.amount {
            msg!(
                "Fee too high..., {} should be less than or equal to {}",
                fee,
                vault_token.amount
            );
            return Err(FeeOverflow.into());
        }

        let amount = vault_token
            .amount
            .checked_sub(fee)
            .ok_or(AmountOverflow)?;

        if vault_token.is_native() {
            let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
                token_program_info.key,
                vault_token_info.key,
                escrow_info.key,
                &vault,
                &[&vault],
            )?;
            msg!("Calling the token program to close pda's temp account...and add the remaining lamports to the escrow account");
            invoke_signed(
                &close_pdas_temp_acc_ix,
                &[
                    vault_token_info.clone(),
                    escrow_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;
            let source_starting_lamports = escrow_info.lamports();
            **escrow_info.lamports.borrow_mut() = source_starting_lamports
                .checked_sub(amount)
                .ok_or(AmountOverflow)?;

            let dest_starting_lamports = payee_token_info.lamports();
            **payee_token_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(amount)
                .ok_or(AmountOverflow)?;
            if fee > 0 {
                let source_starting_lamports = escrow_info.lamports();
                **escrow_info.lamports.borrow_mut() = source_starting_lamports
                    .checked_sub(fee)
                    .ok_or(AmountOverflow)?;

                let dest_starting_lamports = fee_token_info.lamports();
                **fee_token_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(fee)
                    .ok_or(AmountOverflow)?;
            }
        } else {
            let transfer_to_taker_ix = spl_token::instruction::transfer(
                token_program_info.key,
                vault_token_info.key,
                payee_token_info.key,
                &vault,
                &[&vault],
                amount,
            )?;
            msg!("Calling the token program to transfer tokens to the taker...");
            invoke_signed(
                &transfer_to_taker_ix,
                &[
                    vault_token_info.clone(),
                    payee_token_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;
            if fee > 0 {
                let transfer_to_fee_taker_ix = spl_token::instruction::transfer(
                    token_program_info.key,
                    vault_token_info.key,
                    fee_token_info.key,
                    &vault,
                    &[&vault],
                    fee,
                )?;
                msg!("Calling the token program to transfer tokens to the fee taker...");
                invoke_signed(
                    &transfer_to_fee_taker_ix,
                    &[
                        vault_token_info.clone(),
                        fee_token_info.clone(),
                        vault_info.clone(),
                        token_program_info.clone(),
                    ],
                    &[&vault_signer_seeds],
                )?;
            }

            let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
                token_program_info.key,
                vault_token_info.key,
                fee_payer_info.key,
                &vault,
                &[&vault],
            )?;
            msg!("Calling the token program to close pda's temp account...");
            invoke_signed(
                &close_pdas_temp_acc_ix,
                &[
                    vault_token_info.clone(),
                    fee_payer_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;
        }

        msg!("Mark the escrow account as settled...");
        escrow.is_settled = true;
        Escrow::pack(escrow, &mut escrow_info.data.borrow_mut())?;
        Ok(())
    }

    //inside: impl Processor {}
    fn process_cancel(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
        msg!("Process cancelation");
        let account_info_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_info_iter)?;
        assert_signer(authority_info)?;

        let escrow_info = next_account_info(account_info_iter)?;
        let payer_token_info = next_account_info(account_info_iter)?;
        let fee_payer_info = next_account_info(account_info_iter)?;
        let vault_token_info = next_account_info(account_info_iter)?;
        let vault_token =
            TokenAccount::unpack(&vault_token_info.data.borrow())?;

        let mut escrow = Escrow::unpack(&escrow_info.data.borrow())?;

        if escrow.is_canceled() {
            return Err(AccountAlreadyCanceled.into());
        }
        if escrow.is_settled() {
            return Err(AccountAlreadySettled.into());
        }

        assert_account_key(payer_token_info, &escrow.payer_token)?;
        assert_account_key(authority_info, &escrow.authority)?;
        assert_account_key(vault_token_info, &escrow.vault_token)?;

        let token_program_info = next_account_info(account_info_iter)?;

        let (vault_key, bump_seed) = find_program_authority(program_id);

        let vault_info = next_account_info(account_info_iter)?;
        assert_account_key(vault_info, &vault_key)?;

        let amount = vault_token.amount;
        let vault_signer_seeds = [
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &[bump_seed],
        ];
        if vault_token.is_native() {
            let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
                token_program_info.key,
                vault_token_info.key,
                escrow_info.key,
                &vault_key,
                &[&vault_key],
            )?;
            msg!("Calling the token program to close pda's temp account...and add the remaining lamports to the escrow account");
            invoke_signed(
                &close_pdas_temp_acc_ix,
                &[
                    vault_token_info.clone(),
                    escrow_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;
            let source_starting_lamports = escrow_info.lamports();
            **escrow_info.lamports.borrow_mut() = source_starting_lamports
                .checked_sub(amount)
                .ok_or(AmountOverflow)?;

            let dest_starting_lamports = payer_token_info.lamports();
            **payer_token_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(amount)
                .ok_or(AmountOverflow)?;
        } else {
            let transfer_to_payer_ix = spl_token::instruction::transfer(
                token_program_info.key,
                vault_token_info.key,
                payer_token_info.key,
                &vault_key,
                &[&vault_key],
                amount,
            )?;
            msg!("Calling the token program to transfer tokens to the payer...");
            invoke_signed(
                &transfer_to_payer_ix,
                &[
                    vault_token_info.clone(),
                    payer_token_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;

            let close_pdas_temp_acc_ix = spl_token::instruction::close_account(
                token_program_info.key,
                vault_token_info.key,
                fee_payer_info.key,
                &vault_key,
                &[&vault_key],
            )?;
            msg!("Calling the token program to close pda's temp account...");
            invoke_signed(
                &close_pdas_temp_acc_ix,
                &[
                    vault_token_info.clone(),
                    fee_payer_info.clone(),
                    vault_info.clone(),
                    token_program_info.clone(),
                ],
                &[&vault_signer_seeds],
            )?;
        }

        msg!("Mark the escrow account as settled...");
        escrow.is_canceled = true;
        Escrow::pack(escrow, &mut escrow_info.data.borrow_mut())?;
        Ok(())
    }

    //inside: impl Processor {}
    fn process_close(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let authority_info = next_account_info(account_info_iter)?;
        assert_signer(authority_info)?;

        let escrow_info = next_account_info(account_info_iter)?;
        assert_owned_by(escrow_info, program_id)?;

        let escrow = Escrow::unpack(&escrow_info.data.borrow())?;

        if escrow.authority != *authority_info.key {
            return Err(ProgramError::InvalidAccountData);
        }

        if !(escrow.is_settled() || escrow.is_canceled()) {
            return Err(AccountNotSettledOrCanceled.into());
        }

        let fee_payer_info = next_account_info(account_info_iter)?;
        msg!("Closing the escrow account...");
        **fee_payer_info.lamports.borrow_mut() = fee_payer_info
            .lamports()
            .checked_add(escrow_info.lamports())
            .ok_or(AmountOverflow)?;
        **escrow_info.lamports.borrow_mut() = 0;
        *escrow_info.data.borrow_mut() = &mut [];
        Ok(())
    }
}
