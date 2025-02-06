use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::state::{BlockTimestamp, PodSlot, PodUnixTimestamp, VoteState};

pub(crate) const CURRENT_NOTARIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_FINALIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_SKIP_VOTE_VERSION: u8 = 1;

/// A notarization vote, the data expected by
/// `VoteInstruction::Notarize`
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct NotarizationVoteInstructionData {
    /// The version of this vote message
    pub version: u8,

    /// The slot being notarized
    pub slot: PodSlot,

    /// The block id of this slot
    pub block_id: Hash,

    /// The slot of the last replayed block
    /// Prior to APE this is equal to `slot`
    pub replayed_slot: PodSlot,

    /// The bank_hash of the last replayed block
    /// Prior to APE this is the bank hash of `slot`
    pub replayed_bank_hash: Hash,

    /// The timestamp when this vote was created
    pub timestamp: PodUnixTimestamp,
}

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
    /// Prior to APE this is equal to `slot`
    pub replayed_slot: PodSlot,

    /// The bank_hash of the last replayed block
    /// Prior to APE this is the bank hash of `slot`
    pub replayed_bank_hash: Hash,

    /// The timestamp when this vote was created
    pub timestamp: PodUnixTimestamp,
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

    /// The timestamp when this vote was created
    pub timestamp: PodUnixTimestamp,
}

pub(crate) fn process_notarization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &NotarizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    vote_state.latest_notarized_slot = vote.slot;
    vote_state.latest_notarized_block_id = vote.block_id;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.slot,
        timestamp: vote.timestamp,
    };

    // Correct?
    vote_state.replayed_slot = vote.replayed_slot;
    vote_state.replayed_bank_hash = vote.replayed_bank_hash;

    Ok(())
}

pub(crate) fn process_finalization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &FinalizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    vote_state.latest_finalized_slot = vote.slot;
    vote_state.latest_finalized_block_id = vote.block_id;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.slot,
        timestamp: vote.timestamp,
    };

    // Correct?
    vote_state.replayed_slot = vote.replayed_slot;
    vote_state.replayed_bank_hash = vote.replayed_bank_hash;

    Ok(())
}

pub(crate) fn process_skip_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &SkipVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    vote_state.latest_skip_start_slot = vote.start_slot;
    vote_state.latest_skip_end_slot = vote.end_slot;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.end_slot,
        timestamp: vote.timestamp,
    };

    Ok(())
}
