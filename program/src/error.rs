//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the program.
#[derive(Clone, Copy, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VoteError {
    /// Closing an active vote account
    #[error("Cannot close vote account unless it stopped voting at least one full epoch ago")]
    ActiveVoteAccountClose,

    /// Increasing commission too late into the epoch
    #[error("Cannot update commission at this point in the epoch")]
    CommissionUpdateTooLate,

    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Invalid Vote Authorize enum
    #[error("Invalid vote authorize")]
    InvalidAuthorizeType,

    /// Missing epoch schedule sysvar
    #[error("Missing epoch schedule sysvar")]
    MissingEpochScheduleSysvar,

    /// Missing slot hashes sysvar
    #[error("Missing slot hashes sysvar")]
    MissingSlotHashesSysvar,

    /// Replay bank hash mismatch
    #[error("Replay bank hash mismatch")]
    ReplayBankHashMismatch,

    /// Skip end slot exceeds clock slot
    #[error("Skip end slot exceeds clock slot")]
    SkipEndSlotExceedsCurrentSlot,

    /// Skip end slot is lower than the skip start slot
    #[error("Skip end slot is lower than the skip start slot")]
    SkipEndSlotLowerThanSkipStartSlot,

    /// New skip range overlaps with previous
    #[error("Skip range overlaps")]
    SkipRangeOverlaps,

    /// Skip slot range contains finalization vote
    #[error("Skip slot range contains finalization vote")]
    SkipSlotRangeContainsFinalizationVote,

    /// Slot hashes is missing the replayed slot key
    #[error("Slot hashes is missing the replayed slot key")]
    SlotHashesMissingKey,

    /// Timestamp is too old
    #[error("Timestamp is too old")]
    TimestampTooOld,

    /// Version mismatch
    #[error("Version mismatch")]
    VersionMismatch,

    /// Vote too old (notarization / finalization votes aren't monotonic)
    #[error("Notarization / finalization vote isn't monotonic")]
    VoteTooOld,
}

impl From<VoteError> for ProgramError {
    fn from(e: VoteError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VoteError {
    fn type_of() -> &'static str {
        "Vote Error"
    }
}
