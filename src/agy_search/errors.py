"""Sanitized typed failures shared by CLI trust boundaries."""

from dataclasses import dataclass
from typing import override


class AgySearchError(Exception):
    """Base class for expected standalone CLI failures."""


@dataclass(slots=True)
class InvalidModelSlugError(AgySearchError):
    """A requested model cannot be represented as one argv value."""

    reason: str

    @override
    def __str__(self) -> str:
        """Return a stable public message."""
        return "invalid model slug"


@dataclass(slots=True)
class InvalidCommandError(AgySearchError):
    """A downstream command contract is invalid."""

    reason: str

    @override
    def __str__(self) -> str:
        """Return a stable public message."""
        return "invalid agy command"


@dataclass(slots=True)
class ExecutableUnavailableError(AgySearchError):
    """The configured downstream executable could not be launched."""

    executable: str
    reason: str

    @override
    def __str__(self) -> str:
        """Hide local executable paths from public output."""
        return "agy unavailable"


@dataclass(slots=True)
class ProcessTimeoutError(AgySearchError):
    """A bounded downstream invocation exceeded its deadline."""

    timeout_seconds: float

    @override
    def __str__(self) -> str:
        """Return a stable public message."""
        return "agy timed out"


@dataclass(slots=True)
class ProcessFailedError(AgySearchError):
    """The downstream process exited unsuccessfully."""

    returncode: int
    stderr: bytes

    @override
    def __str__(self) -> str:
        """Keep downstream diagnostics out of public output."""
        return "agy process failed"


@dataclass(slots=True)
class OutputInvalidError(AgySearchError):
    """Antigravity output failed the source-backed response contract."""

    reason: str

    @override
    def __str__(self) -> str:
        """Keep private model output out of public diagnostics."""
        return "agy output invalid"


@dataclass(slots=True)
class UnknownModelError(AgySearchError):
    """A requested model was absent from live model discovery."""

    model: str

    @override
    def __str__(self) -> str:
        """Return a stable public message without listing account models."""
        return "unknown agy model"


@dataclass(slots=True)
class OutputWriteError(AgySearchError):
    """A requested JSON output target could not be written atomically."""

    reason: str

    @override
    def __str__(self) -> str:
        """Return a stable public message without exposing local paths."""
        return "output write failed"
