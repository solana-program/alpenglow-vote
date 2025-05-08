//! Alpenglow compute unit benchmark testing.

use {
    alpenglow_vote::{
        instruction::{finalize, notarize, skip},
        state::VoteState,
        vote::{FinalizationVote, NotarizationVote, SkipVote},
    },
    mollusk_svm::Mollusk,
    mollusk_svm_bencher::MolluskComputeUnitBencher,
    solana_bls::Pubkey as BLSPubkey,
    solana_hash::Hash,
    solana_sdk::{account::Account, clock::Clock, pubkey::Pubkey},
};

const BENCHMARK_OUT_DIR: &str = "./benches";
const SBF_OUT_DIR: &str = "../target/deploy";

fn vote_account(authority: &Pubkey) -> Account {
    VoteState::create_account_with_authorized(&Pubkey::new_unique(), authority, authority, 0, 0, BLSPubkey::default())
        .into()
}

fn main() {
    std::env::set_var("SBF_OUT_DIR", SBF_OUT_DIR);

    let mut mollusk = Mollusk::new(&alpenglow_vote::id(), "alpenglow_vote");

    let clock_slot = 6;
    let vote_slot = 5;
    let skip_slot = 4;
    // Setup fork not including 4
    mollusk.warp_to_slot(skip_slot - 1);
    let epoch = mollusk.sysvars.epoch_schedule.get_epoch(clock_slot);
    let leader_schedule_epoch = mollusk
        .sysvars
        .epoch_schedule
        .get_leader_schedule_epoch(clock_slot);
    mollusk.sysvars.clock = Clock {
        slot: clock_slot,
        epoch,
        leader_schedule_epoch,
        ..Default::default()
    };
    mollusk
        .sysvars
        .slot_hashes
        .add(vote_slot, Hash::new_unique());

    let bank_hash = *mollusk.sysvars.slot_hashes.get(&vote_slot).unwrap();

    MolluskComputeUnitBencher::new(mollusk)
        .bench({
            let vote_address = Pubkey::new_unique();
            let authority = Pubkey::new_unique();
            let vote = FinalizationVote::new(vote_slot);
            (
                "finalize",
                &finalize(vote_address, authority, &vote),
                &[
                    (vote_address, vote_account(&authority)),
                    (authority, Account::default()),
                ],
            )
        })
        .bench({
            let vote_address = Pubkey::new_unique();
            let authority = Pubkey::new_unique();
            let vote = NotarizationVote::new(vote_slot, bank_hash, vote_slot, bank_hash);
            (
                "notarize",
                &notarize(vote_address, authority, &vote),
                &[
                    (vote_address, vote_account(&authority)),
                    (authority, Account::default()),
                ],
            )
        })
        .bench({
            let vote_address = Pubkey::new_unique();
            let authority = Pubkey::new_unique();
            let vote = SkipVote::new(skip_slot);
            (
                "skip",
                &skip(vote_address, authority, &vote),
                &[
                    (vote_address, vote_account(&authority)),
                    (authority, Account::default()),
                ],
            )
        })
        .must_pass(true)
        .out_dir(BENCHMARK_OUT_DIR)
        .execute();
}
