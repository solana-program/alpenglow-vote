use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Slot;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::{slot_hashes::PodSlotHashes, Sysvar};

use crate::state::{BlockTimestamp, PodSlot, PodUnixTimestamp, VoteState};

pub(crate) const CURRENT_NOTARIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_FINALIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_SKIP_VOTE_VERSION: u8 = 1;

trait GetVersion {
    fn version(&self) -> u8;
}

trait GetTimestamp {
    fn timestamp(&self) -> PodUnixTimestamp;
}

trait GetReplayed {
    fn replayed_slot(&self) -> &PodSlot;
    fn replayed_bank_hash(&self) -> &Hash;
}

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

impl GetVersion for NotarizationVoteInstructionData {
    fn version(&self) -> u8 {
        self.version
    }
}

impl GetTimestamp for NotarizationVoteInstructionData {
    fn timestamp(&self) -> PodUnixTimestamp {
        self.timestamp
    }
}

impl GetReplayed for NotarizationVoteInstructionData {
    fn replayed_bank_hash(&self) -> &Hash {
        &self.replayed_bank_hash
    }

    fn replayed_slot(&self) -> &PodSlot {
        &self.replayed_slot
    }
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

impl GetVersion for FinalizationVoteInstructionData {
    fn version(&self) -> u8 {
        self.version
    }
}

impl GetTimestamp for FinalizationVoteInstructionData {
    fn timestamp(&self) -> PodUnixTimestamp {
        self.timestamp
    }
}

impl GetReplayed for FinalizationVoteInstructionData {
    fn replayed_bank_hash(&self) -> &Hash {
        &self.replayed_bank_hash
    }

    fn replayed_slot(&self) -> &PodSlot {
        &self.replayed_slot
    }
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

impl GetVersion for SkipVoteInstructionData {
    fn version(&self) -> u8 {
        self.version
    }
}

impl GetTimestamp for SkipVoteInstructionData {
    fn timestamp(&self) -> PodUnixTimestamp {
        self.timestamp
    }
}

fn version_timestamp_checks<T: GetVersion + GetTimestamp, const CURRENT_VERSION: u8>(
    inst_data: &T,
) -> Result<(), ProgramError> {
    if inst_data.version() > CURRENT_VERSION {
        Err(ProgramError::InvalidInstructionData)
    } else if i64::from(inst_data.timestamp())
        >= solana_program::sysvar::clock::Clock::get()?.unix_timestamp
    {
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

fn replay_bank_hash_checks<T: GetReplayed>(
    inst_data: &T,
    vote_slot: Slot,
) -> Result<(), ProgramError> {
    // It doesn't make sense to replay blocks that happen after the slot we're voting on.
    let replayed_slot = Slot::from(*inst_data.replayed_slot());
    if replayed_slot > vote_slot {
        Err(ProgramError::InvalidInstructionData)
    }
    // We must have already executed `vote.replayed_slot` and stored the associated bank hash
    // (error out otherwise). Ensure that our bank hash matches what we observe.
    else if inst_data.replayed_bank_hash()
        != &PodSlotHashes::fetch()?
            .get(&replayed_slot)?
            .ok_or(ProgramError::InvalidInstructionData)?
    {
        Err(ProgramError::InvalidInstructionData)
    } else {
        Ok(())
    }
}

pub(crate) fn process_notarization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    vote: &NotarizationVoteInstructionData,
) -> Result<(), ProgramError> {
    version_timestamp_checks::<_, CURRENT_NOTARIZE_VOTE_VERSION>(vote)?;

    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // A notarization vote must be strictly greater than the latest slot voted upon.
    let vote_slot = Slot::from(vote.slot);
    if vote_slot
        <= vote_state
            .latest_finalized_slot()
            .max(vote_state.latest_notarized_slot())
            .max(vote_state.latest_skip_end_slot())
    {
        return Err(ProgramError::InvalidInstructionData);
    }

    replay_bank_hash_checks(vote, vote_slot)?;

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
    version_timestamp_checks::<_, CURRENT_FINALIZE_VOTE_VERSION>(vote)?;

    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // (1) Only accept finalization votes on slots strictly greater than the latest_finalized_slot.
    // Re. the equality case - it makes no sense to double-finalize vote.
    //
    // (2) Similarly, only accept finalization votes on slots strictly greater than the
    // latest_skip_end_slot. Re. the equality case - validators cannot issue a skip and a finalize
    // for the same slot.
    let vote_slot = Slot::from(vote.slot);
    if vote_slot <= vote_state.latest_finalized_slot()
        || vote_slot <= vote_state.latest_skip_end_slot()
    {
        return Err(ProgramError::InvalidInstructionData);
    }

    replay_bank_hash_checks(vote, vote_slot)?;

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
    version_timestamp_checks::<_, CURRENT_SKIP_VOTE_VERSION>(vote)?;

    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // (1) Skips must be of the form (vote.start_slot, vote.end_slot) = (t, t + N) where t, N are
    // integers >= 0.
    //
    // (2) Skip vote ranges must happen strictly after finalization slot votes.
    //
    // (3) Skip vote ranges must happen at the same time or after notarization slot votes.
    let (vote_start_slot, vote_end_slot) = (Slot::from(vote.start_slot), Slot::from(vote.end_slot));

    if vote_end_slot < vote_start_slot
        || vote_start_slot <= vote_state.latest_finalized_slot()
        || vote_start_slot < vote_state.latest_notarized_slot()
    {
        return Err(ProgramError::InvalidInstructionData);
    }

    vote_state.latest_skip_start_slot = vote.start_slot;
    vote_state.latest_skip_end_slot = vote.end_slot;
    vote_state.latest_timestamp = BlockTimestamp {
        slot: vote.end_slot,
        timestamp: vote.timestamp,
    };

    Ok(())
}
