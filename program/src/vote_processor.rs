use std::cmp::Ordering;

use bytemuck::{Pod, Zeroable};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::clock::Slot;
use solana_program::clock::UnixTimestamp;
use solana_program::epoch_schedule::EpochSchedule;
use solana_program::hash::Hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::slot_hashes::PodSlotHashes;

use crate::error::VoteError;
use crate::state::BlockTimestamp;
use crate::state::{PodSlot, PodUnixTimestamp, VoteState};

pub(crate) const CURRENT_NOTARIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_FINALIZE_VOTE_VERSION: u8 = 1;
pub(crate) const CURRENT_SKIP_VOTE_VERSION: u8 = 1;

/// Number of slots of grace period for which maximum vote credits are awarded - votes landing
/// within this number of slots of the slot that is being voted on are awarded full credits.
pub const VOTE_CREDITS_GRACE_SLOTS: u64 = 2;

/// Maximum number of credits to award for a vote; this number of credits is awarded to votes on
/// slots that land within the grace period. After that grace period, vote credits are reduced.
pub const VOTE_CREDITS_MAXIMUM_PER_SLOT: u64 = 16;

/// A notarization vote, the data expected by
/// `VoteInstruction::Notarize`
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub timestamp: Option<PodUnixTimestamp>,
}

// SAFETY: for our purposes we treat a zero timestamp as the validator not
// supplying a timestamp, so timestamp is safe to be zeroable
unsafe impl Zeroable for NotarizationVoteInstructionData {}
unsafe impl Pod for NotarizationVoteInstructionData {}

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
}

fn replay_bank_hash_checks(
    replayed_slot: Slot,
    replayed_bank_hash: Hash,
    slot_hashes: &PodSlotHashes,
) -> Result<(), VoteError> {
    // We must have already executed `replayed_slot` and stored the associated bank hash
    // (error out otherwise). Ensure that our bank hash matches what we observe.
    if replayed_bank_hash
        != slot_hashes
            .get(&replayed_slot)
            .map_err(|_| VoteError::MissingSlotHashesSysvar)?
            .ok_or(VoteError::SlotHashesMissingKey)?
    {
        Err(VoteError::ReplayBankHashMismatch)
    } else {
        Ok(())
    }
}

/// Credits are awarded as a piece-wise linear function; up to a certain amount of block latency,
/// the vote program awards the maximum number of credits. Then, the number of awarded credits goes
/// down at a rate of 1 credit per block. The minimum number of awarded credits is 1.
fn latency_to_credits(latency: u64) -> u64 {
    let (kink_lo, kink_hi) = (
        VOTE_CREDITS_GRACE_SLOTS,
        VOTE_CREDITS_MAXIMUM_PER_SLOT + VOTE_CREDITS_GRACE_SLOTS - 1,
    );

    if latency <= kink_lo {
        VOTE_CREDITS_MAXIMUM_PER_SLOT
    } else if kink_lo < latency && latency <= kink_hi {
        // NOTE: checked_sub isn't necessary, since latency < kink_hi
        kink_hi.saturating_add(1).saturating_sub(latency)
    } else {
        1
    }
}

