//! Put BLS message here so all clients can agree on the format
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use {
    crate::{
        certificate::{Certificate, CertificateType},
        vote::Vote,
    },
    bitvec::prelude::*,
    solana_bls::Signature as BLSSignature,
    solana_hash::Hash,
    solana_program::clock::Slot,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
/// BLS message type in Alpenglow
pub enum BLSMessage {
    /// Vote message
    Vote(Vote),
    /// Certificate message
    Certificate(Certificate),
}

impl BLSMessage {
    /// Create a new vote message
    pub fn new_vote(vote: Vote) -> Self {
        Self::Vote(vote)
    }

    /// Create a new certificate message
    pub fn new_certificate(
        certificate_type: CertificateType,
        slot: Slot,
        block_id: Option<Hash>,
        replayed_bank_hash: Option<Hash>,
        signature: BLSSignature,
        bitmap: BitVec<u8, Lsb0>,
    ) -> Self {
        Self::Certificate(Certificate {
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
    pub fn deserialize_from(bls_message_in_bytes: &[u8]) -> Self {
        bincode::deserialize(bls_message_in_bytes).unwrap()
    }

    #[cfg(feature = "serde")]
    /// Serialize a BLS message to bytes
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}
