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
#[allow(clippy::large_enum_variant)]
/// BLS message data in Alpenglow
pub enum BLSMessageData {
    /// Vote message, with the vote and the rank of the validator.
    Vote(BLSMessageVoteData),
    /// Certificate message
    Certificate(Certificate),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
/// BLS message to be sent all to all in Alpenglow
pub struct BLSMessage {
    /// The message data
    pub message_data: BLSMessageData,
    /// The signature of the message
    pub signature: BLSSignature,
}

impl BLSMessage {
    /// Create a new vote message
    pub fn new_vote(vote: Vote, my_rank: u16, signature: BLSSignature) -> Self {
        Self {
            message_data: BLSMessageData::Vote(BLSMessageVoteData {
                vote,
                rank: my_rank,
            }),
            signature,
        }
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
        Self {
            message_data: BLSMessageData::Certificate(Certificate {
                certificate_type,
                slot,
                block_id,
                replayed_bank_hash,
                bitmap,
            }),
            signature,
        }
    }
}
