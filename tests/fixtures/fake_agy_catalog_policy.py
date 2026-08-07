#!/usr/bin/env python3
"""Process fixture recording model-catalog and content invocations."""

import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Final


PREFERRED_MODEL: Final = "gemini-3.6-flash-low"
FIRST_RETRY_MODEL: Final = "gemini-3.6-flash-medium"
FINAL_RETRY_MODEL: Final = "gemini-3.6-flash-high"
TRACE_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_TRACE"
CATALOG_MODE_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_MODE"
CATALOG_DELAY_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_DELAY"
CONTENT_DELAY_ENVIRONMENT: Final = "AGY_SEARCH_CONTENT_DELAY"
CONTENT_MODE_ENVIRONMENT: Final = "AGY_SEARCH_CATALOG_CONTENT_MODE"
GROUNDING_ORIGIN: Final = (
    "https://vertexaisearch.cloud.google.com/grounding-api-redirect/"
)


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


def emit_search_response(transports: tuple[str, str]) -> None:
    """Emit a schema-valid diversified search response and one evidence tool."""
    conversation_id = "catalog-policy"
    source_url, second_url = transports
    structured_output = {
        "object": "search",
        "evidence_audit": {
            "candidates": [
                {
                    "scope": "catalog policy",
                    "claim": "catalog policy fixture",
                    "url": source_url,
                    "date": None,
                },
                {
                    "scope": "catalog policy alternate",
                    "claim": "catalog policy fixture alternate",
                    "url": second_url,
                    "date": None,
                },
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
            },
            {
                "title": "Catalog policy alternate",
                "url": second_url,
                "snippet": "catalog policy fixture alternate",
                "date": None,
                "last_updated": None,
            },
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


def schema_accepts(transports: tuple[str, str]) -> bool:
    """Validate the generated unrestricted Search boundary used by this fixture."""
    raw_schema = option_value("--json-schema")
    if raw_schema is None:
        return False
    try:
        schema = json.loads(raw_schema)
        pattern = schema["$defs"]["HttpUrl"]["pattern"]
        candidate_minimum = schema["$defs"]["EvidenceAudit"]["properties"][
            "candidates"
        ]["minItems"]
        result_bounds = schema["properties"]["results"]
    except (json.JSONDecodeError, KeyError, TypeError):
        return False
    return (
        candidate_minimum == 2
        and result_bounds.get("minItems") == 2
        and result_bounds.get("maxItems") == 5
        and all(re.fullmatch(pattern, transport) for transport in transports)
    )


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
        catalog = {
            "preferred": [PREFERRED_MODEL, FIRST_RETRY_MODEL, FINAL_RETRY_MODEL],
            "without-medium": [PREFERRED_MODEL, FINAL_RETRY_MODEL],
            "without-high": [PREFERRED_MODEL, FIRST_RETRY_MODEL],
            "low-only": [PREFERRED_MODEL],
        }
        for model in catalog.get(mode, []):
            print(model)
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
    token_kind = "catalog-policy"
    content_mode = os.environ.get(CONTENT_MODE_ENVIRONMENT)
    if content_mode in {"retry", "retry-twice"}:
        with Path(os.environ[TRACE_ENVIRONMENT]).open(encoding="utf-8") as stream:
            content_count = sum(
                json.loads(line).get("kind") == "content" for line in stream
            )
        failed_attempts = 2 if content_mode == "retry-twice" else 1
        if content_count <= failed_attempts:
            token_kind = "catalog-policy-dead"
    transports = (
        f"{GROUNDING_ORIGIN}{token_kind}-1",
        f"{GROUNDING_ORIGIN}{token_kind}-2",
    )
    if not schema_accepts(transports):
        return 64
    emit_search_response(transports)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