/// Suppose that we, as the vote program, observe a notarization vote `vote` from some validator
/// `v`. We need to determine the number of credits to award `v` for having issued this
/// notarization vote.
///
/// Further, suppose that we are receiving the vote on slot `clock.slot`; here, `clock` refers to
/// the `clock` sysvar we invoke as the vote program. And, suppose that `v` issued the vote for
/// slot `vote.slot`. The math works out as follows:
///
/// Case 1. The vote is associated with a slot later than our slot. I.e., `vote.slot > clock.slot`.
///
/// If this is a notarization / finalization vote, bank hash checks should have failed, since
/// "vote.slot" would not be in our slot hashes. So, we can just return an "unreachable" error.
///
/// If this is a skip vote, the vote should have been discarded.
///
/// Case 2. `clock.slot >= vote.slot`
///
/// We define latency as `clock.slot - vote.slot`. I.e., after how many slots did the vote reach me?
/// We then use the function `latency_to_credits(...)` to determine the correct number of credits to
/// award. See `latency_to_credits` to see how this works.
///
/// This logic ends up being common to both notarization and finalization, so we factor it out in
/// the function `award_credits` below.
///
/// After determining the number of credits to award, we need to update our vote state. See
/// below for how this is done.
fn award_credits(
    vote_state: &mut VoteState,
    vote_slot: u64,
    clock: &Clock,
    epoch_schedule: &EpochSchedule,
) -> Result<(), ProgramError> {
    // Calculate credits to be awarded for this vote
    let earned_credits = match clock.slot.checked_sub(vote_slot) {
        Some(latency) => latency_to_credits(latency),

        // The only way in which this can happen is if vote_slot > clock.slot. This cannot happen,
        // because:
        //
        // Case 1. We're awarding credits for a notarization / finalization vote.
        //
        // In this case, the bank hash checks would have errored out.
        //
        // Case 2. We're awarding credits for a skip vote.
        //
        // Prior to processing credits, we error out if vote_end_slot > clock.slot, guaranteeing
        // that, at this point here, all vote_slots are <= clock.slot.
        //
        // So, let's return an error.
        None => return Err(VoteError::Unreachable.into()),
    };

    let vote_epoch = epoch_schedule.get_epoch(vote_slot);

    let epoch_credits = &mut vote_state.epoch_credits;

    match vote_epoch.cmp(&epoch_credits.epoch()) {
        Ordering::Equal => {
            epoch_credits.set_credits(epoch_credits.credits().saturating_add(earned_credits));
            Ok(())
        }
        Ordering::Less => {
            // We can't have that vote_epoch < epoch_credits.epoch(), since that would imply that
            // we've received a vote for a slot that is lesser than what we've received in the
            // past, and we would have returned an error prior to award_credits having been called.
            Err(VoteError::Unreachable.into())
        }
        Ordering::Greater => {
            let prev_credits = epoch_credits
                .prev_credits()
                .saturating_add(epoch_credits.credits());

            epoch_credits.set_epoch(vote_epoch);
            epoch_credits.set_prev_credits(prev_credits);
            epoch_credits.set_credits(earned_credits);
            Ok(())
        }
    }
}

/// Processing skip credits
fn process_skip_credits(
    vote_state: &mut VoteState,
    vote_start_slot: u64,
    vote_end_slot: u64,
    clock: &Clock,
    slot_hashes: &PodSlotHashes,
    epoch_schedule: &EpochSchedule,
) -> Result<(), ProgramError> {
    let eligible_skip_start = vote_state
        .latest_skip_end_slot()
        .saturating_add(1)
        .max(vote_start_slot);

    for skip_slot in eligible_skip_start..vote_end_slot {
        let hash = slot_hashes
            .get(&skip_slot)
            .map_err(|_| VoteError::MissingSlotHashesSysvar)?;

        // Observing a valid slot hash for the slot `skip_slot` indicates that `skip_slot` was
        // not skipped on this fork. Only award credits to skip votes associated with slots that
        // were skipped.
        if hash.is_none() {
            award_credits(vote_state, skip_slot, clock, epoch_schedule)?;
        }
    }

    Ok(())
}

pub(crate) fn process_notarization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    clock: &Clock,
    slot_hashes: &PodSlotHashes,
    epoch_schedule: &EpochSchedule,
    vote: &NotarizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    let vote_slot = vote.slot.into();

    if vote.version != CURRENT_NOTARIZE_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Notarization votes must be strictly increasing
    if vote_slot <= vote_state.latest_notarized_slot() && vote_state.latest_notarized_slot() != 0 {
        return Err(VoteError::VoteTooOld.into());
    }

    replay_bank_hash_checks(vote_slot, vote.replayed_bank_hash, slot_hashes)?;

    vote_state.latest_notarized_slot = vote.slot;
    vote_state.latest_notarized_block_id = vote.block_id;
    vote_state.latest_notarized_bank_hash = vote.replayed_bank_hash;

    if let Some(timestamp) = vote.timestamp.map(UnixTimestamp::from) {
        if timestamp != 0 && timestamp > vote_state.latest_timestamp().timestamp() {
            vote_state.latest_timestamp = BlockTimestamp {
                slot: vote.slot,
                timestamp: vote
                    .timestamp
                    .expect("timestamp is verified to be not None above"),
            };
        } else {
            return Err(VoteError::TimestampTooOld.into());
        }
    }

    award_credits(vote_state, vote_slot, clock, epoch_schedule)?;

    Ok(())
}

