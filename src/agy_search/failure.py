"""Exhaustive mapping from typed failures to stable public exit codes."""

from agy_search.enums import CliExitCode
from agy_search.errors import (
    AgySearchError,
    ExecutableUnavailableError,
    InvalidCommandError,
    InvalidModelSlugError,
    OutputInvalidError,
    OutputWriteError,
    ProcessFailedError,
    ProcessTimeoutError,
    UnknownModelError,
)


def exit_code_for_error(error: AgySearchError) -> CliExitCode:
    """Classify one sanitized failure without inspecting message strings."""
    match error:
        case ExecutableUnavailableError():
            code = CliExitCode.UNAVAILABLE
        case ProcessTimeoutError():
            code = CliExitCode.TIMEOUT
        case ProcessFailedError():
            code = CliExitCode.UPSTREAM
        case OutputInvalidError():
            code = CliExitCode.OUTPUT
        case UnknownModelError():
            code = CliExitCode.MODEL
        case InvalidCommandError() | InvalidModelSlugError() | OutputWriteError():
            code = CliExitCode.USAGE
        case _:
            code = CliExitCode.UPSTREAM
    return code
