#![cfg(feature = "test-sbf")]

use {
    alpenglow_vote::{
        accounting::EpochCredit,
        instruction::{self, AuthorityType, InitializeAccountInstructionData},
        state::VoteState,
    },
    mollusk_svm::Mollusk,
    rand::Rng,
    solana_program::pubkey::Pubkey,
    solana_sdk::{
        account::Account,
        clock::{Epoch, Slot},
        instruction::Instruction,
        signature::{Keypair, Signer},
    },
    spl_pod::bytemuck::pod_from_bytes,
};

const SLOT: Slot = 53_084_024;
const EPOCH: Epoch = 100;

fn initialize_vote_account_mollusk(
    vote_account: &Keypair,
    node_key: &Keypair,
    authorized_voter: &Pubkey,
    authorized_withdrawer: &Pubkey,
    commission: u8,
) -> Instruction {
    instruction::initialize_account(
        vote_account.pubkey(),
        &InitializeAccountInstructionData {
            node_pubkey: node_key.pubkey(),
            authorized_voter: *authorized_voter,
            authorized_withdrawer: *authorized_withdrawer,
            commission,
        },
    )
}

fn setup_clock_mollusk(mollusk: &mut Mollusk, slot: Option<Slot>) {
    // TODO: use warp_to_slot()
    let clock = &mut mollusk.sysvars.clock;
    clock.slot = slot.unwrap_or(SLOT);
    clock.epoch = EPOCH;
}

fn build_mollusk() -> Mollusk {
    Mollusk::new(&alpenglow_vote::id(), "alpenglow_vote")
}

fn build_mollusk_with_clock(slot: Option<Slot>) -> Mollusk {
    let mut mollusk = build_mollusk();
    setup_clock_mollusk(&mut mollusk, slot);
    mollusk
}

fn build_empty_vote_account(mollusk: &Mollusk) -> Account {
    build_empty_vote_account_with_excess_lamports(mollusk, 0)
}

fn build_empty_vote_account_with_excess_lamports(
    mollusk: &Mollusk,
    excess_lamports: u64,
) -> Account {
    let vote_account_lamports = mollusk
        .sysvars
        .rent
        .minimum_balance(VoteState::size())
        .saturating_add(excess_lamports);
    Account::new(
        vote_account_lamports,
        VoteState::size(),
        &alpenglow_vote::id(),
    )
}

#[test]
fn test_initialize_vote_account_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();
    let commission: u8 = rand::rng().random();

    let instruction = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        commission,
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();

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
}

