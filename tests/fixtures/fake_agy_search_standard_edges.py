"""Deterministic standard-search provenance and retry scenarios."""

import os

from fake_agy_search_types import Emitter, JsonValue


def run_standard_edge_scenario(query: str, emit: Emitter) -> int | None:
    """Emit a focused standard-search edge case when the query names one."""
    match query:
        case "standard-date-korean":
            url = "https://example.com/korean-date"
            emit(
                {
                    "object": "search",
                    "evidence_audit": {
                        "candidates": [
                            {
                                "scope": "Korean publication date",
                                "claim": "Published on 2026-08-06",
                                "url": url,
                                "date": "2026-08-06",
                                "source_date_text": "2026년 8월 6일",
                                "evidence_excerpt": "게시일 2026년 8월 6일",
                            }
                        ],
                        "coverage_complete": True,
                        "conclusion": "Korean date",
                    },
                    "results": [
                        {
                            "title": "한국어 날짜 출처",
                            "url": url,
                            "snippet": "게시일 2026년 8월 6일",
                            "date": "2026-08-06",
                            "last_updated": None,
                        }
                    ],
                },
                "search_web",
                1,
                query,
            )
            return 0
        case (
            "standard-audit-retry"
            | "standard-audit-first-valid"
            | "standard-audit-missing-twice"
            | "standard-output-retry"
            | "grounding-two-results"
            | "grounding-all-dead-then-mixed"
            | "standard-unlisted-tool-retry"
            | "standard-unlisted-tool-invalid-audit"
            | "standard-non-source-first"
        ):
            invocation = _invocation_count()
            if query == "standard-output-retry" and invocation == 1:
                _emit_invalid_url_result(query, emit)
                return 0
            if query == "standard-non-source-first":
                _emit_non_source_retry_result(query, emit, invocation)
                return 0
            complete = query in {
                "standard-audit-first-valid",
                "standard-output-retry",
                "grounding-two-results",
                "grounding-all-dead-then-mixed",
                "standard-unlisted-tool-retry",
            } or (
                query == "standard-audit-retry" and invocation > 1
            )
            _emit_two_source_result(query, emit, complete, invocation)
            return 0
        case _:
            return None


def _invocation_count() -> int:
    trace_path = os.environ.get("AGY_SEARCH_FIXTURE_TRACE")
    if trace_path is None:
        return 1
    with open(trace_path, encoding="utf-8") as stream:
        return sum(1 for _line in stream)


def _emit_two_source_result(
    query: str, emit: Emitter, complete: bool, invocation: int
) -> None:
    urls = _two_source_urls(query, invocation)
    results: list[JsonValue] = [
        {
            "title": "Primary source",
            "url": urls[0],
            "snippet": "Primary evidence",
            "date": None,
            "last_updated": None,
        },
        {
            "title": "Secondary source",
            "url": urls[1],
            "snippet": "Secondary evidence",
            "date": None,
            "last_updated": None,
        },
    ]
    candidates: list[JsonValue] = [
        {
            "scope": "primary",
            "claim": "Primary evidence",
            "url": urls[0],
            "date": None,
        }
    ]
    if complete:
        candidates.append(
            {
                "scope": "secondary",
                "claim": "Secondary evidence",
                "url": urls[1],
                "date": None,
            }
        )
    unlisted_tool = (
        "list_dir"
        if query
        in {"standard-unlisted-tool-retry", "standard-unlisted-tool-invalid-audit"}
        and invocation == 1
        else None
    )
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": candidates,
                "coverage_complete": complete,
                "conclusion": query,
            },
            "results": results,
        },
        "search_web",
        1,
        query,
        additional_tool=unlisted_tool,
    )


def _two_source_urls(query: str, invocation: int) -> list[str]:
    match query:
        case "grounding-two-results":
            return [
                "https://vertexaisearch.cloud.google.com/grounding-api-redirect/primary",
                "https://vertexaisearch.cloud.google.com/grounding-api-redirect/secondary",
            ]
        case "grounding-all-dead-then-mixed":
            tokens = (
                ["dead-primary", "dead-secondary"]
                if invocation == 1
                else ["primary", "secondary"]
            )
            return [
                f"https://vertexaisearch.cloud.google.com/grounding-api-redirect/{token}"
                for token in tokens
            ]
        case _:
            return ["https://example.com/primary", "https://iana.org/secondary"]


def _emit_invalid_url_result(query: str, emit: Emitter) -> None:
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": [
                    {
                        "scope": "invalid first output",
                        "claim": "Missing source URL",
                        "url": "",
                        "date": None,
                    }
                ],
                "coverage_complete": False,
                "conclusion": query,
            },
            "results": [
                {
                    "title": "Invalid first output",
                    "url": "",
                    "snippet": "Missing source URL",
                    "date": None,
                    "last_updated": None,
                }
            ],
        },
        "search_web",
        1,
        query,
    )


def _emit_non_source_retry_result(query: str, emit: Emitter, invocation: int) -> None:
    first = invocation == 1
    url = (
        "https://www.google.com/search?q=korean+market"
        if first
        else "https://example.com/direct-market-source"
    )
    public_date = "2026-08-06" if first else None
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": [
                    {
                        "scope": "Korean market",
                        "claim": "Market evidence",
                        "url": url,
                        "date": None,
                        "source_date_text": "2026년 8월 6일" if first else None,
                        "evidence_excerpt": "2026년 8월 6일 시장" if first else None,
                    }
                ],
                "coverage_complete": True,
                "conclusion": query,
            },
            "results": [
                {
                    "title": "Market source",
                    "url": url,
                    "snippet": "Market evidence",
                    "date": public_date,
                    "last_updated": None,
                }
            ],
        },
        "search_web",
        1,
        query,
    )
