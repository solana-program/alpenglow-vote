use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Slot;
use solana_program::clock::UnixTimestamp;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::slot_hashes::PodSlotHashes;

use crate::error::VoteError;
use crate::state::BlockTimestamp;
use crate::state::{PodSlot, PodUnixTimestamp, VoteState};

pub(crate) const CURRENT_NOTARIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_FINALIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_SKIP_VOTE_VERSION: u8 = 1;

/// A notarization vote, the data expected by
/// `VoteInstruction::Notarize`
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NotarizationVoteInstructionData {
    /// The version of this vote message
    pub version: u8,

    /// The slot being notarized
    pub slot: PodSlot,

    /// The block id of this slot
    pub block_id: Hash,

    /// The slot of the last replayed block
    /// Only relevant after APE
    pub _replayed_slot: PodSlot,

    /// The bank_hash of the last replayed block
    /// Prior to APE this is the bank hash of `slot`
    pub replayed_bank_hash: Hash,

    /// The timestamp when this vote was created
    pub timestamp: Option<PodUnixTimestamp>,
}

// SAFETY: for our purposes we treat a zero timestamp as the validator not
// supplying a timestamp, so timestamp is safe to be zeroable
unsafe impl Zeroable for NotarizationVoteInstructionData {}
unsafe impl Pod for NotarizationVoteInstructionData {}

/// A finalization vote, the data expected by
/// `VoteInstruction::Finalize`
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct FinalizationVoteInstructionData {
    /// The version of this vote message
    pub version: u8,

    /// The slot being finalized
    pub slot: PodSlot,

    /// The block id of this slot
    pub block_id: Hash,

    /// The slot of the last replayed block
    /// Only relevant after APE
    pub _replayed_slot: PodSlot,

    /// The bank_hash of the last replayed block
    /// Prior to APE this is the bank hash of `slot`
    pub replayed_bank_hash: Hash,
}

/// A skip vote, the data expected by
/// `VoteInstruction::Skip`
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct SkipVoteInstructionData {
    /// The version of this vote message
    pub version: u8,

    /// The start of the slot range being skipped
    pub start_slot: PodSlot,

    /// The end (inclusive) of the slot range being skipped
    pub end_slot: PodSlot,
}

fn replay_bank_hash_checks(replayed_slot: Slot, replayed_bank_hash: Hash) -> Result<(), VoteError> {
    // We must have already executed `replayed_slot` and stored the associated bank hash
    // (error out otherwise). Ensure that our bank hash matches what we observe.
    if replayed_bank_hash
        != PodSlotHashes::fetch()
            .map_err(|_| VoteError::MissingSlotHashesSysvar)?
            .get(&replayed_slot)
            .map_err(|_| VoteError::MissingSlotHashesSysvar)?
            .ok_or(VoteError::SlotHashesMissingKey)?
    {
        Err(VoteError::ReplayBankHashMismatch)
    } else {
        Ok(())
    }
}

pub(crate) fn process_notarization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &NotarizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    let vote_slot = vote.slot.into();

    if vote.version != CURRENT_NOTARIZE_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Notarization votes must be strictly increasing
    if vote_slot <= vote_state.latest_notarized_slot() && vote_state.latest_notarized_slot() != 0 {
        return Err(VoteError::VoteTooOld.into());
    }

    replay_bank_hash_checks(vote_slot, vote.replayed_bank_hash)?;

    vote_state.latest_notarized_slot = vote.slot;
    vote_state.latest_notarized_block_id = vote.block_id;
    vote_state.latest_notarized_bank_hash = vote.replayed_bank_hash;

    if let Some(timestamp) = vote.timestamp.map(UnixTimestamp::from) {
        if timestamp != 0 && timestamp > vote_state.latest_timestamp().timestamp() {
            vote_state.latest_timestamp = BlockTimestamp {
                slot: vote.slot,
                timestamp: vote
                    .timestamp
                    .expect("timestamp is verified to be not None above"),
            };
        } else {
            return Err(VoteError::TimestampTooOld.into());
        }
    }

    Ok(())
}

pub(crate) fn process_finalization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &FinalizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    let vote_slot = vote.slot.into();

    if vote.version != CURRENT_FINALIZE_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if vote_slot <= vote_state.latest_finalized_slot() {
        return Err(VoteError::VoteTooOld.into());
    }

    if vote_state.latest_skip_start_slot() <= vote_slot
        && vote_slot <= vote_state.latest_skip_end_slot()
    {
        return Err(VoteError::SkipSlotRangeContainsFinalizationVote.into());
    }

    replay_bank_hash_checks(vote_slot, vote.replayed_bank_hash)?;

    vote_state.latest_finalized_slot = vote.slot;
    vote_state.latest_finalized_block_id = vote.block_id;
    vote_state.latest_finalized_bank_hash = vote.replayed_bank_hash;

    Ok(())
}

pub(crate) fn process_skip_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &SkipVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote.version != CURRENT_SKIP_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (vote_start_slot, vote_end_slot) = (Slot::from(vote.start_slot), Slot::from(vote.end_slot));

    if vote_end_slot < vote_start_slot {
        return Err(VoteError::SkipEndSlotLowerThanSkipStartSlot.into());
    }

    if vote_start_slot <= vote_state.latest_finalized_slot()
        && vote_state.latest_finalized_slot() <= vote_end_slot
    {
        return Err(VoteError::SkipSlotRangeContainsFinalizationVote.into());
    }

    vote_state.latest_skip_start_slot = vote.start_slot;
    vote_state.latest_skip_end_slot = vote.end_slot;

    Ok(())
}
