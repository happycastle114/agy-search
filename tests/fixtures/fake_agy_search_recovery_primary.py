"""Initial temporal recovery search scenarios."""

from fake_agy_search_restrictions import restricted_query
from fake_agy_search_types import Emitter, JsonValue


def emit_recovery_primary(
    query: str, payload: dict[str, JsonValue], emit: Emitter
) -> int:
    primary_url = "https://example.com/primary"
    candidates: list[JsonValue] = [
        {
            "scope": "alpha",
            "claim": "alpha-v1",
            "url": primary_url,
            "date": "2026-08-03",
            "value": "alpha-v1",
            "source_date_text": "2026-08-03",
            "evidence_excerpt": "released 2026-08-03",
        },
        {
            "scope": "beta",
            "claim": "beta-v1",
            "url": primary_url,
            "date": "2026-08-02",
            "value": "beta-v1",
            "source_date_text": "2026-08-02",
            "evidence_excerpt": "released 2026-08-02",
        },
    ]
    if query in {
        "temporal-local",
        "temporal-local-unextractable",
        "temporal-source-first-v25",
        "temporal-primary-after-cutoff-source-first",
        "temporal-source-after-cutoff-fallback",
        "temporal-fallback-after-cutoff",
        "temporal-source-first-tie",
    }:
        primary_url = (
            "https://example.com/local"
            if query == "temporal-local"
            else "https://example.com/local-unextractable"
            if query == "temporal-local-unextractable"
            else "https://example.com/local-v25"
            if query
            in {
                "temporal-source-first-v25",
                "temporal-primary-after-cutoff-source-first",
            }
            else "https://example.com/local-tie"
            if query == "temporal-source-first-tie"
            else "https://example.com/local-after-cutoff"
        )
        candidates = [
            {
                "scope": "alpha",
                "claim": "alpha-v2",
                "url": primary_url,
                "date": "2026-07-29",
                "value": "alpha-v2",
                "source_date_text": "July 29, 2026",
                "evidence_excerpt": "alpha-v2 July 29, 2026",
            },
            {
                "scope": "beta",
                "claim": "beta-v1",
                "url": primary_url,
                "date": "2026-07-28",
                "value": "beta-v1",
                "source_date_text": "July 28, 2026",
                "evidence_excerpt": "beta-v1 July 28, 2026",
            },
        ]
        if query in {
            "temporal-source-first-v25",
            "temporal-primary-after-cutoff-source-first",
        }:
            candidates[0]["claim"] = "v25.1.0"
            candidates[0]["value"] = "v25.1.0"
            candidates[0]["evidence_excerpt"] = "v25.1.0 July 29, 2026"
            candidates[1]["claim"] = "wrong-beta"
            candidates[1]["value"] = "wrong-beta"
            candidates[1]["evidence_excerpt"] = "wrong-beta July 28, 2026"
        if query == "temporal-primary-after-cutoff-source-first":
            candidates[0]["date"] = "2026-08-06"
            candidates[0]["source_date_text"] = "August 6, 2026"
            candidates[0]["evidence_excerpt"] = "v25.1.0 August 6, 2026"
    if query == "temporal-local-value-mismatch":
        primary_url = "https://example.com/local"
        candidates = [
            {
                "scope": "alpha",
                "claim": "alpha-v1",
                "url": primary_url,
                "date": "2026-07-29",
                "value": "alpha-v1",
                "source_date_text": "July 29, 2026",
                "evidence_excerpt": "alpha-v1 July 29, 2026",
            },
            {
                "scope": "beta",
                "claim": "beta-v0",
                "url": primary_url,
                "date": "2026-07-28",
                "value": "beta-v0",
                "source_date_text": "July 28, 2026",
                "evidence_excerpt": "beta-v0 July 28, 2026",
            },
        ]
    if query == "temporal-recoverable-borrowed":
        # Keep the primary winner valid while making the sibling tuple fail
        # only when the fetched source panels are checked.
        candidates[1] = {
            "scope": "beta",
            "claim": "alpha-v1",
            "url": primary_url,
            "date": "2026-08-03",
            "value": "alpha-v1",
            "source_date_text": "2026-08-03",
            "evidence_excerpt": "released 2026-08-03",
        }
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": candidates,
                "coverage_complete": True,
                "conclusion": "recovery required",
            },
            "results": [
                {
                    "title": "alpha-v2"
                    if query.startswith("temporal-local")
                    else "alpha-v1",
                    "url": primary_url,
                    "snippet": "alpha alpha-v2"
                    if query.startswith("temporal-local")
                    else "alpha alpha-v1",
                    "date": "2026-07-29"
                    if query.startswith("temporal-local")
                    else "2026-08-03",
                    "last_updated": None,
                }
            ],
        },
        "search_web",
        1,
        restricted_query(query, payload),
    )
    return 0
