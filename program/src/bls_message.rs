//! Put BLS message here so all clients can agree on the format
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use {
    crate::{
        certificate::{CertificateMessage, CertificateType},
        vote::Vote,
    },
    bitvec::prelude::*,
    solana_bls::Signature as BLSSignature,
    solana_hash::Hash,
    solana_program::clock::Slot,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
/// BLS vote message, we need rank to look up pubkey
pub struct VoteMessage {
    /// The vote
    pub vote: Vote,
    /// The signature
    pub signature: BLSSignature,
    /// The rank of the validator
    pub rank: u16,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
/// BLS message data in Alpenglow
pub enum BLSMessage {
    /// Vote message, with the vote and the rank of the validator.
    Vote(VoteMessage),
    /// Certificate message
    Certificate(CertificateMessage),
}

impl BLSMessage {
    /// Create a new vote message
    pub fn new_vote(vote: Vote, signature: BLSSignature, rank: u16) -> Self {
        Self::Vote(VoteMessage {
            vote,
            signature,
            rank,
        })
    }

    /// Create a new certificate message
    pub fn new_certificate(
        certificate_type: CertificateType,
        slot: Slot,
        block_id: Option<Hash>,
        replayed_bank_hash: Option<Hash>,
        bitmap: BitVec<u8, Lsb0>,
        signature: BLSSignature,
    ) -> Self {
        Self::Certificate(CertificateMessage {
            certificate_type,
            slot,
            block_id,
            replayed_bank_hash,
            signature,
            bitmap,
        })
    }

    #[cfg(feature = "serde")]
    /// Deserialize a BLS message from bytes
    pub fn deserialize(bls_message_in_bytes: &[u8]) -> Self {
        bincode::deserialize(bls_message_in_bytes).unwrap()
    }

    #[cfg(feature = "serde")]
    /// Serialize a BLS message to bytes
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
