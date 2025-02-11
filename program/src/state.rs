//! Program state

use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::clock::Slot;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_pod::primitives::{PodI64, PodU64};

use crate::accounting::{AuthorizedVoter, EpochCredit};
use crate::instruction::InitializeAccountInstructionData;

pub(crate) type PodEpoch = PodU64;
pub(crate) type PodSlot = PodU64;
pub(crate) type PodUnixTimestamp = PodI64;

/// The accounting and vote information associated with
/// this vote account
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default, PartialEq)]
pub struct VoteState {
    /// The current vote state version
    pub(crate) version: u8,

    /// The node that votes in this account
    pub(crate) node_pubkey: Pubkey,

    /// The signer for withdrawals
    pub(crate) authorized_withdrawer: Pubkey,

    /// Percentage (0-100) that represents what part of a rewards
    /// payout should be given to this VoteAccount
    pub(crate) commission: u8,

    /// The signer for vote transactions in this epoch
    pub(crate) authorized_voter: AuthorizedVoter,

    /// The signer for vote transaction in an upcoming epoch
    pub(crate) next_authorized_voter: Option<AuthorizedVoter>,

    /// How many credits this validator is earning in this Epoch
    pub(crate) epoch_credits: EpochCredit,

    /// Most recent timestamp submitted with a vote
    pub(crate) latest_timestamp: BlockTimestamp,

    /// The latest notarized slot
    pub(crate) latest_notarized_slot: PodSlot,

    /// The latest notarized block_id
    pub(crate) latest_notarized_block_id: Hash,

    /// The latest notarized bank_hash
    pub(crate) latest_notarized_bank_hash: Hash,

    /// The latest finalized slot
    pub(crate) latest_finalized_slot: PodSlot,

    /// The latest finalized block_id
    pub(crate) latest_finalized_block_id: Hash,

    /// The latest finalized bank_hash
    pub(crate) latest_finalized_bank_hash: Hash,

    /// The latest skip range start slot
    pub(crate) latest_skip_start_slot: PodSlot,

    /// The latest skip range end slot (inclusive)
    pub(crate) latest_skip_end_slot: PodSlot,

    /// The slot of the latest replayed block
    /// Only relevant after APE
    pub(crate) _replayed_slot: PodSlot,

    /// The bank hash of the latest replayed block
    /// Only relevant after APE
    pub(crate) _replayed_bank_hash: Hash,
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

    /// Vote state version
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Validator that votes in this account
    pub fn node_pubkey(&self) -> &Pubkey {
        &self.node_pubkey
    }

    /// Signer for withdrawals
    pub fn authorized_withdrawer(&self) -> &Pubkey {
        &self.authorized_withdrawer
    }

    /// Percentage (0-100) that represents what part of a rewards
    /// payout should be given to this VoteAccount
    pub fn commission(&self) -> u8 {
        self.commission
    }

    /// The signer for vote transactions in this epoch
    pub fn authorized_voter(&self) -> &AuthorizedVoter {
        &self.authorized_voter
    }

    /// The signer for vote transactions in an upcoming epoch
    pub fn next_authorized_voter(&self) -> Option<&AuthorizedVoter> {
        self.next_authorized_voter.as_ref()
    }

    /// How many credits this validator is earning in this Epoch
    pub fn epoch_credits(&self) -> &EpochCredit {
        &self.epoch_credits
    }

    /// Most recent timestamp submitted with a vote
    pub fn latest_timestamp(&self) -> &BlockTimestamp {
        &self.latest_timestamp
    }

    /// The latest notarized slot
    pub fn latest_notarized_slot(&self) -> Slot {
        Slot::from(self.latest_notarized_slot)
    }

    /// The latest notarized block_id
    pub fn latest_notarized_block_id(&self) -> &Hash {
        &self.latest_notarized_block_id
    }

