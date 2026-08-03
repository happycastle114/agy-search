//! Stable, sanitized failures at CLI trust boundaries.

use std::process::ExitCode;

use thiserror::Error;

#[derive(Debug, Error)]
/// Expected failures with stable, non-sensitive public messages.
pub enum AgyError {
    /// User input could not form one valid downstream request.
    #[error("invalid agy command")]
    InvalidCommand,
    /// The configured Antigravity executable could not launch.
    #[error("agy unavailable")]
    Unavailable,
    /// The downstream deadline elapsed.
    #[error("agy timed out")]
    Timeout,
    /// Antigravity returned a non-zero status.
    #[error("agy process failed")]
    ProcessFailed,
    /// Antigravity output did not satisfy the source contract.
    #[error("agy output invalid")]
    OutputInvalid,
    /// The selected model was not present in live model discovery.
    #[error("unknown agy model")]
    UnknownModel,
    /// The requested output target could not be replaced atomically.
    #[error("output write failed")]
    OutputWrite,
}

impl AgyError {
    /// Map a typed failure to the stable public process status.
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        let code = match self {
            Self::InvalidCommand | Self::OutputWrite => 2,
            Self::Unavailable => 3,
            Self::Timeout => 4,
            Self::ProcessFailed => 5,
            Self::OutputInvalid => 6,
            Self::UnknownModel => 7,
        };
        ExitCode::from(code)
    }
}
