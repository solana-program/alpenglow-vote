//! Alpenglow compute unit benchmark testing.

use {
    alpenglow_vote::{
        instruction::{finalize, notarize, skip},
        state::VoteState,
        vote::{FinalizationVote, NotarizationVote, SkipVote},
    },
    mollusk_svm::Mollusk,
    mollusk_svm_bencher::MolluskComputeUnitBencher,
    solana_sdk::{account::Account, pubkey::Pubkey},
};

const BENCHMARK_OUT_DIR: &str = "./benches";
const SBF_OUT_DIR: &str = "../target/deploy";

fn vote_account(authority: &Pubkey) -> Account {
    VoteState::create_account_with_authorized(&Pubkey::new_unique(), authority, authority, 0, 0)
        .into()
}

fn main() {
    std::env::set_var("SBF_OUT_DIR", SBF_OUT_DIR);

    let mut mollusk = Mollusk::new(&alpenglow_vote::id(), "alpenglow_vote");

    let slot = 5;
    mollusk.warp_to_slot(slot + 1);

    let bank_hash = *mollusk.sysvars.slot_hashes.get(&slot).unwrap();

    MolluskComputeUnitBencher::new(mollusk)
        .bench({
            let vote_address = Pubkey::new_unique();
            let authority = Pubkey::new_unique();
            let vote = FinalizationVote::new(slot, bank_hash, slot, bank_hash);
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
            let vote = NotarizationVote::new(slot, bank_hash, slot, bank_hash);
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
            let vote = SkipVote::new(slot);
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
