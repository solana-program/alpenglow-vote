//! Program state

use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_pod::primitives::{PodI64, PodU64};

use crate::accounting::{AuthorizedVoter, EpochCredit};
use crate::instruction::InitializeAccountInstructionData;

pub(crate) type PodEpoch = PodU64;
type PodSlot = PodU64;
type PodUnixTimestamp = PodI64;

/// The accounting and vote information associated with
/// this vote account
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default, PartialEq)]
pub struct VoteState {
    /// The current vote state version
    pub version: u8,

    /// The node that votes in this account
    pub node_pubkey: Pubkey,

    /// The signer for withdrawals
    pub authorized_withdrawer: Pubkey,

    /// Percentage (0-100) that represents what part of a rewards
    /// payout should be given to this VoteAccount
    pub commission: u8,

    /// The signer for vote transactions in this epoch
    pub authorized_voter: AuthorizedVoter,

    /// The signer for vote transaction in an upcoming epoch
    pub next_authorized_voter: Option<AuthorizedVoter>,

    /// How many credits this validator is earning in this Epoch
    pub epoch_credits: EpochCredit,

    /// Most recent timestamp submitted with a vote
    pub last_timestamp: BlockTimestamp,
}

/// Represents the time at which a block was voted on
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default, PartialEq)]
pub struct BlockTimestamp {
    /// Slot of the voted on block
    pub slot: PodSlot,
    /// Unix timestamp for when the vote was cast
    pub timestamp: PodUnixTimestamp,
}

impl VoteState {
    const VOTE_STATE_VERSION: u8 = 1;

    pub(crate) fn new(init_data: &InitializeAccountInstructionData, clock: &Clock) -> Self {
        Self {
            version: Self::VOTE_STATE_VERSION,
            node_pubkey: init_data.node_pubkey,
            authorized_voter: AuthorizedVoter {
                epoch: PodU64::from(clock.epoch),
                voter: init_data.authorized_voter,
            },
            next_authorized_voter: None,
            authorized_withdrawer: init_data.authorized_withdrawer,
            commission: init_data.commission,
            ..VoteState::default()
        }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        self.version > 0
    }

    pub(crate) fn set_vote_account_state(
        vote_account: &AccountInfo,
        vote_state: &VoteState,
    ) -> Result<(), ProgramError> {
        if u64::from(vote_state.authorized_voter.epoch) == 0 {
            // TODO: put this in a better place
            return Err(ProgramError::InvalidArgument);
        }
        vote_account
            .try_borrow_mut_data()?
            .copy_from_slice(bytemuck::bytes_of(vote_state));
        Ok(())
    }

    /// The size of the vote account that stores this VoteState
    pub const fn size() -> usize {
        std::mem::size_of::<VoteState>()
    }
}