    /// The latest notarized bank_hash
    pub fn latest_notarized_bank_hash(&self) -> &Hash {
        &self.latest_notarized_bank_hash
    }

    /// The latest finalized slot
    pub fn latest_finalized_slot(&self) -> Slot {
        Slot::from(self.latest_finalized_slot)
    }

    /// The latest finalized block_id
    pub fn latest_finalized_block_id(&self) -> &Hash {
        &self.latest_finalized_block_id
    }

    /// The latest notarized bank_hash
    pub fn latest_finalized_bank_hash(&self) -> &Hash {
        &self.latest_finalized_bank_hash
    }

    /// The latest skip range start slot
    pub fn latest_skip_start_slot(&self) -> Slot {
        Slot::from(self.latest_skip_start_slot)
    }

    /// The latest skip range end slot
    pub fn latest_skip_end_slot(&self) -> Slot {
        Slot::from(self.latest_skip_end_slot)
    }

    /// Set the node_pubkey
    pub fn set_node_pubkey(&mut self, node_pubkey: Pubkey) {
        self.node_pubkey = node_pubkey
    }

    /// Set the authorized withdrawer
    pub fn set_authorized_withdrawer(&mut self, authorized_withdrawer: Pubkey) {
        self.authorized_withdrawer = authorized_withdrawer
    }

    /// Set the commission
    pub fn set_commission(&mut self, commission: u8) {
        self.commission = commission
    }

    /// Set the authorized voter
    pub fn set_authorized_voter(&mut self, authorized_voter: AuthorizedVoter) {
        self.authorized_voter = authorized_voter
    }

    /// Set the next authorized voter
    pub fn set_next_authorized_voter(&mut self, next_authorized_voter: AuthorizedVoter) {
        self.next_authorized_voter = Some(next_authorized_voter)
    }

    /// Set the epoch credits
    pub fn set_epoch_credits(&mut self, epoch_credits: EpochCredit) {
        self.epoch_credits = epoch_credits
    }

    /// Set the latest timestamp
    pub fn set_latest_timestamp(&mut self, latest_timestamp: BlockTimestamp) {
        self.latest_timestamp = latest_timestamp
    }

    /// Set the latest notarized slot
    pub fn set_latest_notarized_slot(&mut self, latest_notarized_slot: Slot) {
        self.latest_notarized_slot = PodSlot::from(latest_notarized_slot)
    }

    /// Set the latest notarized block id
    pub fn set_latest_notarized_block_id(&mut self, latest_notarized_block_id: Hash) {
        self.latest_notarized_block_id = latest_notarized_block_id
    }

    /// Set the latest notarized bank hash
    pub fn set_latest_notarized_bank_hash(&mut self, latest_notarized_bank_hash: Hash) {
        self.latest_notarized_bank_hash = latest_notarized_bank_hash
    }

    /// Set the latest finalized slot
    pub fn set_latest_finalized_slot(&mut self, latest_finalized_slot: Slot) {
        self.latest_finalized_slot = PodSlot::from(latest_finalized_slot)
    }

    /// Set the latest finalized block id
    pub fn set_latest_finalized_block_id(&mut self, latest_finalized_block_id: Hash) {
        self.latest_finalized_block_id = latest_finalized_block_id
    }

    /// Set the latest finalized bank hash
    pub fn set_latest_finalized_bank_hash(&mut self, latest_finalized_bank_hash: Hash) {
        self.latest_finalized_bank_hash = latest_finalized_bank_hash
    }

    /// Set the latest skip start slot
    pub fn set_latest_skip_start_slot(&mut self, latest_skip_start_slot: Slot) {
        self.latest_skip_start_slot = PodSlot::from(latest_skip_start_slot)
    }

    /// Set the latest skip end slot
    pub fn set_latest_skip_end_slot(&mut self, latest_skip_end_slot: Slot) {
        self.latest_skip_end_slot = PodSlot::from(latest_skip_end_slot)
    }
}
