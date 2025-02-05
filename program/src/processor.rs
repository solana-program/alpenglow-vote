//! Program state processor

use solana_program::program_error::ProgramError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::{self, Clock},
    entrypoint::ProgramResult,
    epoch_schedule,
    pubkey::Pubkey,
    rent,
    sysvar::Sysvar,
};
use spl_pod::primitives::PodU64;

use crate::accounting;
use crate::error::VoteError;
use crate::instruction::{
    decode_instruction_data, decode_instruction_data_with_seed, decode_instruction_type,
    AuthorityType, AuthorizeCheckedWithSeedInstructionData, AuthorizeInstructionData,
    AuthorizeWithSeedInstructionData, InitializeAccountInstructionData, VoteInstruction,
};
use crate::state::VoteState;

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction_type = decode_instruction_type(input)?;
    let account_info_iter = &mut accounts.iter();

    let vote_account = next_account_info(account_info_iter)?;
    if vote_account.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
    }

    match instruction_type {
        VoteInstruction::InitializeAccount => {
            let rent_account = next_account_info(account_info_iter)?;
            let rent = rent::Rent::from_account_info(rent_account)?;
            if !rent.is_exempt(vote_account.lamports(), vote_account.data_len()) {
                return Err(ProgramError::InsufficientFunds);
            }

            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(node_pubkey) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let instruction_data =
                decode_instruction_data::<InitializeAccountInstructionData>(input)?;
            if instruction_data.node_pubkey != *node_pubkey {
                return Err(ProgramError::MissingRequiredSignature);
            }

            initialize_account(vote_account, instruction_data, &clock)
        }
        VoteInstruction::Authorize => {
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(authority_pubkey) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let instruction_data = decode_instruction_data::<AuthorizeInstructionData>(input)?;
            let vote_authorize = AuthorityType::try_from(instruction_data.authority_type)
                .map_err(|_| ProgramError::from(VoteError::InvalidAuthorizeType))?;
            accounting::authorize(
                vote_account,
                &instruction_data.new_authorized_pubkey,
                vote_authorize,
                authority_pubkey,
                &clock,
            )
        }
        VoteInstruction::AuthorizeChecked => {
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(authority_pubkey) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };
            let Some(new_authority_pubkey) = next_account_info(account_info_iter)?.signer_key()
            else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let vote_authorize = AuthorityType::try_from(*decode_instruction_data::<u8>(input)?)
                .map_err(|_| ProgramError::from(VoteError::InvalidAuthorizeType))?;

            accounting::authorize(
                vote_account,
                new_authority_pubkey,
                vote_authorize,
                authority_pubkey,
                &clock,
            )
        }
        VoteInstruction::AuthorizeWithSeed => {
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(base_key) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let (instruction_data, seed) =
                decode_instruction_data_with_seed::<AuthorizeWithSeedInstructionData>(input)?;
            let seed =
                std::str::from_utf8(seed.data()).map_err(|_| ProgramError::InvalidArgument)?;
            let vote_authorize = AuthorityType::try_from(instruction_data.authority_type)
                .map_err(|_| ProgramError::from(VoteError::InvalidAuthorizeType))?;

            let authority_pubkey = Pubkey::create_with_seed(
                base_key,
                seed,
                &instruction_data.current_authority_derived_key_owner,
            )?;

            accounting::authorize(
                vote_account,
                &instruction_data.new_authority,
                vote_authorize,
                &authority_pubkey,
                &clock,
            )
        }
        VoteInstruction::AuthorizeCheckedWithSeed => {
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(base_key) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let Some(new_authority_pubkey) = next_account_info(account_info_iter)?.signer_key()
            else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let (instruction_data, seed) = decode_instruction_data_with_seed::<
                AuthorizeCheckedWithSeedInstructionData,
            >(input)?;
            let seed =
                std::str::from_utf8(seed.data()).map_err(|_| ProgramError::InvalidArgument)?;
            let vote_authorize = AuthorityType::try_from(instruction_data.authority_type)
                .map_err(|_| ProgramError::from(VoteError::InvalidAuthorizeType))?;

            let authority_pubkey = Pubkey::create_with_seed(
                base_key,
                seed,
                &instruction_data.current_authority_derived_key_owner,
            )?;

            accounting::authorize(
                vote_account,
                new_authority_pubkey,
                vote_authorize,
                &authority_pubkey,
                &clock,
            )
        }
        VoteInstruction::Withdraw => {
            let recipient = next_account_info(account_info_iter)?;
            let rent_account = next_account_info(account_info_iter)?;
            let rent = rent::Rent::from_account_info(rent_account)?;
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(withdraw_authority_pubkey) =
                next_account_info(account_info_iter)?.signer_key()
            else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let lamports = u64::from(*decode_instruction_data::<PodU64>(input)?);

            accounting::withdraw(
                vote_account,
                recipient,
                lamports,
                withdraw_authority_pubkey,
                &rent,
                &clock,
            )
        }
        VoteInstruction::UpdateValidatorIdentity => {
            let Some(new_node_pubkey) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };
            let Some(withdraw_pubkey) = next_account_info(account_info_iter)?.signer_key() else {
                return Err(ProgramError::MissingRequiredSignature);
            };
            accounting::update_validator_identity(vote_account, new_node_pubkey, withdraw_pubkey)
        }
        VoteInstruction::UpdateCommission => {
            let epoch_schedule_account = next_account_info(account_info_iter)?;
            let epoch_schedule =
                epoch_schedule::EpochSchedule::from_account_info(epoch_schedule_account)?;
            let clock_account = next_account_info(account_info_iter)?;
            let clock = clock::Clock::from_account_info(clock_account)?;

            let Some(withdraw_authority_pubkey) =
                next_account_info(account_info_iter)?.signer_key()
            else {
                return Err(ProgramError::MissingRequiredSignature);
            };

            let commission = *decode_instruction_data::<u8>(input)?;

            accounting::update_commission(
                vote_account,
                commission,
                withdraw_authority_pubkey,
                &epoch_schedule,
                &clock,
            )
        }
    }
}

/// Initialize the vote_state for a vote account
/// Assumes that the account is being init as part of a account creation or balance transfer and
/// that the transaction must be signed by the staker's keys
pub(crate) fn initialize_account(
    vote_account: &AccountInfo,
    init_data: &InitializeAccountInstructionData,
    clock: &Clock,
) -> Result<(), ProgramError> {
    if vote_account.data_len() != std::mem::size_of::<VoteState>() {
        return Err(ProgramError::InvalidAccountData);
    }
    {
        let vote_state = vote_account.data.borrow();
        let vote_state = bytemuck::from_bytes::<VoteState>(&vote_state);

        if vote_state.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
    }

    VoteState::set_vote_account_state(vote_account, &VoteState::new(init_data, clock))
}
