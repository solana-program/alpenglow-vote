//! Define certificte types here for now, maybe move later if we process
//! certificates off chain

#[derive(Clone, Copy, Debug, PartialEq)]
/// Certificate Type in Alpenglow
pub enum CertificateType {
    /// Finalize slow: >= 60% Finalize
    Finalize,
    /// Finalize fast: >= 80% Notarize
    FinalizeFast,
    /// Notarize: >= 60% Notarize
    Notarize,
    /// Notarize fallback: >= 60% Notarize or NotarizeFallback
    NotarizeFallback,
    /// Skip: >= 60% Skip or SkipFallback
    Skip,
}
