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

    /// Decreasing commission too late into the epoch
    #[error("Cannot update commission at this point in the epoch")]
    CommissionUpdateTooLate,

    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Invalid Vote Authorize enum
    #[error("Invalid vote authorize")]
    InvalidAuthorizeType,
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
