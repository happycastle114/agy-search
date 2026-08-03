#!/usr/bin/env python3
"""Dependency-free fake of the public Antigravity process contract."""

import json
import sys
from urllib.parse import urljoin


def emit(structured_output: dict[str, object], tool: str) -> None:
    events = (
        {"event": "init"},
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
        print(json.dumps(event, separators=(",", ":")))


def request() -> tuple[str, dict[str, object]]:
    prompt = sys.argv[sys.argv.index("-p") + 1]
    operation = prompt.split(" operation", maxsplit=1)[0].rsplit(maxsplit=1)[-1]
    payload = json.loads(prompt.split("INPUT_JSON=", maxsplit=1)[1])
    return operation, payload


def run_operation() -> int:
    operation, payload = request()
    if operation == "search":
        query = str(payload["query"])
        url = "file:///private/source" if query == "invalid-source" else "https://example.com/source"
        source: dict[str, object] = {"title": "Source", "url": url, "snippet": query}
        if query == "extra-field":
            source["private"] = "must not escape"
        results = [source, source] if query == "duplicate-source" else [source]
        emit(
            {
                "object": "search",
                "results": results,
            },
            "call_mcp_tool" if query == "mcp-only" else "search_web",
        )
        return 0
    if operation == "extract":
        emit(
            {
                "object": "extract",
                "results": [
                    {"url": url, "title": "Page", "content": "Extracted content"}
                    for url in payload["urls"]
                ],
            },
            "read_url_content",
        )
        return 0
    if operation in {"map", "crawl"}:
        base_url = str(payload["url"])
        page_url = (
            "https://outside.example/docs"
            if payload.get("instructions") == "external"
            else urljoin(base_url, "/docs")
        )
        result = {"url": page_url, "title": "Docs"}
        result["depth" if operation == "map" else "content"] = (
            1 if operation == "map" else "Crawled content"
        )
        results = [] if payload.get("instructions") == "empty" else [result]
        emit(
            {"object": operation, "base_url": base_url, "results": results},
            "read_url_content",
        )
        return 0
    if operation == "research":
        source_url = "https://example.com/source"
        findings = [] if payload.get("query") == "empty-findings" else [
            {"title": "Finding", "summary": "Detail", "citations": [source_url]}
        ]
        emit(
            {
                "object": "research",
                "title": "Research",
                "summary": "Synthesis",
                "findings": findings,
                "sources": [
                    {"title": "Source", "url": source_url, "snippet": "Evidence"}
                ],
            },
            "search_web",
        )
        return 0
    return 23


def main() -> int:
    if sys.argv[1:] == ["--version"]:
        print("9.9.9-fixture")
        return 0
    if sys.argv[1:] == ["models"]:
        print("fixture-model")
        print("fixture-model-high")
        return 0
    return run_operation()


if __name__ == "__main__":
    raise SystemExit(main())
