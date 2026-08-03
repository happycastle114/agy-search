"""Typed direct-argv contract for the official Antigravity CLI."""

from dataclasses import dataclass

from agy_search.enums import AgyEffort, AgyOutputFormat, AgyRunMode
from agy_search.errors import InvalidCommandError, InvalidModelSlugError


@dataclass(frozen=True, slots=True)
class ModelSlug:
    """One dynamically discovered Antigravity model slug."""

    value: str

    def __post_init__(self) -> None:
        """Reject values that cannot be one direct argument."""
        if (
            not self.value
            or self.value != self.value.strip()
            or any(character.isspace() for character in self.value)
        ):
            raise InvalidModelSlugError(reason="model slug must be one non-empty argument")


@dataclass(frozen=True, slots=True)
class AgyPrintRequest:
    """One schema-constrained Antigravity print invocation."""

    prompt: str
    print_timeout: str
    json_schema: str
    model: ModelSlug | None = None
    effort: AgyEffort | None = None
    mode: AgyRunMode = AgyRunMode.PLAN
    output_format: AgyOutputFormat = AgyOutputFormat.STREAM_JSON


@dataclass(frozen=True, slots=True)
class AgyCommandBuilder:
    """Build shell-free argv tuples for one official executable."""

    executable: str

    def __post_init__(self) -> None:
        """Require one configured executable."""
        if not self.executable.strip():
            raise InvalidCommandError(reason="agy executable must be non-empty")

    def print_argv(self, request: AgyPrintRequest) -> tuple[str, ...]:
        """Build a plan-mode schema-constrained print invocation."""
        command = [
            self.executable,
            "--mode",
            request.mode.value,
            "--print-timeout",
            request.print_timeout,
            "--output-format",
            request.output_format.value,
            "--json-schema",
            request.json_schema,
        ]
        if request.model is not None:
            command.extend(("--model", request.model.value))
        if request.effort is not None:
            command.extend(("--effort", request.effort.value))
        command.extend(("-p", request.prompt))
        return tuple(command)

    def models_argv(self) -> tuple[str, ...]:
        """Build the official dynamic model-discovery invocation."""
        return (self.executable, "models")

    def version_argv(self) -> tuple[str, ...]:
        """Build the official version-discovery invocation."""
        return (self.executable, "--version")
