import json

import pytest

from agy_search.enums import AgyResearchTool
from agy_search.errors import OutputInvalidError
from agy_search.events import JsonObject, parse_structured_run
from agy_search.models import ExtractResponse, SearchResponse


def _event_stream(structured_output: JsonObject, tool: str = "search_web") -> str:
    events = (
        {"event": "init", "session_id": "private-fixture-session"},
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
                "usage": {"input_tokens": 2, "output_tokens": 3, "total_tokens": 5},
                "duration_seconds": 0.25,
                "num_turns": 2,
            },
        },
    )
    return "\n".join(json.dumps(event, separators=(",", ":")) for event in events)


def test_structured_run_requires_matching_research_tool_evidence() -> None:
    # Given: a valid search response and observed search tool
    output = _event_stream(
        {
            "object": "search",
            "results": [
                {
                    "title": "Source",
                    "url": "https://example.com/source",
                    "snippet": "Source-backed summary",
                },
            ],
        },
    )

    # When: the stream is parsed for search
    run = parse_structured_run(
        output,
        SearchResponse,
        required_tools=frozenset({AgyResearchTool.SEARCH_WEB}),
    )

    # Then: the typed response, tool evidence, and safe telemetry are returned
    assert run.response.object == "search"
    assert run.research_tools == (AgyResearchTool.SEARCH_WEB,)
    assert run.usage.total_tokens == 5


def test_extract_rejects_search_only_tool_evidence() -> None:
    # Given: a valid-looking extract response produced without a page-read tool
    output = _event_stream(
        {
            "object": "extract",
            "results": [
                {
                    "url": "https://example.com/page",
                    "title": "Page",
                    "content": "Extracted content",
                },
            ],
        },
    )

    # When: the stream is parsed for extraction
    with pytest.raises(OutputInvalidError):
        _ = parse_structured_run(
            output,
            ExtractResponse,
            required_tools=frozenset({AgyResearchTool.READ_URL_CONTENT}),
        )

    # Then: model recall or the wrong tool cannot satisfy extraction


@pytest.mark.parametrize(
    ("tool", "state"),
    [
        ("call_mcp_tool", "DONE"),
        ("search_web", "ACTIVE"),
    ],
)
def test_generic_or_incomplete_tool_steps_do_not_prove_live_web_research(
    tool: str,
    state: str,
) -> None:
    # Given: a valid-looking response with unrelated or incomplete tool activity
    document: JsonObject = {
        "object": "search",
        "results": [
            {
                "title": "Source",
                "url": "https://example.com/source",
                "snippet": "Unproven source",
            },
        ],
    }
    output = _event_stream(document, tool=tool).replace('"state":"DONE"', f'"state":"{state}"')

    # When: it crosses the live-web evidence boundary
    with pytest.raises(OutputInvalidError):
        _ = parse_structured_run(
            output,
            SearchResponse,
            required_tools=frozenset({AgyResearchTool.SEARCH_WEB}),
        )

    # Then: only a completed built-in web tool can satisfy provenance


@pytest.mark.parametrize(
    "output",
    [
        "{not-json",
        json.dumps({"event": "init"}),
        _event_stream({"object": "search", "results": []}),
        _event_stream(
            {
                "object": "search",
                "results": [
                    {
                        "title": "Local",
                        "url": "file:///private/source",
                        "snippet": "Not a web source",
                    },
                ],
            },
        ),
    ],
)
def test_malformed_or_sourceless_stream_fails_closed(output: str) -> None:
    # Given: malformed, incomplete, empty, or non-web structured output
    # When: it crosses the event boundary
    with pytest.raises(OutputInvalidError):
        _ = parse_structured_run(
            output,
            SearchResponse,
            required_tools=frozenset({AgyResearchTool.SEARCH_WEB}),
        )

    # Then: no partial response escapes
