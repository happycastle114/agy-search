#!/usr/bin/env python3
"""Process fixture recording model-catalog and content invocations."""

import json
import os
import sys
import time
from pathlib import Path
from typing import Final


PREFERRED_MODEL: Final = "gemini-3.6-flash-low"
TRACE_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_TRACE"
CATALOG_MODE_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_MODE"
CATALOG_DELAY_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_DELAY"
CONTENT_DELAY_ENVIRONMENT: Final = "AGY_SEARCH_CONTENT_DELAY"


def trace(record: dict[str, str | None]) -> None:
    """Append one process-visible invocation record when tracing is enabled."""
    path = os.environ.get(TRACE_ENVIRONMENT)
    if path is None:
        return
    with Path(path).open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(record, separators=(",", ":")) + "\n")


def option_value(option: str) -> str | None:
    """Return a CLI option value when the fixture received that option."""
    try:
        index = sys.argv.index(option)
    except ValueError:
        return None
    return sys.argv[index + 1]


def emit_search_response() -> None:
    """Emit the minimum valid stream-json search response and one evidence tool."""
    conversation_id = "catalog-policy"
    source_url = "https://example.com/catalog-policy"
    structured_output = {
        "object": "search",
        "evidence_audit": {
            "candidates": [
                {
                    "scope": "catalog policy",
                    "claim": "catalog policy fixture",
                    "url": source_url,
                    "date": None,
                }
            ],
            "coverage_complete": True,
            "conclusion": "catalog policy fixture",
        },
        "results": [
            {
                "title": "Catalog policy",
                "url": source_url,
                "snippet": "catalog policy fixture",
                "date": None,
                "last_updated": None,
            }
        ],
    }
    events = [
        {"event": "init", "conversation_id": conversation_id},
        {
            "event": "step_update",
            "step_update": {
                "conversation_id": conversation_id,
                "state": "DONE",
                "step_type": "tool",
                "tool_info": {"name": "search_web"},
            },
        },
        {
            "event": "result",
            "conversation_id": conversation_id,
            "result": {
                "structured_output": structured_output,
                "usage": {"total_tokens": 1},
            },
        },
    ]
    for event in events:
        print(json.dumps(event, separators=(",", ":")))


def main() -> int:
    """Dispatch the exact Antigravity surface exercised by catalog-policy tests."""
    arguments = sys.argv[1:]
    if arguments == ["--version"]:
        trace({"kind": "version", "model": None, "effort": None})
        print("1.1.10")
        return 0
    if arguments == ["models"]:
        trace({"kind": "models", "model": None, "effort": None})
        configured_delay = os.environ.get(CATALOG_DELAY_ENVIRONMENT)
        if configured_delay is not None:
            time.sleep(float(configured_delay))
        mode = os.environ.get(CATALOG_MODE_ENVIRONMENT, "preferred")
        if mode == "failed":
            return 1
        if mode == "preferred":
            print(PREFERRED_MODEL)
        print("fixture-model")
        return 0
    trace(
        {
            "kind": "content",
            "model": option_value("--model"),
            "effort": option_value("--effort"),
        }
    )
    configured_delay = os.environ.get(CONTENT_DELAY_ENVIRONMENT)
    if configured_delay is not None:
        time.sleep(float(configured_delay))
    emit_search_response()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
