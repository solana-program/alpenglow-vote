//! Vote data types for use by clients

use std::ops::RangeInclusive;

use either::Either;
use serde::{Deserialize, Serialize};
use solana_program::clock::{Slot, UnixTimestamp};
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;

use crate::instruction::{decode_instruction_data, decode_instruction_type, VoteInstruction};
use crate::vote_processor::{
    FinalizationVoteInstructionData, NotarizationVoteInstructionData, SkipVoteInstructionData,
};

/// Enum that clients can use to parse and create the vote
/// structures expected by the program
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Vote {
    /// A notarization vote
    Notarize(NotarizationVote),
    /// A finalization vote
    Finalize(FinalizationVote),
    /// A skip vote
    Skip(SkipVote),
}

impl Vote {
    /// Create a new notarization vote
    pub fn new_notarization_vote(
        slot: Slot,
        block_id: Hash,
        bank_hash: Hash,
        timestamp: Option<UnixTimestamp>,
    ) -> Self {
        Self::from(NotarizationVote::new(
            slot, block_id, slot, bank_hash, timestamp,
        ))
    }

    /// Create a new finalization vote
    pub fn new_finalization_vote(slot: Slot, block_id: Hash, bank_hash: Hash) -> Self {
        Self::from(FinalizationVote::new(slot, block_id, slot, bank_hash))
    }

    /// Create a new skip vote
    pub fn new_skip_vote(start: Slot, end: Slot) -> Self {
        Self::from(SkipVote::new(start, end))
    }

    /// If this instruction represented by `instruction_data` is a vote
    pub fn is_simple_vote(instruction_data: &[u8]) -> Result<bool, ProgramError> {
        let instruction_type = decode_instruction_type(instruction_data)?;
        Ok(matches!(
            instruction_type,
            VoteInstruction::Notarize | VoteInstruction::Finalize | VoteInstruction::Skip
        ))
    }

    /// Deserializes instruction represented by `instruction_data` into a `Vote`
    /// Must be guarded by `is_simple_vote`
    pub fn deserialize_simple_vote(instruction_data: &[u8]) -> Result<Vote, ProgramError> {
        debug_assert!(Self::is_simple_vote(instruction_data)?);
        let instruction_type = decode_instruction_type(instruction_data)?;
        match instruction_type {
            VoteInstruction::Notarize => {
                let notarization_vote =
                    decode_instruction_data::<NotarizationVoteInstructionData>(instruction_data)?;
                Ok(Vote::from(NotarizationVote::new_internal(
                    notarization_vote,
                )))
            }
            VoteInstruction::Finalize => {
                let finalization_vote =
                    decode_instruction_data::<FinalizationVoteInstructionData>(instruction_data)?;
                Ok(Vote::from(FinalizationVote::new_internal(
                    finalization_vote,
                )))
            }
            VoteInstruction::Skip => {
                let skip_vote =
                    decode_instruction_data::<SkipVoteInstructionData>(instruction_data)?;
                Ok(Vote::from(SkipVote::new_internal(skip_vote)))
            }
            _ => panic!("Programmer error"),
        }
    }

    /// The slot which was voted for. For skip votes, this is the end of the range
    pub fn slot(&self) -> Slot {
        match self {
            Self::Notarize(vote) => vote.slot(),
            Self::Finalize(vote) => vote.slot(),
            Self::Skip(vote) => vote.end_slot,
        }
    }

    /// The skip range for skip votes
    pub fn skip_range(&self) -> Option<RangeInclusive<Slot>> {
        match self {
            Self::Notarize(_) | Self::Finalize(_) => None,
            Self::Skip(vote) => Some(vote.skip_range()),
        }
    }

    /// Whether the vote is a notarization vote
    pub fn is_notarization(&self) -> bool {
        matches!(self, Self::Notarize(_))
    }

    /// Whether the vote is a finalization vote
    pub fn is_finalize(&self) -> bool {
        matches!(self, Self::Finalize(_))
    }

