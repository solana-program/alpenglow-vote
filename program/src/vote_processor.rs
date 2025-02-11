use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Slot;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::{slot_hashes::PodSlotHashes, Sysvar};

use crate::error::VoteError;
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

fn version_timestamp_checks(
    slot: PodSlot,
    last_block_timestamp: &BlockTimestamp,
    version: u8,
    current_version: u8,
) -> Result<(), ProgramError> {
    let last_slot_i: u64 = last_block_timestamp.slot.into();
    let last_timestamp_i: i64 = last_block_timestamp.timestamp.into();

    let slot_i: u64 = slot.into();
    let timestamp_i = solana_program::sysvar::clock::Clock::get()?.unix_timestamp;
    let timestamp = PodUnixTimestamp::from_primitive(timestamp_i);

    if version != current_version {
        Err(VoteError::VersionMismatch.into())
    } else if slot_i < last_slot_i
        || timestamp_i < last_timestamp_i
        || (slot == last_block_timestamp.slot
            && &BlockTimestamp { slot, timestamp } != last_block_timestamp
            && last_slot_i != 0)
    {
        Err(VoteError::TimestampTooOld.into())
    } else {
        Ok(())
    }
}

fn replay_bank_hash_checks(
    replayed_slot: Slot,
    replayed_bank_hash: Hash,
    vote_slot: Slot,
) -> Result<(), VoteError> {
    // It doesn't make sense to replay blocks that happen after the slot we're voting on.
    if replayed_slot > vote_slot {
        Err(VoteError::ReplaySlotIsAheadOfVoteSlot)
    }
    // We must have already executed `replayed_slot` and stored the associated bank hash
    // (error out otherwise). Ensure that our bank hash matches what we observe.
    else if replayed_bank_hash
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

    version_timestamp_checks(
        vote.slot,
        &vote_state.latest_timestamp,
        vote.version,
        CURRENT_NOTARIZE_VOTE_VERSION,
    )?;

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // A notarization vote must be strictly greater than the latest slot voted upon.
    let vote_slot = Slot::from(vote.slot);
    if vote_slot <= vote_state.latest_notarized_slot() {
        return Err(VoteError::VoteTooOld.into());
    }

    replay_bank_hash_checks(
        vote.replayed_slot.into(),
        vote.replayed_bank_hash,
        vote_slot,
    )?;

    vote_state.latest_notarized_slot = vote.slot;
    vote_state.latest_notarized_block_id = vote.block_id;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.slot,
        timestamp: vote.timestamp,
    };

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

    version_timestamp_checks(
        vote.slot,
        &vote_state.latest_timestamp,
        vote.version,
        CURRENT_FINALIZE_VOTE_VERSION,
    )?;

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let vote_slot = Slot::from(vote.slot);
    if vote_slot <= vote_state.latest_finalized_slot() {
        return Err(VoteError::VoteTooOld.into());
    }

    if vote_state.latest_skip_start_slot() <= vote_slot
        && vote_slot <= vote_state.latest_skip_end_slot()
    {
        return Err(VoteError::VoteTooOld.into());
    }

    replay_bank_hash_checks(
        vote.replayed_slot.into(),
        vote.replayed_bank_hash,
        vote_slot,
    )?;

    vote_state.latest_finalized_slot = vote.slot;
    vote_state.latest_finalized_block_id = vote.block_id;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.slot,
        timestamp: vote.timestamp,
    };

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

    version_timestamp_checks(
        vote.end_slot,
        &vote_state.latest_timestamp,
        vote.version,
        CURRENT_SKIP_VOTE_VERSION,
    )?;

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
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.end_slot,
        timestamp: vote.timestamp,
    };

    Ok(())
}
