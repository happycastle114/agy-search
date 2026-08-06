"""General deterministic search scenarios and dispatch."""

import json

from fake_agy_search_recovery_primary import emit_recovery_primary
from fake_agy_search_recovery_scope import emit_recovered_scope
from fake_agy_search_restrictions import restricted_query
from fake_agy_search_single_scope import emit_single_scope
from fake_agy_search_standard_edges import run_standard_edge_scenario
from fake_agy_search_types import (
    DatePolicy,
    Effort,
    Emitter,
    JsonValue,
    ScopePolicy,
    SingleScopeQuery,
    SourcePolicy,
    VerificationMode,
)


def run_search(
    payload: dict[str, JsonValue], arguments: list[str], emit: Emitter
) -> int:
    query = str(payload["query"])
    edge_result = run_standard_edge_scenario(query, emit)
    if edge_result is not None:
        return edge_result
    if query == "fixture" and "source_restriction" in payload:
        return 31
    scope = payload.get("scope")
    try:
        single_scope_query = SingleScopeQuery(query)
    except ValueError:
        single_scope_query = None
    if single_scope_query is not None:
        return emit_single_scope(payload, arguments, emit, single_scope_query)
    if query in {
        "temporal-recoverable",
        "temporal-recoverable-borrowed",
        "temporal-recoverable-one-invalid",
        "temporal-recoverable-bare-query",
        "temporal-recoverable-first-followup-query",
        "temporal-recoverable-value-followup",
        "temporal-recoverable-poisoned-followup",
        "temporal-recoverable-read-url",
        "temporal-recoverable-three-searches",
        "temporal-local",
        "temporal-local-unextractable",
        "temporal-local-value-mismatch",
        "temporal-source-first-v25",
        "temporal-primary-after-cutoff-source-first",
        "temporal-source-after-cutoff-fallback",
        "temporal-fallback-after-cutoff",
        "temporal-source-first-tie",
    }:
        if scope is not None:
            if query == "temporal-local":
                return 98
            return emit_recovered_scope(query, str(scope), payload, emit)
        return emit_recovery_primary(query, payload, emit)
    if query == "temporal-schema":
        schema = json.loads(arguments[arguments.index("--json-schema") + 1])
        candidate_required = set(schema["$defs"]["ScopeEvidence"].get("required", []))
        public_required = set(schema["$defs"]["WebSource"].get("required", []))
        required_candidate_fields = {
            "date",
            "evidence_excerpt",
            "source_date_text",
            "value",
        }
        if not required_candidate_fields.issubset(candidate_required) or (
            "date" not in public_required
        ):
            return 28
    if query == "temporal-complete" and (
        payload.get("verification") != VerificationMode.TEMPORAL_COMPARISON.value
    ):
        return 27
    if query == "temporal-policy" and (
        payload.get("scope_policy") != ScopePolicy.COMPLETE_REQUESTED_SCOPE.value
        or payload.get("date_policy") != DatePolicy.EXPLICIT_SOURCE_ONLY.value
    ):
        return 26
    if query == "default-fast-primary":
        effort_index = arguments.index("--effort") if "--effort" in arguments else None
        if effort_index is None or arguments[effort_index + 1] != Effort.LOW.value:
            return 25
        url = (
            "https://example.com/primary-source"
            if payload.get("source_policy") == SourcePolicy.PRIMARY_FIRST.value
            else "https://iana.org/aggregator-summary"
        )
    elif query == "invalid-source":
        url = "file:///private/source"
    elif query in {
        "grounding-redirect",
        "grounding-invalid-final",
        "grounding-multiple-lines",
    }:
        token = {
            "grounding-redirect": "token",
            "grounding-invalid-final": "invalid-final",
            "grounding-multiple-lines": "multiple-lines",
        }[query]
        url = f"https://vertexaisearch.cloud.google.com/grounding-api-redirect/{token}"
    elif query == "grounding-google-wrapper":
        url = "https://www.google.com/url?q=https%3A%2F%2Fexample.com%2Fcanonical"
    elif query == "grounding-trailing-dot":
        url = "https://vertexaisearch.cloud.google.com./grounding-api-redirect/token"
    elif query in {"source-domain-subdomain", "source-exact-url"}:
        url = "https://doc.rust-lang.org/book/"
    elif query == "source-domain-lookalike":
        url = "https://rust-lang.org.evil.example/blog"
    elif query == "source-contributor-blog":
        url = "https://contributor.example/rust-release"
    else:
        url = "https://example.com/source"
    source: dict[str, JsonValue] = {"title": "Source", "url": url, "snippet": query}
    public_date_scenarios = {
        "standard-date-month-only": ("2013-02-20", "February 2013"),
        "standard-date-iso": ("2013-02-20", "2013-02-20"),
        "standard-date-english": ("1999-06-01", "June 1, 1999"),
    }
    if query in public_date_scenarios:
        public_date, source_date_text = public_date_scenarios[query]
        source["date"] = public_date
        source["last_updated"] = None
    elif query == "standard-malformed-update":
        source["date"] = None
        source["last_updated"] = "2026-13-40"
    elif query == "standard-malformed-date":
        source["date"] = "February 20, 2013"
        source["last_updated"] = None
    elif query == "standard-date-null":
        source["date"] = None
        source["last_updated"] = None
    dated_queries = {
        "explicit-date",
        "explicit-update",
        "temporal-complete",
        "temporal-invalid-date",
        "temporal-eight-tools",
        "temporal-schema",
        "temporal-source-text-missing",
        "temporal-unbound",
        "temporal-unbound-last-updated",
        "temporal-wrong-winner",
    }
    if query in dated_queries:
        source["date"] = "2026-08-03"
        source["last_updated"] = (
            "2026-08-04"
            if query in {"explicit-update", "temporal-unbound-last-updated"}
            else None
        )
    if query == "extra-field":
        source["private"] = "must not escape"
    results = [source, source] if query == "duplicate-source" else [source]
    candidates: list[JsonValue] = [
        {
            "scope": "primary fixture",
            "claim": query,
            "url": url,
            "date": source.get("date"),
        },
        {
            "scope": "corroborating fixture",
            "claim": query,
            "url": url,
            "date": source.get("date"),
        },
    ]
    if query in public_date_scenarios:
        candidates[0]["source_date_text"] = source_date_text
        candidates[0]["evidence_excerpt"] = f"Published {source_date_text}"
    elif query in {"explicit-date", "explicit-update"}:
        candidates[0]["source_date_text"] = "2026-08-03"
        candidates[0]["evidence_excerpt"] = "Published 2026-08-03"
    if query == "temporal-incomplete":
        candidates = candidates[:1]
    temporal_queries = {
        "temporal-complete",
        "temporal-invalid-date",
        "temporal-eight-tools",
        "temporal-schema",
        "temporal-source-text-missing",
        "temporal-unbound-last-updated",
        "temporal-wrong-winner",
    }
    if query in temporal_queries:
        newest_date = (
            "August 3, 2026" if query == "temporal-invalid-date" else "2026-08-03"
        )
        older_date = (
            "August 2, 2026" if query == "temporal-invalid-date" else "2026-08-02"
        )
        candidates = [
            {
                "scope": "newer fixture",
                "claim": "Newest v2",
                "url": url,
                "date": newest_date,
                "value": "v2",
                "evidence_excerpt": "v2 released August 3, 2026",
            },
            {
                "scope": "older fixture",
                "claim": "Older v1",
                "url": url,
                "date": older_date,
                "value": "v1",
                "evidence_excerpt": "v1 released August 2, 2026",
            },
        ]
        if query != "temporal-source-text-missing":
            candidates[0]["source_date_text"] = "August 3, 2026"
            candidates[1]["source_date_text"] = "August 2, 2026"
        winner_is_older = query == "temporal-wrong-winner"
        source["title"] = "Older v1" if winner_is_older else "Newest v2"
        source["snippet"] = "v1" if winner_is_older else "v2"
        source["date"] = older_date if winner_is_older else newest_date
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": candidates,
                "coverage_complete": query != "temporal-incomplete",
                "conclusion": query,
            },
            "results": results,
        },
        "call_mcp_tool" if query == "mcp-only" else "search_web",
        8 if query == "temporal-eight-tools" else 3 if query == "too-many-tools" else 1,
        restricted_query(query, payload),
    )
    return 0