    /// Whether the vote is a skip vote
    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip(_))
    }

    /// Whether the vote is a notarization or finalization
    pub fn is_notarization_or_finalization(&self) -> bool {
        matches!(self, Self::Notarize(_) | Self::Finalize(_))
    }

    /// The voted slots, `Left` for a notarize/finalize vote on a single slot, `Right` for a skip range
    pub fn voted_slots(&self) -> Either<Slot, RangeInclusive<Slot>> {
        match self {
            Self::Notarize(vote) => Either::Left(vote.slot()),
            Self::Finalize(vote) => Either::Left(vote.slot()),
            Self::Skip(vote) => Either::Right(vote.skip_range()),
        }
    }
}

impl From<NotarizationVote> for Vote {
    fn from(vote: NotarizationVote) -> Self {
        Self::Notarize(vote)
    }
}

impl From<FinalizationVote> for Vote {
    fn from(vote: FinalizationVote) -> Self {
        Self::Finalize(vote)
    }
}

impl From<SkipVote> for Vote {
    fn from(vote: SkipVote) -> Self {
        Self::Skip(vote)
    }
}

/// A notarization vote
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct NotarizationVote {
    slot: Slot,
    block_id: Hash,
    _replayed_slot: Slot,
    replayed_bank_hash: Hash,
    timestamp: Option<UnixTimestamp>,
}

impl NotarizationVote {
    fn new_internal(notarization_vote: &NotarizationVoteInstructionData) -> Self {
        Self {
            slot: Slot::from(notarization_vote.slot),
            block_id: notarization_vote.block_id,
            _replayed_slot: 0,
            replayed_bank_hash: notarization_vote.replayed_bank_hash,
            timestamp: notarization_vote.timestamp.map(UnixTimestamp::from),
        }
    }

    /// Construct a notarization vote for `slot`
    pub fn new(
        slot: Slot,
        block_id: Hash,
        _replayed_slot: Slot,
        replayed_bank_hash: Hash,
        timestamp: Option<UnixTimestamp>,
    ) -> Self {
        Self {
            slot,
            block_id,
            _replayed_slot,
            replayed_bank_hash,
            timestamp,
        }
    }

    /// The slot to notarize
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// The block_id of the notarization slot
    pub fn block_id(&self) -> &Hash {
        &self.block_id
    }

    /// The bank hash of the latest replayed slot
    pub fn replayed_bank_hash(&self) -> &Hash {
        &self.replayed_bank_hash
    }

    /// The time at which this vote was created
    pub fn timestamp(&self) -> Option<UnixTimestamp> {
        self.timestamp
    }
}

/// A finalization vote
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct FinalizationVote {
    slot: Slot,
    block_id: Hash,
    _replayed_slot: Slot,
    replayed_bank_hash: Hash,
}

impl FinalizationVote {
    fn new_internal(finalization_vote: &FinalizationVoteInstructionData) -> Self {
        Self {
            slot: Slot::from(finalization_vote.slot),
            block_id: finalization_vote.block_id,
            _replayed_slot: 0,
            replayed_bank_hash: finalization_vote.replayed_bank_hash,
        }
    }

    /// Construct a finalization vote for `slot`
    pub fn new(slot: Slot, block_id: Hash, _replayed_slot: Slot, replayed_bank_hash: Hash) -> Self {
        Self {
            slot,
            block_id,
            _replayed_slot,
            replayed_bank_hash,
        }
    }

    /// The slot to notarize
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// The block_id of the notarization slot
    pub fn block_id(&self) -> &Hash {
        &self.block_id
    }

    /// The bank hash of the latest replayed slot
    pub fn replayed_bank_hash(&self) -> &Hash {
        &self.replayed_bank_hash
    }
}

/// A skip vote
/// Represents a range of slots to skip
/// inclusive on both ends
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct SkipVote {
    pub(crate) start_slot: Slot,
    pub(crate) end_slot: Slot,
}

impl SkipVote {
    fn new_internal(skip_vote: &SkipVoteInstructionData) -> Self {
        Self {
            start_slot: Slot::from(skip_vote.start_slot),
            end_slot: Slot::from(skip_vote.end_slot),
        }
    }

    /// Construct a skip vote for `[start_slot, end_slot]`
    pub fn new(start_slot: Slot, end_slot: Slot) -> Self {
        Self {
            start_slot,
            end_slot,
        }
    }

    /// The inclusive on both ends range of slots to skip
    pub fn skip_range(&self) -> RangeInclusive<Slot> {
        self.start_slot..=self.end_slot
    }
}
