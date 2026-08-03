import json
from pathlib import Path
from typing import final

import pytest

from agy_search.backend import AgyBackend, AgyBackendConfig
from agy_search.contract import AgyCommandBuilder, ModelSlug
from agy_search.enums import AgyEffort
from agy_search.errors import OutputInvalidError
from agy_search.events import JsonObject
from agy_search.models import MapRequest, SearchRequest
from agy_search.process import ProcessOutput, ProcessRequest


def _stream(structured_output: JsonObject, tool: str) -> bytes:
    events: tuple[JsonObject, ...] = (
        {
            "event": "step_update",
            "step_update": {
                "state": "DONE",
                "step_type": "tool",
                "tool_info": {"name": tool},
            },
        },
        {
            "event": "result",
            "result": {
                "structured_output": structured_output,
                "usage": {"total_tokens": 8},
            },
        },
    )
    return "\n".join(json.dumps(event) for event in events).encode()


@final
class _BackendRunner:
    def __init__(self, stdout: bytes) -> None:
        self._stdout = stdout
        self.requests: list[ProcessRequest] = []

    async def run(self, request: ProcessRequest) -> ProcessOutput:
        self.requests.append(request)
        return ProcessOutput(stdout=self._stdout, stderr=b"", returncode=0)


@pytest.mark.anyio
async def test_search_builds_schema_constrained_isolated_run(tmp_path: Path) -> None:
    # Given: a source-backed terminal search response
    runner = _BackendRunner(
        _stream(
            {
                "object": "search",
                "results": [
                    {
                        "title": "Source",
                        "url": "https://example.com/source",
                        "snippet": "Summary",
                    },
                ],
            },
            tool="search_web",
        ),
    )
    backend = AgyBackend(
        builder=AgyCommandBuilder("/fixture/agy"),
        runner=runner,
        config=AgyBackendConfig(
            isolation_base=tmp_path,
            timeout_seconds=5.0,
            print_timeout="5s",
            model=ModelSlug("fixture-model"),
            effort=AgyEffort.HIGH,
        ),
    )

    # When: the search operation executes
    response = await backend.search(SearchRequest(query="fixture", max_results=1))

    # Then: response and argv contract are typed, and isolation is removed
    assert response.results[0].url == "https://example.com/source"
    request = runner.requests[0]
    assert request.cwd.parent == tmp_path
    assert not request.cwd.exists()
    assert "--json-schema" in request.command
    assert request.command[request.command.index("--model") + 1] == "fixture-model"
    assert request.command[request.command.index("--effort") + 1] == "high"
    prompt = request.command[request.command.index("-p") + 1]
    assert "built-in search_web" in prompt
    assert "Do not use call_mcp_tool" in prompt


@pytest.mark.anyio
async def test_map_rejects_external_results_unless_explicitly_allowed(tmp_path: Path) -> None:
    # Given: a map response containing a different-origin URL
    runner = _BackendRunner(
        _stream(
            {
                "object": "map",
                "base_url": "https://example.com/",
                "results": [
                    {
                        "url": "https://outside.example/docs",
                        "title": "External",
                        "depth": 1,
                    },
                ],
            },
            tool="read_url_content",
        ),
    )
    backend = AgyBackend(
        builder=AgyCommandBuilder("agy"),
        runner=runner,
        config=AgyBackendConfig(isolation_base=tmp_path),
    )

    # When: the default same-origin boundary validates the result
    with pytest.raises(OutputInvalidError):
        _ = await backend.map(MapRequest(url="https://example.com"))

    # Then: cross-origin discovery is not silently returned
