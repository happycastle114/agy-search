from pathlib import Path
from typing import final

import pytest

from agy_search.contract import AgyCommandBuilder
from agy_search.discovery import AgyDiscovery
from agy_search.errors import OutputInvalidError
from agy_search.process import ProcessOutput, ProcessRequest


@final
class _DiscoveryRunner:
    def __init__(self, outputs: tuple[ProcessOutput, ...]) -> None:
        self._outputs = iter(outputs)
        self.requests: list[ProcessRequest] = []

    async def run(self, request: ProcessRequest) -> ProcessOutput:
        self.requests.append(request)
        return next(self._outputs)


@pytest.mark.anyio
async def test_discovery_reports_version_and_dynamic_models(tmp_path: Path) -> None:
    # Given: downstream version and model output from the official commands
    runner = _DiscoveryRunner(
        (
            ProcessOutput(stdout=b"1.1.10\n", stderr=b"", returncode=0),
            ProcessOutput(
                stdout=b"gemini-3.6-flash-low\nclaude-sonnet-4-6\n",
                stderr=b"",
                returncode=0,
            ),
        ),
    )
    discovery = AgyDiscovery(
        builder=AgyCommandBuilder("/fixture/agy"),
        runner=runner,
        cwd=tmp_path,
        timeout_seconds=3.0,
    )

    # When: status checks executable and authenticated model discovery
    status = await discovery.status()

    # Then: the public result is ready and both minimal argv calls were used
    assert status.available is True
    assert status.version == "1.1.10"
    assert status.model_count == 2
    assert tuple(request.command for request in runner.requests) == (
        ("/fixture/agy", "--version"),
        ("/fixture/agy", "models"),
    )


@pytest.mark.anyio
async def test_model_discovery_rejects_duplicate_or_invalid_slugs(tmp_path: Path) -> None:
    # Given: model discovery output that cannot be a unique argv allowlist
    runner = _DiscoveryRunner(
        (
            ProcessOutput(
                stdout=b"valid-model\nvalid-model\n",
                stderr=b"",
                returncode=0,
            ),
        ),
    )
    discovery = AgyDiscovery(
        builder=AgyCommandBuilder("agy"),
        runner=runner,
        cwd=tmp_path,
        timeout_seconds=3.0,
    )

    # When: the output crosses the discovery boundary
    with pytest.raises(OutputInvalidError):
        _ = await discovery.models()

    # Then: ambiguous model selection cannot escape
