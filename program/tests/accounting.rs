#![cfg(feature = "test-sbf")]

use {
    alpenglow_vote::{
        accounting::EpochCredit,
        instruction::{self, InitializeAccountInstructionData},
        processor::process_instruction,
        state::{BlockTimestamp, VoteState},
    },
    rand::Rng,
    solana_program::pubkey::Pubkey,
    solana_program_test::*,
    solana_sdk::{
        clock::{Clock, Epoch, Slot},
        rent::Rent,
        signature::{Keypair, Signer},
        system_instruction,
        transaction::Transaction,
    },
    spl_pod::bytemuck::pod_from_bytes,
};

fn program_test() -> ProgramTest {
    ProgramTest::new(
        "alpenglow_vote",
        alpenglow_vote::id(),
        processor!(process_instruction),
    )
}

const SLOT: Slot = 53084024;
const EPOCH: Epoch = 100;

async fn setup_clock(context: &mut ProgramTestContext, slot: Option<Slot>) {
    let clock: Clock = context.banks_client.get_sysvar().await.unwrap();
    let mut new_clock = clock.clone();
    new_clock.slot = slot.unwrap_or(SLOT);
    new_clock.epoch = EPOCH;
    context.set_sysvar(&new_clock);
}

async fn initialize_vote_account(
    context: &mut ProgramTestContext,
    vote_account: &Keypair,
    node_key: &Keypair,
    authorized_voter: &Pubkey,
    authorized_withdrawer: &Pubkey,
    commission: u8,
) {
    let account_length = VoteState::size();
    println!("Creating an account of size {account_length}");
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &vote_account.pubkey(),
                1.max(Rent::default().minimum_balance(account_length)),
                account_length as u64,
                &alpenglow_vote::id(),
            ),
            instruction::initialize_account(
                vote_account.pubkey(),
                &InitializeAccountInstructionData {
                    node_pubkey: node_key.pubkey(),
                    authorized_voter: *authorized_voter,
                    authorized_withdrawer: *authorized_withdrawer,
                    commission,
                },
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, vote_account, node_key],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_initialize_vote_account() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();
    let commission: u8 = rand::rng().random();

    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        commission,
    )
    .await;

    let vote_account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();
    assert_eq!(1, vote_state.version);
    assert_eq!(node_key.pubkey(), vote_state.node_pubkey);
    assert_eq!(
        authorized_withdrawer.pubkey(),
        vote_state.authorized_withdrawer
    );
    assert_eq!(commission, vote_state.commission);
    assert_eq!(authorized_voter.pubkey(), vote_state.authorized_voter.voter);
    assert_eq!(EPOCH, u64::from(vote_state.authorized_voter.epoch));
    assert_eq!(None, vote_state.next_authorized_voter);
    assert_eq!(EpochCredit::default(), vote_state.epoch_credits);
    assert_eq!(BlockTimestamp::default(), vote_state.last_timestamp);
}