pub(crate) fn process_finalization_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    clock: &Clock,
    slot_hashes: &PodSlotHashes,
    epoch_schedule: &EpochSchedule,
    vote: &FinalizationVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    let vote_slot = vote.slot.into();

    if vote.version != CURRENT_FINALIZE_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

    if vote_state.authorized_voter.voter != *vote_authority {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if vote_slot <= vote_state.latest_finalized_slot() {
        return Err(VoteError::VoteTooOld.into());
    }

    if vote_state.latest_skip_start_slot() <= vote_slot
        && vote_slot <= vote_state.latest_skip_end_slot()
    {
        return Err(VoteError::SkipSlotRangeContainsFinalizationVote.into());
    }

    replay_bank_hash_checks(vote_slot, vote.replayed_bank_hash, slot_hashes)?;

    vote_state.latest_finalized_slot = vote.slot;
    vote_state.latest_finalized_block_id = vote.block_id;
    vote_state.latest_finalized_bank_hash = vote.replayed_bank_hash;

    award_credits(vote_state, vote_slot, clock, epoch_schedule)?;

    Ok(())
}

pub(crate) fn process_skip_vote(
    vote_account: &AccountInfo,
    vote_authority: &Pubkey,
    clock: &Clock,
    slot_hashes: &PodSlotHashes,
    epoch_schedule: &EpochSchedule,
    vote: &SkipVoteInstructionData,
) -> Result<(), ProgramError> {
    let mut vote_state = vote_account.data.borrow_mut();
    let vote_state = bytemuck::from_bytes_mut::<VoteState>(&mut vote_state);

    if vote.version != CURRENT_SKIP_VOTE_VERSION {
        return Err(VoteError::VersionMismatch.into());
    }

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

    // Discard votes strictly after clock.slot
    let vote_end_slot = vote.end_slot.into();

    if vote_end_slot > clock.slot {
        return Err(VoteError::SkipEndSlotExceedsCurrentSlot.into());
    }

    process_skip_credits(
        vote_state,
        vote.start_slot.into(),
        vote_end_slot,
        clock,
        slot_hashes,
        epoch_schedule,
    )?;

    vote_state.latest_skip_start_slot = vote.start_slot;
    vote_state.latest_skip_end_slot = vote.end_slot;

    Ok(())
}

#[cfg(test)]
mod tests {
    use solana_sdk::epoch_schedule::EpochSchedule;
    use solana_sdk::program_error::ProgramError;
    use solana_sdk::{clock::Clock, pubkey::Pubkey};
    use spl_pod::primitives::PodU64;
    use test_case::test_case;

    use crate::accounting::EpochCredit;
    use crate::error::VoteError;
    use crate::vote_processor::award_credits;
    use crate::{
        instruction::InitializeAccountInstructionData,
        state::VoteState,
        vote_processor::{
            latency_to_credits, VOTE_CREDITS_GRACE_SLOTS, VOTE_CREDITS_MAXIMUM_PER_SLOT,
        },
    };

    #[test]
    fn test_parity_old_vote_program() {
        assert_eq!(
            VOTE_CREDITS_GRACE_SLOTS,
            solana_sdk::vote::state::VOTE_CREDITS_GRACE_SLOTS as u64
        );
        assert_eq!(
            VOTE_CREDITS_MAXIMUM_PER_SLOT,
            solana_sdk::vote::state::VOTE_CREDITS_MAXIMUM_PER_SLOT as u64
        );
    }

    #[test]
    fn test_latency_to_credits_max_credits() {
        for latency in 0..=VOTE_CREDITS_GRACE_SLOTS {
            assert_eq!(VOTE_CREDITS_MAXIMUM_PER_SLOT, latency_to_credits(latency));
        }
    }

    #[test]
    fn test_latency_to_credits_ramp_down() {
        for latency in 3..=VOTE_CREDITS_MAXIMUM_PER_SLOT + 1 {
            assert_eq!(
                VOTE_CREDITS_MAXIMUM_PER_SLOT - (latency - 2),
                latency_to_credits(latency)
            );
        }
    }

    #[test]
    fn test_latency_to_credits_min() {
        for latency in [18, 20, 100, 1_000, 10_000, 100_000] {
            assert_eq!(1, latency_to_credits(latency));
        }
    }

    fn setup_vote_state(clock: &Clock) -> VoteState {
        let iaid = InitializeAccountInstructionData {
            node_pubkey: Pubkey::new_unique(),
            authorized_voter: Pubkey::new_unique(),
            authorized_withdrawer: Pubkey::new_unique(),
            commission: 0_u8,
        };

        VoteState::new(&iaid, clock)
    }

    #[test]
    fn test_award_credits_vote_slot_cannot_be_after_clock_slot() {
        let vote_slot = 1024_u64;
        let clock = Clock {
            slot: 512,
            epoch: 256,
            ..Clock::default()
        };

        let mut vote_state = setup_vote_state(&clock);
        let result = award_credits(
            &mut vote_state,
            vote_slot,
            &clock,
            &EpochSchedule::default(),
        );

        assert!(result.is_err());
        assert_eq!(
            ProgramError::from(VoteError::Unreachable),
            result.unwrap_err()
        );
    }

