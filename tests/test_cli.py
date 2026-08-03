import shutil
from pathlib import Path

import pytest
from pydantic import TypeAdapter
from typer.testing import CliRunner, Result

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

_RUNNER = CliRunner()
_FIXTURE_AGY = Path(__file__).parent / "fixtures" / "fake_agy.py"


@pytest.fixture
def fake_agy(tmp_path: Path) -> Path:
    executable = tmp_path / "agy"
    _ = shutil.copy(_FIXTURE_AGY, executable)
    executable.chmod(0o755)
    return executable


type ContentResponse = (
    SearchResponse | ExtractResponse | MapResponse | CrawlResponse | ResearchResponse
)

_CONTENT_RESPONSE_ADAPTER: TypeAdapter[ContentResponse] = TypeAdapter(ContentResponse)


def _invoke(
    fake_agy: Path,
    arguments: list[str],
    *,
    input_text: str | None = None,
) -> Result:
    return _RUNNER.invoke(
        app,
        ["--agy-path", str(fake_agy), *arguments],
        input=input_text,
    )


def test_help_exposes_seven_standalone_commands() -> None:
    # Given: the standalone Typer application
    # When: root help is rendered
    result = _RUNNER.invoke(app, ["--help"])

    # Then: discovery and five content operations are directly available
    assert result.exit_code == CliExitCode.SUCCESS
    for command in ("status", "models", "search", "extract", "map", "crawl", "research"):
        assert command in result.stdout


def test_version_is_available_without_downstream_agy() -> None:
    # Given: the installed standalone CLI
    # When: its eager version option runs without a command
    result = _RUNNER.invoke(app, ["--version"])

    # Then: package discovery does not require the downstream executable
    assert result.exit_code == CliExitCode.SUCCESS
    assert result.stdout == "0.1.0\n"


def test_status_and_models_use_live_discovery(fake_agy: Path) -> None:
    # Given: one executable with dynamic model output
    # When: discovery commands run through the public CLI
    status = _invoke(fake_agy, ["status"])
    models = _invoke(fake_agy, ["models"])

    # Then: both emit machine-readable JSON
    assert status.exit_code == CliExitCode.SUCCESS
    assert StatusResponse.model_validate_json(status.stdout).model_dump(mode="json") == {
        "available": True,
        "model_count": 2,
        "object": "status",
        "version": "9.9.9-fixture",
    }
    assert ModelsResponse.model_validate_json(models.stdout).models == (
        "fixture-model",
        "fixture-model-high",
    )


def test_search_reads_dash_query_and_writes_canonical_json(fake_agy: Path) -> None:
    # Given: a query supplied over stdin and one dynamically valid model
    # When: the search command executes
    result = _invoke(
        fake_agy,
        ["--model", "fixture-model", "search", "-", "--max-results", "1"],
        input_text="stdin query\n",
    )

    # Then: stdout contains only the validated response document
    assert result.exit_code == CliExitCode.SUCCESS
    document = SearchResponse.model_validate_json(result.stdout)
    assert document.object == "search"
    assert document.results[0].snippet == "stdin query"


@pytest.mark.parametrize(
    ("arguments", "expected_object"),
    [
        (["extract", "https://example.com/page"], "extract"),
        (["map", "https://example.com"], "map"),
        (["crawl", "https://example.com"], "crawl"),
        (["research", "research topic"], "research"),
    ],
)
def test_content_commands_execute_end_to_end(
    fake_agy: Path,
    arguments: list[str],
    expected_object: str,
) -> None:
    # Given: a deterministic downstream agy process
    # When: one public content command runs end to end
    result = _invoke(fake_agy, arguments)

    # Then: its distinct validated response reaches stdout
    assert result.exit_code == CliExitCode.SUCCESS
    assert _CONTENT_RESPONSE_ADAPTER.validate_json(result.stdout).object == expected_object


def test_output_file_keeps_stdout_empty_and_creates_context_directory(
    fake_agy: Path,
    tmp_path: Path,
) -> None:
    # Given: a nested context-saving output target
    output_path = tmp_path / ".agy-search" / "search.json"

    # When: search writes with the Tavily-style output option
    result = _invoke(fake_agy, ["search", "fixture", "-o", str(output_path)])

    # Then: JSON is on disk, stdout stays clean, and confirmation is on stderr
    assert result.exit_code == CliExitCode.SUCCESS
    assert result.stdout == ""
    assert SearchResponse.model_validate_json(output_path.read_text()).object == "search"
    assert str(output_path) in result.stderr


def test_invalid_source_and_unknown_model_use_stable_exit_codes(fake_agy: Path) -> None:
    # Given: an invalid web result and a model absent from dynamic discovery
    # When: each request crosses its boundary
    invalid_source = _invoke(fake_agy, ["search", "invalid-source"])
    unknown_model = _invoke(fake_agy, ["--model", "missing-model", "search", "fixture"])

    # Then: output and model failures remain distinct and stdout stays empty
    assert invalid_source.exit_code == CliExitCode.OUTPUT
    assert unknown_model.exit_code == CliExitCode.MODEL
    assert invalid_source.stdout == ""
    assert unknown_model.stdout == ""


def test_relative_executable_is_resolved_before_isolated_content_run() -> None:
    # Given: a valid agy executable path relative to the invocation directory
    relative_agy = Path("tests/fixtures/fake_agy.py")

    # When: a content operation changes into its isolated workspace
    result = _invoke(relative_agy, ["research", "fixture"])

    # Then: the executable remains launchable through its resolved path
    assert result.exit_code == CliExitCode.SUCCESS
    assert ResearchResponse.model_validate_json(result.stdout).object == "research"


def test_missing_executable_in_content_run_is_sanitized(tmp_path: Path) -> None:
    # Given: an absolute executable path that does not exist
    missing_agy = tmp_path / "missing-agy"

    # When: a content operation enters and exits its isolated workspace
    result = _invoke(missing_agy, ["search", "fixture"])

    # Then: context cleanup preserves the typed public failure without a traceback
    assert result.exit_code == CliExitCode.UNAVAILABLE
    assert result.stdout == ""
    assert result.stderr == "error: agy unavailable\n"
