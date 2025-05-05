//! Define certificate types here for now, maybe move later if we process
//! certificates off chain

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Certificate Type in Alpenglow
pub enum CertificateType {
    /// Finalize slow: at least 60 percent Finalize
    Finalize,
    /// Finalize fast: at least 80 percent Notarize
    FinalizeFast,
    /// Notarize: at least 60 percent Notarize
    Notarize,
    /// Notarize fallback: at least 60 percent Notarize or NotarizeFallback
    NotarizeFallback,
    /// Skip: at least 60 percent Skip or SkipFallback
    Skip,
}
