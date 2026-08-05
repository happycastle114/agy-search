//! Typed Antigravity release-version preflight and shared invocation deadline.

use std::{fmt, path::PathBuf};

use tokio::time::{Duration, Instant};

use crate::{
    error::AgyError,
    process::{ProcessRequest, run},
    types::TimeoutSeconds,
};

const MINIMUM_VERSION: AgyVersion = AgyVersion::new(1, 1, 10);

/// One end-to-end deadline shared by all downstream preflight and content work.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Deadline(Instant);

impl Deadline {
    /// Start a deadline from the user-provided end-to-end timeout.
    pub(crate) fn after(timeout: TimeoutSeconds) -> Self {
        Self(Instant::now() + timeout.duration())
    }

    /// Return the remaining budget, failing closed after its elapsed instant.
    pub(crate) fn remaining(self) -> Result<Duration, AgyError> {
        self.0
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or(AgyError::Timeout)
    }

    /// Expose the fixed instant for existing deadline-aware downstream helpers.
    pub(crate) const fn instant(self) -> Instant {
        self.0
    }
}

/// One official bare `X.Y.Z` release version with semantic numeric ordering.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct AgyVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl AgyVersion {
    const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    fn parse_decimal(value: Option<&str>) -> Result<u64, AgyError> {
        let value = value.ok_or(AgyError::OutputInvalid)?;
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(AgyError::OutputInvalid);
        }
        value.parse::<u64>().map_err(|_| AgyError::OutputInvalid)
    }

    fn parse_output(output: &[u8]) -> Result<Self, AgyError> {
        let text = std::str::from_utf8(output).map_err(|_| AgyError::OutputInvalid)?;
        let line = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .ok_or(AgyError::OutputInvalid)?;
        if line.is_empty() || line.contains(char::is_whitespace) {
            return Err(AgyError::OutputInvalid);
        }
        let mut components = line.split('.');
        let parsed = Self::new(
            Self::parse_decimal(components.next())?,
            Self::parse_decimal(components.next())?,
            Self::parse_decimal(components.next())?,
        );
        if components.next().is_some() {
            return Err(AgyError::OutputInvalid);
        }
        Ok(parsed)
    }
}

impl fmt::Display for AgyVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Verify the installed CLI release before any content or availability claim.
pub(crate) async fn require_supported(
    executable: &str,
    cwd: PathBuf,
    deadline: Deadline,
) -> Result<AgyVersion, AgyError> {
    let output = run(ProcessRequest {
        argv: vec![executable.to_owned(), "--version".to_owned()],
        cwd,
        timeout: deadline.remaining()?,
    })
    .await?;
    let version = AgyVersion::parse_output(&output.stdout)?;
    if version < MINIMUM_VERSION {
        Err(AgyError::OutputInvalid)
    } else {
        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_semantic_official_release_versions() {
        assert!(matches!(
            AgyVersion::parse_output(b"1.1.10\n"),
            Ok(version) if version == AgyVersion::new(1, 1, 10)
        ));
        assert!(
            AgyVersion::parse_output(b"1.1.9\n").is_ok_and(|version| version < MINIMUM_VERSION)
        );
        assert!(
            AgyVersion::parse_output(b"2.0.0\r\n").is_ok_and(|version| version > MINIMUM_VERSION)
        );
    }

    #[test]
    fn rejects_non_official_or_ambiguous_release_payloads() {
        for output in [
            b"agy 1.1.10\n".as_slice(),
            b"1.1\n",
            b"1.1.10-rc.1\n",
            b"1.01.10\n",
            b"1.1.10\nextra\n",
        ] {
            assert!(matches!(
                AgyVersion::parse_output(output),
                Err(AgyError::OutputInvalid)
            ));
        }
    }
}
