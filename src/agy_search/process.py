"""Bounded shell-free execution and marker-owned workspace isolation."""

import shutil
import tempfile
from collections.abc import Generator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, final

import anyio

from agy_search.errors import (
    ExecutableUnavailableError,
    ProcessFailedError,
    ProcessTimeoutError,
)

_DIRECTORY_PREFIX = "agy-search-"
_OWNERSHIP_MARKER = ".agy-search-owned"


@dataclass(frozen=True, slots=True)
class ProcessRequest:
    """One bounded direct-argv invocation."""

    command: tuple[str, ...]
    cwd: Path
    timeout_seconds: float


@dataclass(frozen=True, slots=True)
class ProcessOutput:
    """Captured output from a successful downstream process."""

    stdout: bytes
    stderr: bytes
    returncode: int


class ProcessRunner(Protocol):
    """Port for shell-free process execution."""

    async def run(self, request: ProcessRequest) -> ProcessOutput:
        """Execute one request or raise a typed failure."""
        ...


@final
class AnyioProcessRunner:
    """Production runner with a shared concurrency bound."""

    def __init__(self, max_concurrency: int) -> None:
        """Create a runner with one positive process concurrency bound."""
        self._limiter = anyio.CapacityLimiter(max_concurrency)

    async def run(self, request: ProcessRequest) -> ProcessOutput:
        """Run one command without a shell and enforce its deadline."""
        try:
            async with self._limiter:
                with anyio.fail_after(request.timeout_seconds):
                    completed = await anyio.run_process(
                        request.command,
                        cwd=request.cwd,
                        check=False,
                    )
        except TimeoutError as error:
            raise ProcessTimeoutError(timeout_seconds=request.timeout_seconds) from error
        except OSError as error:
            raise ExecutableUnavailableError(
                executable=request.command[0],
                reason=type(error).__name__,
            ) from error

        if completed.returncode != 0:
            raise ProcessFailedError(
                returncode=completed.returncode,
                stderr=completed.stderr,
            )
        return ProcessOutput(
            stdout=completed.stdout,
            stderr=completed.stderr,
            returncode=completed.returncode,
        )


@contextmanager
def isolated_directory(base: Path) -> Generator[Path, None, None]:
    """Create and remove one exact marker-owned isolation directory."""
    base.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix=_DIRECTORY_PREFIX, dir=base))
    marker = directory / _OWNERSHIP_MARKER
    marker.touch(exist_ok=False)
    try:
        yield directory
    finally:
        _remove_owned_directory(directory=directory, base=base)


def _remove_owned_directory(*, directory: Path, base: Path) -> None:
    resolved_directory = directory.resolve()
    resolved_base = base.resolve()
    marker = resolved_directory / _OWNERSHIP_MARKER
    is_exact_child = resolved_directory.parent == resolved_base
    is_named_for_package = resolved_directory.name.startswith(_DIRECTORY_PREFIX)
    if is_exact_child and is_named_for_package and marker.is_file():
        shutil.rmtree(resolved_directory)
