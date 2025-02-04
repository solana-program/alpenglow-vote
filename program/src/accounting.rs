//! Accounting related operations on the Vote Account

use bytemuck::{Pod, PodInOption, Zeroable, ZeroableInOption};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::clock::Slot;
use solana_program::epoch_schedule::EpochSchedule;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use spl_pod::bytemuck::pod_from_bytes_mut;
use spl_pod::primitives::PodU64;

use crate::error::VoteError;
use crate::instruction::AuthorityType;
use crate::state::{PodEpoch, VoteState};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct AuthorizedVoter {
    pub epoch: PodEpoch,
    pub voter: Pubkey,
}

// UNSAFE: we require that `epoch > 0` so this is safe
unsafe impl ZeroableInOption for AuthorizedVoter {}
unsafe impl PodInOption for AuthorizedVoter {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct EpochCredit {
    pub epoch: PodEpoch,
    pub credits: PodU64,
    pub prev_credits: PodU64,
}

/// Authorize the given pubkey to withdraw or sign votes. This may be called multiple times,
/// but will implicitly withdraw authorization from the previously authorized key
pub(crate) fn authorize(
    vote_account: &AccountInfo,
    new_authority: &Pubkey,
    vote_authorize: AuthorityType,
    authority: &Pubkey,
    clock: &Clock,
) -> Result<(), ProgramError> {
    let mut buffer = vote_account.try_borrow_mut_data()?;
    let vote_state = pod_from_bytes_mut::<VoteState>(&mut buffer)?;

    match vote_authorize {
        AuthorityType::Voter => {
            // Current authorized withdrawer or voter must match
            if vote_state.authorized_withdrawer != *authority
                && vote_state.authorized_voter.voter != *authority
            {
                return Err(ProgramError::MissingRequiredSignature);
            }

            let epoch_in_effect = clock
                .leader_schedule_epoch
                .checked_add(1)
                .ok_or(ProgramError::InvalidInstructionData)?;
            // Overwrite the next authorized voter
            vote_state.next_authorized_voter = Some(AuthorizedVoter {
                epoch: PodU64::from(epoch_in_effect),
                voter: *new_authority,
            });
        }
        AuthorityType::Withdrawer => {
            // Current authorized withdrawer must match
            if vote_state.authorized_withdrawer != *authority {
                return Err(ProgramError::MissingRequiredSignature);
            }
            vote_state.authorized_withdrawer = *new_authority;
        }
    }
    Ok(())
}

pub(crate) fn withdraw(
    vote_account: &AccountInfo,
    recipient: &AccountInfo,
    lamports: u64,
    withdraw_pubkey: &Pubkey,
    rent_sysvar: &Rent,
    clock: &Clock,
) -> Result<(), ProgramError> {
    let vote_state = vote_account.data.borrow();
    let vote_state = bytemuck::from_bytes::<VoteState>(&vote_state);

    if vote_state.authorized_withdrawer != *withdraw_pubkey {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let remaining_balance = vote_account
        .try_lamports()?
        .checked_sub(lamports)
        .ok_or(ProgramError::InsufficientFunds)?;

    if remaining_balance == 0 {
        let last_epoch_with_credits = u64::from(vote_state.epoch_credits.epoch);
        let current_epoch = clock.epoch;
        // if current_epoch - last_epoch_with_credits < 2 then the validator has received credits
        // either in the current epoch or the previous epoch. If it's >= 2 then it has been at least
        // one full epoch since the validator has received credits.
        let reject_active_vote_account_close =
            current_epoch.saturating_sub(last_epoch_with_credits) < 2;

        if reject_active_vote_account_close {
            return Err(VoteError::ActiveVoteAccountClose.into());
        } else {
            // Deinitialize upon zero-balance
            VoteState::set_vote_account_state(vote_account, &VoteState::default())?;
        }
    } else {
        let min_rent_exempt_balance = rent_sysvar.minimum_balance(vote_account.data_len());
        if remaining_balance < min_rent_exempt_balance {
            return Err(ProgramError::InsufficientFunds);
        }
    }

    let vote_account_lamports = vote_account.try_borrow_mut_lamports()?;
    vote_account_lamports
        .checked_sub(lamports)
        .ok_or(ProgramError::InsufficientFunds)?;
    let recipient_lamports = recipient.try_borrow_mut_lamports()?;
    recipient_lamports
        .checked_add(lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}

pub(crate) fn update_validator_identity(
    vote_account: &AccountInfo,
    new_node_pubkey: &Pubkey,
    withdraw_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let mut buffer = vote_account.try_borrow_mut_data()?;
    let vote_state = pod_from_bytes_mut::<VoteState>(&mut buffer)?;

    if vote_state.authorized_withdrawer != *withdraw_pubkey {
        return Err(ProgramError::MissingRequiredSignature);
    }

    vote_state.node_pubkey = *new_node_pubkey;
    Ok(())
}

pub(crate) fn update_commission(
    vote_account: &AccountInfo,
    commission: u8,
    withdraw_pubkey: &Pubkey,
    epoch_schedule: &EpochSchedule,
    clock: &Clock,
) -> Result<(), ProgramError> {
    let mut buffer = vote_account.try_borrow_mut_data()?;
    let vote_state = pod_from_bytes_mut::<VoteState>(&mut buffer)?;

    if vote_state.authorized_withdrawer != *withdraw_pubkey {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let is_commission_increase = commission > vote_state.commission;
    if !is_commission_increase && !is_commission_update_allowed(clock.slot, epoch_schedule) {
        return Err(VoteError::CommissionUpdateTooLate.into());
    }

    vote_state.commission = commission;

    Ok(())
}

/// Given the current slot and epoch schedule, determine if a commission change
/// is allowed
fn is_commission_update_allowed(slot: Slot, epoch_schedule: &EpochSchedule) -> bool {
    // always allowed during warmup epochs
    if let Some(relative_slot) = slot
        .saturating_sub(epoch_schedule.first_normal_slot)
        .checked_rem(epoch_schedule.slots_per_epoch)
    {
        // allowed up to the midpoint of the epoch
        relative_slot.saturating_mul(2) <= epoch_schedule.slots_per_epoch
    } else {
        // no slots per epoch, just allow it, even though this should never happen
        true
    }
}
