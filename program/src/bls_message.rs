//! Put BLS messages here so all clients can agree on the format
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
#[derive(Clone, Copy, Debug, PartialEq)]
/// BLS message vote data, we need rank to look up pubkey
pub struct BLSMessageVoteData {
    /// The vote
    pub vote: Vote,
    /// The rank of the validator
    pub rank: u16,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
/// BLS vote message to be sent all to all in Alpenglow
pub struct BLSVoteMessage {
    /// The vote data
    pub vote_data: BLSMessageVoteData,
    /// The signature of the message
    pub signature: BLSSignature,
}

impl BLSVoteMessage {
    /// Create a new vote message
    pub fn new_vote(vote: Vote, my_rank: u16, signature: BLSSignature) -> Self {
        Self {
            vote_data: BLSMessageVoteData {
                vote,
                rank: my_rank,
            },
            signature,
        }
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
/// BLS certificate message to be sent all to all in Alpenglow
pub struct BLSCertificateMessage {
    /// The certificate
    pub certificate: Certificate,
    /// The signature of the message
    pub signature: BLSSignature,
}

impl BLSCertificateMessage {
    /// Create a new certificate message
    pub fn new_certificate(
        certificate_type: CertificateType,
        slot: Slot,
        block_id: Option<Hash>,
        replayed_bank_hash: Option<Hash>,
        bitmap: BitVec<u8, Lsb0>,
        signature: BLSSignature,
    ) -> Self {
        Self {
            certificate: Certificate {
                certificate_type,
                slot,
                block_id,
                replayed_bank_hash,
                bitmap,
            },
            signature,
        }
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
