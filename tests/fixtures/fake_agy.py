#!/usr/bin/env python3
"""Deterministic fake of the official agy process contract for CLI E2E tests."""

from __future__ import annotations

import json
import sys
from typing import TYPE_CHECKING
from urllib.parse import urljoin

from agy_search.models import (
    CrawlRequest,
    ExtractRequest,
    MapRequest,
    ResearchRequest,
    SearchRequest,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from agy_search.events import JsonObject


def _write_line(value: str) -> None:
    _ = sys.stdout.write(f"{value}\n")


def _emit(structured_output: JsonObject, tool: str) -> None:
    events = (
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
                "usage": {"total_tokens": 7},
            },
        },
    )
    for event in events:
        _write_line(json.dumps(event, separators=(",", ":")))


def _request() -> tuple[str, str]:
    prompt = sys.argv[sys.argv.index("-p") + 1]
    operation = prompt.split(" operation", maxsplit=1)[0].rsplit(maxsplit=1)[-1]
    payload = prompt.split("INPUT_JSON=", maxsplit=1)[1]
    return operation, payload


def _search(payload: str) -> int:
    request = SearchRequest.model_validate_json(payload)
    query = request.query
    url = "file:///private/source" if query == "invalid-source" else "https://example.com/source"
    _emit(
        {
            "object": "search",
            "results": [{"title": "Source", "url": url, "snippet": query}],
        },
        "search_web",
    )
    return 0


def _extract(payload: str) -> int:
    request = ExtractRequest.model_validate_json(payload)
    _emit(
        {
            "object": "extract",
            "results": [
                {"url": url, "title": "Page", "content": "Extracted content"}
                for url in request.urls
            ],
        },
        "read_url_content",
    )
    return 0


def _map(payload: str) -> int:
    request = MapRequest.model_validate_json(payload)
    base_url = request.url
    _emit(
        {
            "object": "map",
            "base_url": base_url,
            "results": [
                {"url": urljoin(base_url, "/docs"), "title": "Docs", "depth": 1},
            ],
        },
        "read_url_content",
    )
    return 0


def _crawl(payload: str) -> int:
    request = CrawlRequest.model_validate_json(payload)
    base_url = request.url
    _emit(
        {
            "object": "crawl",
            "base_url": base_url,
            "results": [
                {
                    "url": urljoin(base_url, "/docs"),
                    "title": "Docs",
                    "content": "Crawled content",
                },
            ],
        },
        "read_url_content",
    )
    return 0


def _research(payload: str) -> int:
    _ = ResearchRequest.model_validate_json(payload)
    source_url = "https://example.com/source"
    _emit(
        {
            "object": "research",
            "title": "Research",
            "summary": "Synthesis",
            "findings": [
                {"title": "Finding", "summary": "Detail", "citations": [source_url]},
            ],
            "sources": [
                {"title": "Source", "url": source_url, "snippet": "Evidence"},
            ],
        },
        "search_web",
    )
    return 0


def _run_operation() -> int:
    operation, payload = _request()
    handlers: dict[str, Callable[[str], int]] = {
        "search": _search,
        "extract": _extract,
        "map": _map,
        "crawl": _crawl,
        "research": _research,
    }
    handler = handlers.get(operation)
    return handler(payload) if handler is not None else 23


def main() -> int:
    if sys.argv[1:] == ["--version"]:
        _write_line("9.9.9-fixture")
        return 0
    if sys.argv[1:] == ["models"]:
        _write_line("fixture-model")
        _write_line("fixture-model-high")
        return 0
    return _run_operation()


if __name__ == "__main__":
    raise SystemExit(main())
