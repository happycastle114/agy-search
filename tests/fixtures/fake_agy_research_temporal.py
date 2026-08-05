"""Temporal research responses for the deterministic Antigravity process fake."""

from enum import Enum

JsonValue = None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]


class ResearchQuery(str, Enum):
    EMPTY_FINDINGS = "empty-findings"
    EXPLICIT_DATE = "explicit-date"
    TEMPORAL_POLICY = "temporal-policy"
    TEMPORAL_SCHEMA = "temporal-research-schema"
    MULTI_SOURCE = "temporal-research-multi-source"
    SHARED_PAGE = "temporal-research-shared-page"
    MISSING_OLDER_TEXT = "temporal-research-shared-page-missing-older"
    NON_ISO_SOURCE = "temporal-research-non-iso-source"
    ORPHAN_SOURCE_DATE = "temporal-research-orphan-source-date"
    NO_LATEST_SOURCE_DATE = "temporal-research-no-latest-source-date"
    UNBOUND_LAST_UPDATED = "temporal-research-unbound-last-updated"
    SINGLE_SCOPE = "temporal-research-single-scope"


def temporal_response(query: ResearchQuery | None) -> dict[str, JsonValue] | None:
    match query:
        case ResearchQuery.SINGLE_SCOPE:
            url = "https://example.com/antigravity-releases"
            return temporal_document(
                [
                    audit_candidate(
                        "Antigravity CLI", "Antigravity CLI 1.1.10", url, "2026-08-03"
                    )
                    | {
                        "value": "1.1.10",
                        "source_date_text": "August 3, 2026",
                        "evidence_excerpt": "1.1.10 August 3, 2026",
                    }
                ],
                [source("Antigravity CLI 1.1.10 August 3, 2026", url, "2026-08-03")],
            )
        case ResearchQuery.MULTI_SOURCE | ResearchQuery.TEMPORAL_SCHEMA:
            return temporal_document(
                [
                    temporal_candidate("alpha", "alpha-v1", "https://example.com/alpha", "2026-08-02"),
                    temporal_candidate("beta", "beta-v2", "https://example.com/beta", "2026-08-03"),
                ],
                [
                    source("Alpha alpha-v1 2026-08-02", "https://example.com/alpha", "2026-08-02"),
                    source("Beta beta-v2 2026-08-03", "https://example.com/beta", "2026-08-03"),
                ],
            )
        case ResearchQuery.UNBOUND_LAST_UPDATED:
            response = temporal_response(ResearchQuery.MULTI_SOURCE)
            if response is None:
                return None
            response["sources"][1]["last_updated"] = "2026-08-04"
            return response
        case (
            ResearchQuery.SHARED_PAGE
            | ResearchQuery.MISSING_OLDER_TEXT
            | ResearchQuery.NO_LATEST_SOURCE_DATE
        ):
            candidates = [
                temporal_candidate("alpha", "alpha-v1", "https://example.com/releases", "2026-08-02"),
                temporal_candidate("beta", "beta-v2", "https://example.com/releases", "2026-08-03"),
            ]
            text = "alpha-v1 2026-08-02 beta-v2 2026-08-03"
            if query is ResearchQuery.MISSING_OLDER_TEXT:
                text = "beta-v2 2026-08-03"
            source_date = "2026-08-03"
            if query is ResearchQuery.NO_LATEST_SOURCE_DATE:
                source_date = "2026-08-02"
            return temporal_document(candidates, [source(text, "https://example.com/releases", source_date)])
        case ResearchQuery.NON_ISO_SOURCE:
            return temporal_document(
                [
                    temporal_candidate("alpha", "alpha-v1", "https://example.com/alpha", "2026-08-02"),
                    temporal_candidate("beta", "beta-v2", "https://example.com/beta", "2026-08-03"),
                ],
                [
                    source("alpha-v1 2026-08-02", "https://example.com/alpha", "2026-08-02"),
                    source("beta-v2 2026-08-03", "https://example.com/beta", "August 3, 2026"),
                ],
            )
        case ResearchQuery.ORPHAN_SOURCE_DATE:
            return temporal_document(
                [
                    temporal_candidate("alpha", "alpha-v1", "https://example.com/alpha", "2026-08-02"),
                    temporal_candidate("beta", "beta-v2", "https://example.com/beta", "2026-08-03"),
                ],
                [
                    source("alpha-v1 2026-08-02", "https://example.com/alpha", "2026-08-01"),
                    source("beta-v2 2026-08-03", "https://example.com/beta", "2026-08-03"),
                ],
            )
        case _:
            return None


def temporal_document(
    candidates: list[dict[str, JsonValue]], sources: list[dict[str, JsonValue]]
) -> dict[str, JsonValue]:
    return {
        "object": "research",
        "evidence_audit": {
            "candidates": candidates,
            "coverage_complete": True,
            "conclusion": "Temporal synthesis",
        },
        "title": "Research",
        "summary": "Synthesis",
        "findings": findings(str(sources[0]["url"])),
        "sources": sources,
    }


def findings(source_url: str) -> list[JsonValue]:
    return [{"title": "Finding", "summary": "Detail", "citations": [source_url]}]


def source(text: str, url: str, date: str) -> dict[str, JsonValue]:
    return {"title": text, "url": url, "snippet": text, "date": date, "last_updated": None}


def temporal_candidate(
    scope: str, value: str, url: str, date: str
) -> dict[str, JsonValue]:
    return audit_candidate(scope, value, url, date) | {
        "value": value,
        "source_date_text": date,
        "evidence_excerpt": f"{value} released {date}",
    }


def audit_candidate(
    scope: str, claim: str, url: str, date: str | None
) -> dict[str, JsonValue]:
    candidate: dict[str, JsonValue] = {
        "scope": scope,
        "claim": claim,
        "url": url,
        "date": date,
    }
    if date is not None:
        candidate["source_date_text"] = date
        candidate["evidence_excerpt"] = f"Published {date}"
    return candidate
