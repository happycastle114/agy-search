#!/usr/bin/env python3
"""Dependency-free fake of the public Antigravity process contract."""

import json
import os
import sys
from urllib.parse import urljoin

from fake_agy_agent import valid_agent_invocation
from fake_agy_research import run_research
from fake_agy_search import run_search


def emit(
    structured_output: dict[str, object],
    tool: str,
    tool_count: int = 1,
    query: str | None = None,
    *,
    additional_tool: str | None = None,
    additional_tool_url: str | list[str] | None = None,
    followup_query: str | None = None,
    additional_tool_count: int = 1,
    scenario_override: str | None = None,
    failed_tool_before: str | None = None,
    tool_url: str | None = None,
) -> None:
    evidence_audit = structured_output.get("evidence_audit")
    scenario = scenario_override or (
        evidence_audit.get("conclusion") if isinstance(evidence_audit, dict) else query
    )
    current_conversation = "current-conversation"
    tool_conversation = (
        "foreign-conversation"
        if scenario == "foreign-conversation-tools"
        else current_conversation
    )
    events = [{"event": "init", "conversation_id": current_conversation}]
    if failed_tool_before is not None:
        failed_info: dict[str, object] = {"name": failed_tool_before}
        for state in ("ACTIVE", "ERROR"):
            events.append(
                {
                    "event": "step_update",
                    "step_update": {
                        "conversation_id": tool_conversation,
                        "state": state,
                        "step_type": "tool",
                        "tool_info": (
                            failed_info
                            if state == "ACTIVE"
                            else failed_info | {"error": "invalid arguments"}
                        ),
                    },
                }
            )
    for tool_index in range(tool_count):
        active_query = followup_query if tool_index > 0 and followup_query else query
        tool_info: dict[str, object] = {"name": tool}
        if tool_url is not None:
            tool_info["parameters"] = {"Url": tool_url}
        elif active_query is not None:
            tool_info["parameters"] = {"query": active_query}
        if active_query is not None or tool_url is not None:
            events.append(
                {
                    "event": "step_update",
                    "step_update": {
                        "conversation_id": tool_conversation,
                        "state": "ACTIVE",
                        "step_type": "tool",
                        "tool_info": tool_info,
                    },
                }
            )
        events.append(
            {
                "event": "step_update",
                "step_update": {
                    "conversation_id": tool_conversation,
                    "state": "DONE",
                    "step_type": "tool",
                    "tool_info": tool_info,
                },
            }
        )
    if scenario in {"failed-extra-tools", "unfinished-extra-tools"}:
        for tool_index in range(4):
            tool_info = {
                "name": "read_url_content",
                "parameters": {"Url": f"https://example.com/extra-{tool_index}"},
            }
            events.append(
                {
                    "event": "step_update",
                    "step_update": {
                        "conversation_id": tool_conversation,
                        "state": "ACTIVE",
                        "step_type": "tool",
                        "tool_info": tool_info,
                    },
                }
            )
            if scenario == "failed-extra-tools":
                events.append(
                    {
                        "event": "step_update",
                        "step_update": {
                            "conversation_id": tool_conversation,
                            "state": "DONE",
                            "step_type": "tool",
                            "tool_info": tool_info | {"error": "network"},
                        },
                    }
                )
    if additional_tool is not None:
        for additional_index in range(additional_tool_count):
            for state in ("ACTIVE", "DONE"):
                additional_tool_info: dict[str, object] = {"name": additional_tool}
                if additional_tool_url is not None:
                    selected_url = (
                        additional_tool_url[additional_index]
                        if isinstance(additional_tool_url, list)
                        else additional_tool_url
                    )
                    additional_tool_info["parameters"] = {"Url": selected_url}
                events.append(
                    {
                        "event": "step_update",
                        "step_update": {
                            "conversation_id": tool_conversation,
                            "state": state,
                            "step_type": "tool",
                            "tool_info": additional_tool_info,
                        },
                    }
                )
    events.append(
        {
            "event": "result",
            "conversation_id": current_conversation,
            "result": {
                "structured_output": structured_output,
                "usage": {"total_tokens": 7},
            },
        }
    )
    for event in events:
        print(json.dumps(event, separators=(",", ":")))


def request() -> tuple[str, dict[str, object]]:
    prompt = sys.argv[sys.argv.index("-p") + 1]
    operation = prompt.split(" operation", maxsplit=1)[0].rsplit(maxsplit=1)[-1]
    payload = json.loads(prompt.split("INPUT_JSON=", maxsplit=1)[1])
    return operation, payload


def trace_content_invocation(payload: dict[str, object]) -> None:
    trace_path = os.environ.get("AGY_SEARCH_FIXTURE_TRACE")
    if trace_path is None:
        return
    record = json.dumps(
        {
            "scope": payload.get("scope"),
            "query": payload.get("query"),
            "required_search_query": payload.get("required_search_query"),
            "cutoff": (
                payload.get("temporal_contract", {}).get("cutoff")
                if isinstance(payload.get("temporal_contract"), dict)
                else None
            ),
        },
        separators=(",", ":"),
    ).encode()
    descriptor = os.open(
        trace_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600
    )
    try:
        os.write(descriptor, record + b"\n")
    finally:
        os.close(descriptor)


def run_operation() -> int:
    if not valid_agent_invocation(sys.argv[1:]):
        return 24
    if sys.argv.count("--disable-slash-commands") != 1:
        return 24
    if sys.argv.index("--disable-slash-commands") > sys.argv.index("-p"):
        return 24
    operation, payload = request()
    trace_content_invocation(payload)
    if operation == "search":
        return run_search(payload, sys.argv, emit)
    if operation == "extract":
        emit(
            {
                "object": "extract",
                "results": [
                    {"url": url, "title": "Page", "content": "Extracted content"}
                    for url in payload["urls"]
                ],
            },
            "search_web",
            0,
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
            "search_web",
            2 if payload.get("instructions") == "two-tools" else 1,
        )
        return 0
    if operation == "research":
        return run_research(payload, sys.argv, emit)
    return 23


def main() -> int:
    if sys.argv[1:] == ["--version"]:
        print("9.9.9")
        return 0
    if sys.argv[1:] == ["models"]:
        print("fixture-model\tFixture Model")
        print("fixture-model-high\tFixture Model (High)")
        return 0
    return run_operation()


if __name__ == "__main__":
    raise SystemExit(main())
