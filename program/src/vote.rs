//! Vote data types for use by clients

use solana_program::clock::{Slot, UnixTimestamp};
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;

use crate::instruction::{decode_instruction_data, decode_instruction_type, VoteInstruction};
use crate::vote_processor::{
    FinalizationVoteInstructionData, NotarizationVoteInstructionData, SkipVoteInstructionData,
};

/// Enum that clients can use to parse and create the vote
/// structures expected by the program
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Vote {
    /// A notarization vote
    NotarizationVote(NotarizationVote),
    /// A finalization vote
    FinalizationVote(FinalizationVote),
    /// A skip vote
    SkipVote(SkipVote),
}

macro_rules! dispatch {
    (
        $(#[$($attrss:tt)*])*
        $vis:vis fn $name:ident(&self $(, $arg:ident : $ty:ty)?) $(-> $out:ty)?
    ) => {
        $(#[$($attrss)*])*
        #[inline]
        $vis fn $name(&self $(, $arg:$ty)?) $(-> $out)? {
            match self {
                Self::NotarizationVote(shred) => shred.$name($($arg, )?),
                Self::FinalizationVote(shred) => shred.$name($($arg, )?),
                Self::SkipVote(shred) => shred.$name($($arg, )?),
            }
        }
    };
}

impl Vote {
    dispatch! {
        /// The time at which this vote was created
        pub fn timestamp(&self) -> UnixTimestamp
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
}

impl From<NotarizationVote> for Vote {
    fn from(vote: NotarizationVote) -> Self {
        Self::NotarizationVote(vote)
    }
}

impl From<FinalizationVote> for Vote {
    fn from(vote: FinalizationVote) -> Self {
        Self::FinalizationVote(vote)
    }
}

impl From<SkipVote> for Vote {
    fn from(vote: SkipVote) -> Self {
        Self::SkipVote(vote)
    }
}

/// A notarization vote
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotarizationVote {
    slot: Slot,
    block_id: Hash,
    _replayed_slot: Slot,
    replayed_bank_hash: Hash,
    timestamp: UnixTimestamp,
}

impl NotarizationVote {
    fn new_internal(notarization_vote: &NotarizationVoteInstructionData) -> Self {
        Self {
            slot: Slot::from(notarization_vote.slot),
            block_id: notarization_vote.block_id,
            _replayed_slot: 0,
            replayed_bank_hash: notarization_vote.replayed_bank_hash,
            timestamp: UnixTimestamp::from(notarization_vote.timestamp),
        }
    }

    /// Construct a notarization vote for `slot`
    pub fn new(
        slot: Slot,
        block_id: Hash,
        _replayed_slot: Slot,
        replayed_bank_hash: Hash,
        timestamp: UnixTimestamp,
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
    pub fn timestamp(&self) -> UnixTimestamp {
        self.timestamp
    }
}

/// A finalization vote
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FinalizationVote {
    slot: Slot,
    block_id: Hash,
    _replayed_slot: Slot,
    replayed_bank_hash: Hash,
    timestamp: UnixTimestamp,
}

impl FinalizationVote {
    fn new_internal(finalization_vote: &FinalizationVoteInstructionData) -> Self {
        Self {
            slot: Slot::from(finalization_vote.slot),
            block_id: finalization_vote.block_id,
            _replayed_slot: 0,
            replayed_bank_hash: finalization_vote.replayed_bank_hash,
            timestamp: UnixTimestamp::from(finalization_vote.timestamp),
        }
    }

    /// Construct a finalization vote for `slot`
    pub fn new(
        slot: Slot,
        block_id: Hash,
        _replayed_slot: Slot,
        replayed_bank_hash: Hash,
        timestamp: UnixTimestamp,
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
    pub fn timestamp(&self) -> UnixTimestamp {
        self.timestamp
    }
}

/// A skip vote
/// Represents a range of slots to skip
/// inclusive on both ends
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkipVote {
    start_slot: Slot,
    end_slot: Slot,
    timestamp: UnixTimestamp,
}

impl SkipVote {
    fn new_internal(skip_vote: &SkipVoteInstructionData) -> Self {
        Self {
            start_slot: Slot::from(skip_vote.start_slot),
            end_slot: Slot::from(skip_vote.end_slot),
            timestamp: UnixTimestamp::from(skip_vote.timestamp),
        }
    }

    /// Construct a skip vote for `[start_slot, end_slot]`
    pub fn new(start_slot: Slot, end_slot: Slot, timestamp: UnixTimestamp) -> Self {
        Self {
            start_slot,
            end_slot,
            timestamp,
        }
    }

    /// The inclusive on both ends range of slots to skip
    pub fn skip_range(&self) -> (Slot, Slot) {
        (self.start_slot, self.end_slot)
    }

    /// The time at which this vote was created
    pub fn timestamp(&self) -> UnixTimestamp {
        self.timestamp
    }
}
