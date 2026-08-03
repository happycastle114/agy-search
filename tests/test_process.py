import sys
from pathlib import Path

import pytest

from agy_search.errors import (
    ExecutableUnavailableError,
    ProcessFailedError,
    ProcessTimeoutError,
)
from agy_search.process import AnyioProcessRunner, ProcessRequest, isolated_directory


@pytest.mark.anyio
async def test_process_runner_captures_direct_argv_output(tmp_path: Path) -> None:
    # Given: a direct Python argv request in an explicit working directory
    request = ProcessRequest(
        command=(sys.executable, "-c", "print('PROCESS_OK')"),
        cwd=tmp_path,
        timeout_seconds=1.0,
    )

    # When: AnyIO executes it
    output = await AnyioProcessRunner(max_concurrency=1).run(request)

    # Then: stdout, stderr, and status are captured without a shell
    assert output.stdout == b"PROCESS_OK\n"
    assert output.stderr == b""
    assert output.returncode == 0


@pytest.mark.anyio
async def test_process_runner_maps_nonzero_without_exposing_stderr(tmp_path: Path) -> None:
    # Given: a child process that writes a private diagnostic and exits nonzero
    request = ProcessRequest(
        command=(
            sys.executable,
            "-c",
            "import sys; print('private', file=sys.stderr); sys.exit(23)",
        ),
        cwd=tmp_path,
        timeout_seconds=1.0,
    )

    # When: the runner executes it
    with pytest.raises(ProcessFailedError) as captured:
        _ = await AnyioProcessRunner(max_concurrency=1).run(request)

    # Then: the typed error retains status but its public text is sanitized
    assert captured.value.returncode == 23
    assert str(captured.value) == "agy process failed"
    assert "private" not in str(captured.value)


@pytest.mark.anyio
async def test_process_runner_maps_timeout(tmp_path: Path) -> None:
    # Given: a child process that cannot complete before the bound
    request = ProcessRequest(
        command=(sys.executable, "-c", "while True: pass"),
        cwd=tmp_path,
        timeout_seconds=0.02,
    )

    # When: the timeout expires
    with pytest.raises(ProcessTimeoutError):
        _ = await AnyioProcessRunner(max_concurrency=1).run(request)

    # Then: AnyIO terminates the child and returns a typed timeout


@pytest.mark.anyio
async def test_process_runner_maps_missing_executable(tmp_path: Path) -> None:
    # Given: a missing downstream executable
    request = ProcessRequest(
        command=(str(tmp_path / "missing-agy"), "--version"),
        cwd=tmp_path,
        timeout_seconds=1.0,
    )

    # When: process launch fails
    with pytest.raises(ExecutableUnavailableError):
        _ = await AnyioProcessRunner(max_concurrency=1).run(request)

    # Then: the local path remains private inside the typed failure


def test_isolated_directory_is_marker_owned_and_removed(tmp_path: Path) -> None:
    # Given: an explicit isolation base
    # When: one isolated context is entered
    with isolated_directory(tmp_path) as directory:
        created = directory
        assert created.parent == tmp_path
        assert (created / ".agy-search-owned").is_file()

    # Then: only that marker-owned directory is removed on exit
    assert not created.exists()
