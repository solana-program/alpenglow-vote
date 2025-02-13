#![cfg(feature = "test-sbf")]

use {
    alpenglow_vote::{
        accounting::EpochCredit,
        instruction::{self, AuthorityType, InitializeAccountInstructionData},
        processor::process_instruction,
        state::VoteState,
    },
    rand::Rng,
    solana_program::pubkey::Pubkey,
    solana_program_test::*,
    solana_sdk::{
        clock::{Clock, Epoch, Slot, UnixTimestamp},
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

const SLOT: Slot = 53_084_024;
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
    excess_lamports: Option<u64>,
) {
    let account_length = VoteState::size();
    let rent: Rent = context.banks_client.get_sysvar().await.unwrap();

    let account_lamports = rent
        .minimum_balance(account_length)
        .saturating_add(excess_lamports.unwrap_or(0));

    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &vote_account.pubkey(),
                account_lamports,
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
async fn test_initialize_vote_account_basic() {
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
        None,
    )
    .await;

    let vote_account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();

    assert_eq!(1, vote_state.version());
    assert_eq!(node_key.pubkey(), *vote_state.node_pubkey());
    assert_eq!(
        authorized_withdrawer.pubkey(),
        *vote_state.authorized_withdrawer()
    );
    assert_eq!(commission, vote_state.commission());
    assert_eq!(
        authorized_voter.pubkey(),
        *vote_state.authorized_voter().voter()
    );
    assert_eq!(EPOCH, vote_state.authorized_voter().epoch());
    assert_eq!(None, vote_state.next_authorized_voter());
    assert_eq!(EpochCredit::default(), *vote_state.epoch_credits());
    assert_eq!(UnixTimestamp::from(0), vote_state.latest_timestamp());
}

#[tokio::test]
async fn test_authorize_voter_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize(
            vote_account.pubkey(),
            authorized_voter.pubkey(),
            new_authority.pubkey(),
            AuthorityType::Voter,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_voter],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[tokio::test]
async fn test_authorize_withdrawer_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize(
            vote_account.pubkey(),
            authorized_withdrawer.pubkey(),
            new_authority.pubkey(),
            AuthorityType::Withdrawer,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_withdrawer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[tokio::test]
async fn test_authorize_checked_voter_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_checked(
            vote_account.pubkey(),
            authorized_voter.pubkey(),
            new_authority.pubkey(),
            AuthorityType::Voter,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_voter, &new_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[tokio::test]
async fn test_authorize_checked_withdrawer_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_checked(
            vote_account.pubkey(),
            authorized_withdrawer.pubkey(),
            new_authority.pubkey(),
            AuthorityType::Withdrawer,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_withdrawer, &new_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[tokio::test]
async fn test_authorize_with_seed_voter_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let owner = Keypair::new();
    let base_key = Keypair::new();
    let voter_seed = "voter-thequickbrownfox";
    let withdrawer_seed = "withdrawer-thequickbrownfox";

    let vote_account = Keypair::new();
    let node_key = Keypair::new();

    let authorized_voter =
        Pubkey::create_with_seed(&base_key.pubkey(), voter_seed, &owner.pubkey()).unwrap();

    let authorized_withdrawer =
        Pubkey::create_with_seed(&base_key.pubkey(), withdrawer_seed, &owner.pubkey()).unwrap();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_with_seed(
            vote_account.pubkey(),
            base_key.pubkey(),
            owner.pubkey(),
            voter_seed,
            new_authority.pubkey(),
            AuthorityType::Voter,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &base_key],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[tokio::test]
async fn test_authorize_with_seed_withdrawer_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let owner = Keypair::new();
    let base_key = Keypair::new();
    let voter_seed = "voter-thequickbrownfox";
    let withdrawer_seed = "withdrawer-thequickbrownfox";

    let vote_account = Keypair::new();
    let node_key = Keypair::new();

    let authorized_voter =
        Pubkey::create_with_seed(&base_key.pubkey(), voter_seed, &owner.pubkey()).unwrap();

    let authorized_withdrawer =
        Pubkey::create_with_seed(&base_key.pubkey(), withdrawer_seed, &owner.pubkey()).unwrap();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_with_seed(
            vote_account.pubkey(),
            base_key.pubkey(),
            owner.pubkey(),
            withdrawer_seed,
            new_authority.pubkey(),
            AuthorityType::Withdrawer,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &base_key],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[tokio::test]
async fn test_authorize_checked_with_seed_voter_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let owner = Keypair::new();
    let base_key = Keypair::new();
    let voter_seed = "voter-thequickbrownfox";
    let withdrawer_seed = "withdrawer-thequickbrownfox";

    let vote_account = Keypair::new();
    let node_key = Keypair::new();

    let authorized_voter =
        Pubkey::create_with_seed(&base_key.pubkey(), voter_seed, &owner.pubkey()).unwrap();

    let authorized_withdrawer =
        Pubkey::create_with_seed(&base_key.pubkey(), withdrawer_seed, &owner.pubkey()).unwrap();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_checked_with_seed(
            vote_account.pubkey(),
            base_key.pubkey(),
            owner.pubkey(),
            voter_seed,
            new_authority.pubkey(),
            AuthorityType::Voter,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &base_key, &new_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[tokio::test]
async fn test_authorize_checked_with_seed_withdrawer_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let owner = Keypair::new();
    let base_key = Keypair::new();
    let voter_seed = "voter-thequickbrownfox";
    let withdrawer_seed = "withdrawer-thequickbrownfox";

    let vote_account = Keypair::new();
    let node_key = Keypair::new();

    let authorized_voter =
        Pubkey::create_with_seed(&base_key.pubkey(), voter_seed, &owner.pubkey()).unwrap();

    let authorized_withdrawer =
        Pubkey::create_with_seed(&base_key.pubkey(), withdrawer_seed, &owner.pubkey()).unwrap();

    let new_authority = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_txn = Transaction::new_signed_with_payer(
        &[instruction::authorize_checked_with_seed(
            vote_account.pubkey(),
            base_key.pubkey(),
            owner.pubkey(),
            withdrawer_seed,
            new_authority.pubkey(),
            AuthorityType::Withdrawer,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &base_key, &new_authority],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(authorize_txn)
        .await
        .unwrap();

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();
    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[tokio::test]
async fn test_update_commission_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    // Create a vote account with known commission
    let commission_before: u8 = 42;
    let commission_after: u8 = 69;

    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        commission_before,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert_eq!(42, vote_state.commission());

    // Issue an UpdateCommission transaction
    let update_commission_txn = Transaction::new_signed_with_payer(
        &[instruction::update_commission(
            vote_account.pubkey(),
            authorized_withdrawer.pubkey(),
            commission_after,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_withdrawer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(update_commission_txn)
        .await
        .unwrap();

    // Ensure that the set commission mastches
    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert_eq!(69, vote_state.commission());
}

#[tokio::test]
async fn test_update_validator_identity_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    // This (probably) won't fail (p is very low)
    let new_node_key = Keypair::new();
    assert_ne!(node_key, new_node_key);

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        None,
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert_eq!(node_key.pubkey(), *vote_state.node_pubkey());

    // Issue an UpdateValidatorIdentity transaction
    let update_vi_txn = Transaction::new_signed_with_payer(
        &[instruction::update_validator_identity(
            vote_account.pubkey(),
            authorized_withdrawer.pubkey(),
            new_node_key.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &new_node_key, &authorized_withdrawer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(update_vi_txn)
        .await
        .unwrap();

    // Ensure that the set commission mastches
    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    let vote_state: &VoteState = pod_from_bytes(&account.data).unwrap();

    assert_eq!(new_node_key.pubkey(), *vote_state.node_pubkey());
}

#[tokio::test]
async fn test_withdraw_basic() {
    let mut context = program_test().start_with_context().await;
    setup_clock(&mut context, None).await;

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();
    let recipient_account = Keypair::new();

    // Create a vote account
    initialize_vote_account(
        &mut context,
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
        Some(1_234_567),
    )
    .await;

    let account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // 3584400 is the rent exempt amount
    assert_eq!(3_584_400 + 1_234_567, account.lamports);

    // Issue a Withdraw transaction
    let txn = Transaction::new_signed_with_payer(
        &[instruction::withdraw(
            vote_account.pubkey(),
            authorized_withdrawer.pubkey(),
            1_234_567,
            recipient_account.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &authorized_withdrawer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(txn).await.unwrap();

    // Ensure that the vote account has the right balance
    let vote_account = context
        .banks_client
        .get_account(vote_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(3_584_400, vote_account.lamports);

    // Ensure that the recipient account has the right balance
    let recipient_account = context
        .banks_client
        .get_account(recipient_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(1_234_567, recipient_account.lamports);
}
