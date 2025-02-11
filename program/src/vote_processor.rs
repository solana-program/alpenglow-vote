use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::state::{PodSlot, PodUnixTimestamp};

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
    /// Only relevant after APE
    pub _replayed_slot: PodSlot,

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
    /// Only relevant after APE
    pub _replayed_slot: PodSlot,

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
    _vote_account: &AccountInfo,
    _vote_authority: &Pubkey,
    _vote: &NotarizationVoteInstructionData,
) -> Result<(), ProgramError> {
    Ok(())
}

pub(crate) fn process_finalization_vote(
    _vote_account: &AccountInfo,
    _vote_authority: &Pubkey,
    _vote: &FinalizationVoteInstructionData,
) -> Result<(), ProgramError> {
    Ok(())
}

pub(crate) fn process_skip_vote(
    _vote_account: &AccountInfo,
    _vote_authority: &Pubkey,
    _vote: &SkipVoteInstructionData,
) -> Result<(), ProgramError> {
    Ok(())
}
