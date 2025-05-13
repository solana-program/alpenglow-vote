//! Put BLS message here so all clients can agree on the format

use {
    crate::{
        certificate::{Certificate, CertificateType},
        vote::Vote,
    },
    bit_vec::BitVec,
    solana_bls::Signature as BLSSignature,
    solana_hash::Hash,
    solana_program::clock::Slot,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
/// BLS message type in Alpenglow
pub enum BlsMessage {
    /// Vote message
    Vote(Vote),
    /// Certificate message
    Certificate(Box<Certificate>),
}

impl BlsMessage {
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
        bitmap: BitVec,
    ) -> Self {
        Self::Certificate(Box::new(Certificate {
            certificate_type,
            slot,
            block_id,
            replayed_bank_hash,
            signature,
            bitmap,
        }))
    }
}
