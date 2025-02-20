//! Alpenglow Vote program
#![deny(missing_docs)]
#![feature(custom_test_frameworks)]
pub mod accounting;
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod vote;
mod vote_processor;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;

solana_program::declare_id!("Vote222222222222222222222222222222222222222");
