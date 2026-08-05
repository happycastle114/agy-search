"""Per-scope temporal recovery scenarios."""

from fake_agy_search_types import Emitter, JsonValue


def emit_recovered_scope(
    query: str, scope: str, payload: dict[str, JsonValue], emit: Emitter
) -> int:
    is_alpha = scope == "alpha"
    source_url = f"https://example.com/{scope}"
    value = "alpha-v2" if is_alpha else "beta-v1"
    date = "2026-08-05" if is_alpha else "2026-08-04"
    if query == "temporal-fallback-after-cutoff" and is_alpha:
        date = "2026-08-06"
    if query == "temporal-recoverable-borrowed" and not is_alpha:
        # Deliberately borrow the alpha tuple while claiming the beta panel.
        # The public source verifier must reject this cross-section tuple.
        source_url = "https://example.com/primary"
        value = "alpha-v1"
        date = "2026-08-03"
    excerpt = f"{value} released {date}"
    if query == "temporal-recoverable-one-invalid" and not is_alpha:
        excerpt = f"released {date}"
    required_search_query = payload_required_search_query(query, scope, payload)
    if payload.get("required_search_query") != required_search_query:
        return 29
    scoped_query = (
        scope if query == "temporal-recoverable-bare-query" else required_search_query
    )
    if query == "temporal-recoverable-first-followup-query":
        scoped_query = f"{required_search_query} release date"
    followup_query = None
    if query == "temporal-recoverable-value-followup":
        scoped_query = required_search_query
        followup_query = f"{required_search_query} {value}"
    if query == "temporal-recoverable-poisoned-followup":
        scoped_query = required_search_query
        followup_query = (
            f"{required_search_query} {value} July 29, 2026 "
            "https://example.com/release"
        )
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": [
                    {
                        "scope": scope,
                        "claim": value,
                        "url": source_url,
                        "date": date,
                        "value": value,
                        "source_date_text": date,
                        "evidence_excerpt": excerpt,
                    }
                ],
                "coverage_complete": True,
                "conclusion": value,
            },
            "results": [
                {
                    "title": value,
                    "url": source_url,
                    "snippet": f"{scope} {value}",
                    "date": date,
                    "last_updated": None,
                }
            ],
        },
        "search_web",
        3
        if query == "temporal-recoverable-three-searches"
        else 2
        if followup_query is not None
        else 1,
        query=scoped_query,
        additional_tool=(
            "read_url_content" if query == "temporal-recoverable-read-url" else None
        ),
        followup_query=followup_query,
    )
    return 0


def payload_required_search_query(
    query: str, scope: str, payload: dict[str, JsonValue]
) -> str:
    tokens = [
        f'For exact scope "{scope}" only, find its latest release, exact version, and source-published date; do not use another scope\'s value. Original request constraints: {query}'
    ]
    restriction = payload.get("source_restriction")
    domains = restriction.get("domains", []) if isinstance(restriction, dict) else []
    tokens.extend(f"site:{domain}" for domain in domains)
    country = payload.get("country")
    if country is not None:
        tokens.append(f"country:{country}")
    return " ".join(tokens)
