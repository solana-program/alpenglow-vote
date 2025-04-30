//! BLS certificate instruction data
use bytemuck::{Pod, Zeroable};

/// Size of a BLS public key in an affine point representation
pub const BLS_PUBLIC_KEY_AFFINE_SIZE: usize = 96;

/// Size of a BLS signature in an affine point representation
pub const BLS_SIGNATURE_AFFINE_SIZE: usize = 192;

/// Currently we plan to support max of 4096 validators here.
pub const BLS_BITMAP_SIZE: usize = 512;

/// The BLS certificate type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BLSCertificateType {
    /// Fast finalization
    FastFinalization = 0,
    /// Finalization
    Finalization = 1,
    /// Notarization
    Notarization = 2,
    /// Notarization fallback
    NotarizationFallbck = 3,
    /// Skip
    Skip = 4,
}

/// The BLS certificate instruction data
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BLSCertificateInstructionData {
    /// The slot for the certificate
    pub slot: u64,
    /// The certificate type for the certificate
    pub certificate_type: BLSCertificateType,
    /// The BLS certificate
    pub bls_certificate: [u8; BLS_PUBLIC_KEY_AFFINE_SIZE],
    /// The BLS signature
    pub bls_signature: [u8; BLS_SIGNATURE_AFFINE_SIZE],
    /// The bitmap of validators
    pub validator_bitmap: [u8; BLS_BITMAP_SIZE],
}
unsafe impl Zeroable for BLSCertificateInstructionData {}
unsafe impl Pod for BLSCertificateInstructionData {}
