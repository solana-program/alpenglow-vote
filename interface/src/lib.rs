#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]
//! The [Aspenglow vote program][np].
//!
//! [np]: https://docs.solanalabs.com/runtime/programs#aspenglow-vote-program

pub mod authorized_voters;
pub mod error;
pub mod instruction;
pub mod state;

pub mod program {
    solana_pubkey::declare_id!("ALpengLowVote111111111111111111111111111111");
}
