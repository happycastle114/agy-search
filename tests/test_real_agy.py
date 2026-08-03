"""Opt-in compatibility gate against a signed-in real Antigravity CLI."""

import json
import os
import shutil
import sys

import pytest
from pydantic import TypeAdapter
from typer.testing import CliRunner

from agy_search.cli import app
from agy_search.enums import CliExitCode
from agy_search.models import (
    CrawlResponse,
    ExtractResponse,
    MapResponse,
    ModelsResponse,
    ResearchResponse,
    SearchResponse,
    StatusResponse,
)

type ContentResponse = (
    SearchResponse | ExtractResponse | MapResponse | CrawlResponse | ResearchResponse
)

_CONTENT_RESPONSE_ADAPTER: TypeAdapter[ContentResponse] = TypeAdapter(ContentResponse)
_RUNNER = CliRunner()


def _required_environment(name: str) -> str:
    value = os.getenv(name)
    if value is None or not value.strip():
        pytest.skip(f"{name} is required for the opt-in real Antigravity gate")
    return value


@pytest.mark.real_agy
def test_real_antigravity_stream_json_model_and_all_content_operations() -> None:
    """Prove discovery and every public operation against real stream-json events."""
    if os.getenv("AGY_SEARCH_RUN_REAL_E2E") != "1":
        pytest.skip("set AGY_SEARCH_RUN_REAL_E2E=1 to spend real Antigravity usage")

    configured_executable = os.getenv("AGY_SEARCH_AGY_PATH", "agy")
    executable = shutil.which(configured_executable)
    if executable is None:
        pytest.skip(f"Antigravity executable is unavailable: {configured_executable}")
    model = _required_environment("AGY_SEARCH_REAL_MODEL")
    common = ["--agy-path", executable, "--timeout", "180"]

    status_result = _RUNNER.invoke(app, [*common, "status", "--json"])
    assert status_result.exit_code == CliExitCode.SUCCESS, status_result.stderr
    status = StatusResponse.model_validate_json(status_result.stdout)
    assert status.available

    models_result = _RUNNER.invoke(app, [*common, "models", "--json"])
    assert models_result.exit_code == CliExitCode.SUCCESS, models_result.stderr
    models = ModelsResponse.model_validate_json(models_result.stdout)
    assert model in models.models

    content_common = [*common, "--model", model, "--effort", "low"]
    scenarios = (
        (
            "search",
            (["search", "IANA Example Domain official website", "-n", "2", "--json"],),
        ),
        ("extract", (["extract", "https://example.com/", "--json"],)),
        (
            "map",
            (
                [
                    "map",
                    "https://antigravity.google/",
                    "--limit",
                    "3",
                    "--instructions",
                    "Find official changelog and documentation pages",
                    "--json",
                ],
                [
                    "map",
                    "https://antigravity.google/",
                    "--limit",
                    "1",
                    "--instructions",
                    "Return the official changelog URL",
                    "--json",
                ],
            ),
        ),
        (
            "crawl",
            (["crawl", "https://example.com/", "--limit", "1", "--json"],),
        ),
        (
            "research",
            (
                [
                    "research",
                    "Explain the purpose of IANA Example Domain using primary sources",
                    "--max-sources",
                    "3",
                    "--json",
                ],
            ),
        ),
    )
    completed: list[str] = []
    for expected_object, attempts in scenarios:
        result = _RUNNER.invoke(app, [*content_common, *attempts[0]])
        if result.exit_code == CliExitCode.OUTPUT and len(attempts) > 1:
            result = _RUNNER.invoke(app, [*content_common, *attempts[1]])
        assert result.exit_code == CliExitCode.SUCCESS, f"{expected_object} failed: {result.stderr}"
        response = _CONTENT_RESPONSE_ADAPTER.validate_json(result.stdout)
        assert response.object == expected_object
        completed.append(expected_object)

    _ = sys.stdout.write(
        f"{
            json.dumps(
                {
                    'agy_version': status.version,
                    'model': model,
                    'object': 'real-e2e',
                    'operations': completed,
                },
                sort_keys=True,
                separators=(',', ':'),
            )
        }\n"
    )