#[test]
fn test_authorize_voter_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize(
        vote_account.pubkey(),
        authorized_voter.pubkey(),
        new_authority.pubkey(),
        AuthorityType::Voter,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_voter.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();
    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();

    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[test]
fn test_authorize_withdrawer_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize(
        vote_account.pubkey(),
        authorized_withdrawer.pubkey(),
        new_authority.pubkey(),
        AuthorityType::Withdrawer,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();
    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();

    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[test]
fn test_authorize_checked_voter_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize_checked(
        vote_account.pubkey(),
        authorized_voter.pubkey(),
        new_authority.pubkey(),
        AuthorityType::Voter,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_voter.pubkey(), Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();
    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();

    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[test]
fn test_authorize_checked_withdrawer_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    let new_authority = Keypair::new();

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize_checked(
        vote_account.pubkey(),
        authorized_withdrawer.pubkey(),
        new_authority.pubkey(),
        AuthorityType::Withdrawer,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer.pubkey(), Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();
    let vote_state: &VoteState = pod_from_bytes(&vote_account.data).unwrap();

    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[test]
fn test_authorize_with_seed_voter_basic() {
    let mollusk = build_mollusk_with_clock(None);

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
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize_with_seed(
        vote_account.pubkey(),
        base_key.pubkey(),
        owner.pubkey(),
        voter_seed,
        new_authority.pubkey(),
        AuthorityType::Voter,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (base_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_voter, Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[test]
fn test_authorize_with_seed_withdrawer_basic() {
    let mollusk = build_mollusk_with_clock(None);

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
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize_with_seed(
        vote_account.pubkey(),
        base_key.pubkey(),
        owner.pubkey(),
        withdrawer_seed,
        new_authority.pubkey(),
        AuthorityType::Withdrawer,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (base_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer, Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[test]
fn test_authorize_checked_with_seed_voter_basic() {
    let mollusk = build_mollusk_with_clock(None);

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
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    let authorize_ixn = instruction::authorize_checked_with_seed(
        vote_account.pubkey(),
        base_key.pubkey(),
        owner.pubkey(),
        voter_seed,
        new_authority.pubkey(),
        AuthorityType::Voter,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (base_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_voter, Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(
        Some(new_authority.pubkey()),
        vote_state.next_authorized_voter().map(|nav| *nav.voter()),
    );
}

#[test]
fn test_authorize_checked_with_seed_withdrawer_basic() {
    let mollusk = build_mollusk_with_clock(None);

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
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter,
        &authorized_withdrawer,
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert!(vote_state.next_authorized_voter().is_none());

    // Issue an Authorize transaction
    //
    let authorize_ixn = instruction::authorize_checked_with_seed(
        vote_account.pubkey(),
        base_key.pubkey(),
        owner.pubkey(),
        withdrawer_seed,
        new_authority.pubkey(),
        AuthorityType::Withdrawer,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, authorize_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (base_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer, Account::default()),
            (new_authority.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();
    assert_eq!(new_authority.pubkey(), *vote_state.authorized_withdrawer());
}

#[test]
fn test_update_commission_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    // Create a vote account with known commission
    let commission_before: u8 = 42;
    let commission_after: u8 = 69;

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        commission_before,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(42, vote_state.commission());

    // Issue an UpdateCommission transaction
    let update_commission_txn = instruction::update_commission(
        vote_account.pubkey(),
        authorized_withdrawer.pubkey(),
        commission_after,
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, update_commission_txn],
        &[
            (node_key.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(commission_after, vote_state.commission());
}

#[test]
fn test_update_validator_identity_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let old_node = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();

    // This (probably) won't fail (p is very low - if it fails, you probably win something)
    let new_node = Keypair::new();
    assert_ne!(old_node, new_node);

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &old_node,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (old_node.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(old_node.pubkey(), *vote_state.node_pubkey());

    // Issue an UpdateValidatorIdentity transaction
    let update_vi_txn = instruction::update_validator_identity(
        vote_account.pubkey(),
        authorized_withdrawer.pubkey(),
        new_node.pubkey(),
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, update_vi_txn],
        &[
            (old_node.pubkey(), Account::default()),
            (vote_account.pubkey(), build_empty_vote_account(&mollusk)),
            (authorized_withdrawer.pubkey(), Account::default()),
            (new_node.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    let vote_state: &VoteState =
        pod_from_bytes(&result.get_account(&vote_account.pubkey()).unwrap().data).unwrap();

    assert_eq!(new_node.pubkey(), *vote_state.node_pubkey());
}

#[test]
fn test_withdraw_basic() {
    let mollusk = build_mollusk_with_clock(None);

    let vote_account = Keypair::new();
    let node_key = Keypair::new();
    let authorized_voter = Keypair::new();
    let authorized_withdrawer = Keypair::new();
    let recipient_account = Keypair::new();

    // Create a vote account
    let initialize_ixn = initialize_vote_account_mollusk(
        &vote_account,
        &node_key,
        &authorized_voter.pubkey(),
        &authorized_withdrawer.pubkey(),
        42,
    );

    let result = mollusk.process_instruction(
        &initialize_ixn,
        &[
            (node_key.pubkey(), Account::default()),
            (
                vote_account.pubkey(),
                build_empty_vote_account_with_excess_lamports(&mollusk, 1_234_567),
            ),
        ],
    );

    assert!(result.raw_result.is_ok());

    let account = result.get_account(&vote_account.pubkey()).unwrap();

    let rent_exempt_amount = 2_359_440;
    assert_eq!(rent_exempt_amount + 1_234_567, account.lamports);

    // Issue a Withdraw transaction
    let withdraw_ixn = instruction::withdraw(
        vote_account.pubkey(),
        authorized_withdrawer.pubkey(),
        1_234_567,
        recipient_account.pubkey(),
    );

    let result = mollusk.process_instruction_chain(
        &[initialize_ixn, withdraw_ixn],
        &[
            (node_key.pubkey(), Account::default()),
            (
                vote_account.pubkey(),
                build_empty_vote_account_with_excess_lamports(&mollusk, 1_234_567),
            ),
            (authorized_withdrawer.pubkey(), Account::default()),
            (recipient_account.pubkey(), Account::default()),
        ],
    );

    assert!(result.raw_result.is_ok());

    // Ensure that the vote account has the right balance
    let vote_account = result.get_account(&vote_account.pubkey()).unwrap();
    assert_eq!(rent_exempt_amount, vote_account.lamports);

    // Ensure that the recipient account has the right balance
    let recipient_account = result.get_account(&recipient_account.pubkey()).unwrap();
    assert_eq!(1_234_567, recipient_account.lamports);
}