    fn epoch_to_starting_slot(epoch: u64) -> u64 {
        epoch
            .saturating_sub(14)
            .saturating_mul(432_000)
            .saturating_add(524_256)
    }

    #[test_case(1; "one")]
    #[test_case(2; "two")]
    #[test_case(5; "five")]
    #[test_case(20; "twenty")]
    fn test_award_credits_vote_epoch_equals_epoch_init(latency: u64) {
        let clock = Clock {
            slot: epoch_to_starting_slot(256).saturating_add(latency),
            epoch: 256,
            ..Clock::default()
        };
        let mut vote_state = setup_vote_state(&clock);

        let vote_slot = epoch_to_starting_slot(256);

        assert_eq!(0, vote_state.epoch_credits.epoch());
        assert_eq!(0, vote_state.epoch_credits.prev_credits());
        assert_eq!(0, vote_state.epoch_credits.credits());

        assert!(award_credits(
            &mut vote_state,
            vote_slot,
            &clock,
            &EpochSchedule::default()
        )
        .is_ok());
        assert_eq!(latency, clock.slot.saturating_sub(vote_slot));

        let expected_earned_credits = latency_to_credits(latency);

        assert_eq!(256, vote_state.epoch_credits.epoch());
        assert_eq!(0, vote_state.epoch_credits.prev_credits());
        assert_eq!(expected_earned_credits, vote_state.epoch_credits.credits());
    }

    #[test_case(1; "one")]
    #[test_case(2; "two")]
    #[test_case(5; "five")]
    #[test_case(20; "twenty")]
    fn test_award_credits_vote_epoch_equals_clock_epoch_intermediate(latency: u64) {
        let clock = Clock {
            slot: epoch_to_starting_slot(256).saturating_add(latency),
            epoch: 256,
            ..Clock::default()
        };
        let epoch_schedule = EpochSchedule::default();
        let mut vote_state = setup_vote_state(&clock);
        let vote_slot = epoch_to_starting_slot(256);

        assert_eq!(256, epoch_schedule.get_epoch(clock.slot));
        assert_eq!(256, epoch_schedule.get_epoch(vote_slot));

        vote_state.epoch_credits = EpochCredit {
            epoch: PodU64::from(256),
            credits: PodU64::from(123),
            prev_credits: PodU64::from(234),
        };

        assert!(award_credits(&mut vote_state, vote_slot, &clock, &epoch_schedule).is_ok());
        assert_eq!(latency, clock.slot.saturating_sub(vote_slot));

        let expected_earned_credits = latency_to_credits(clock.slot.saturating_sub(vote_slot));

        assert_eq!(256, vote_state.epoch_credits.epoch());
        assert_eq!(234, vote_state.epoch_credits.prev_credits());
        assert_eq!(
            expected_earned_credits.saturating_add(123),
            vote_state.epoch_credits.credits()
        );
    }

    #[test_case(1; "one")]
    #[test_case(2; "two")]
    #[test_case(5; "five")]
    #[test_case(20; "twenty")]
    fn test_award_credits_vote_epoch_greater_than_clock_epoch(latency: u64) {
        let clock = Clock {
            slot: epoch_to_starting_slot(256),
            epoch: 256,
            ..Clock::default()
        };
        let epoch_schedule = EpochSchedule::default();
        let mut vote_state = setup_vote_state(&clock);
        let vote_slot = epoch_to_starting_slot(256).saturating_sub(latency);

        assert_eq!(255, epoch_schedule.get_epoch(vote_slot));
        assert_eq!(256, epoch_schedule.get_epoch(clock.slot));

        vote_state.epoch_credits = EpochCredit {
            epoch: PodU64::from(12),
            credits: PodU64::from(123),
            prev_credits: PodU64::from(234),
        };

        assert!(award_credits(&mut vote_state, vote_slot, &clock, &epoch_schedule).is_ok());
        assert_eq!(latency, clock.slot.saturating_sub(vote_slot));

        let expected_earned_credits = latency_to_credits(clock.slot.saturating_sub(vote_slot));

        assert_eq!(255, vote_state.epoch_credits.epoch());
        assert_eq!(123 + 234, vote_state.epoch_credits.prev_credits());
        assert_eq!(expected_earned_credits, vote_state.epoch_credits.credits());
    }
}
