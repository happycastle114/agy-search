"""Research fixtures for the deterministic Antigravity process fake."""

from collections.abc import Callable
from enum import Enum
from typing import assert_never

from fake_agy_research_temporal import (
    JsonValue,
    ResearchQuery,
    audit_candidate,
    findings,
    source,
    temporal_response,
)

Emitter = Callable[[dict[str, JsonValue], str, int], None]


class ScopePolicy(str, Enum):
    COMPLETE_REQUESTED_SCOPE = "complete_requested_scope"


class DatePolicy(str, Enum):
    EXPLICIT_SOURCE_ONLY = "explicit_source_only"


class ReadOnlyResearchQuery(str, Enum):
    EXACT_URL = "source-research-exact-read-only"
    DOMAIN_ONLY = "source-research-domain-read-only"


def run_research(
    payload: dict[str, JsonValue], arguments: list[str], emit: Emitter
) -> int:
    query = parse_query(payload)
    if query is ResearchQuery.TEMPORAL_SCHEMA:
        if not schema_is_temporally_strict(arguments):
            return 28
    if query is ResearchQuery.SINGLE_SCOPE and not schema_has_one_exact_scope(arguments):
        return 28
    if query is ResearchQuery.TEMPORAL_POLICY and not policies_are_explicit(payload):
        return 26
    response = temporal_response(query)
    if response is None:
        raw_query = str(payload["query"])
        response = (
            restricted_response(raw_query)
            or standard_date_response(raw_query)
            or ordinary_response(query)
        )
    tool_counts = {
        "research-five-tools": (2, 3),
        "research-seven-tools": (2, 5),
        "research-ten-tools": (2, 8),
        "research-eleven-tools": (2, 9),
    }
    raw_query = str(payload["query"])
    read_only_url = direct_read_url(raw_query)
    search_count, read_count = tool_counts.get(raw_query, (1, 0))
    if read_only_url is not None:
        search_count, read_count = (0, 1)
    emit(
        response,
        "search_web",
        search_count,
        query=restricted_query(payload),
        additional_tool="read_url_content" if read_count else None,
        additional_tool_count=read_count,
        scenario_override=raw_query,
        additional_tool_url=read_only_url,
    )
    return 0


def direct_read_url(raw_query: str) -> str | None:
    try:
        query = ReadOnlyResearchQuery(raw_query)
    except ValueError:
        return None
    match query:
        case ReadOnlyResearchQuery.EXACT_URL | ReadOnlyResearchQuery.DOMAIN_ONLY:
            return "https://doc.rust-lang.org/book/"
        case _:
            assert_never(query)


def restricted_query(payload: dict[str, JsonValue]) -> str | None:
    restriction = payload.get("source_restriction")
    if not isinstance(restriction, dict):
        return None
    query = str(payload["query"])
    domains = restriction.get("domains")
    if not isinstance(domains, list) or not domains:
        return query
    return " ".join([query, *(f"site:{domain}" for domain in domains)])


def restricted_response(raw_query: str) -> dict[str, JsonValue] | None:
    if not raw_query.startswith("source-research-"):
        return None
    allowed = (
        "https://v.daum.net/v/20260807120301584"
        if raw_query == "source-research-news-portal"
        else "https://doc.rust-lang.org/book/"
    )
    disallowed = "https://contributor.example/rust-release"
    sources = [source("Allowed", allowed, "2026-08-03")]
    citations = [allowed]
    candidates = [audit_candidate("allowed", "Allowed", allowed, "2026-08-03")]
    if raw_query == "source-research-public-mixed":
        sources.append(source("Contributor", disallowed, "2026-08-03"))
        citations.append(disallowed)
        candidates.append(audit_candidate("contributor", "Third party", disallowed, "2026-08-03"))
    elif raw_query == "source-research-citation-mixed":
        sources.append(source("Contributor", disallowed, "2026-08-03"))
        citations.append(disallowed)
        candidates.append(audit_candidate("contributor", "Third party", disallowed, "2026-08-03"))
    elif raw_query == "source-research-audit-mixed":
        candidates.append(audit_candidate("hidden", "Third party", disallowed, "2026-08-03"))
    return {
        "object": "research",
        "evidence_audit": {
            "candidates": candidates,
            "coverage_complete": True,
            "conclusion": "Restricted synthesis",
        },
        "title": "Research",
        "summary": "Synthesis",
        "findings": [{"title": "Finding", "summary": "Detail", "citations": citations}],
        "sources": sources,
    }


def parse_query(payload: dict[str, JsonValue]) -> ResearchQuery | None:
    try:
        return ResearchQuery(str(payload["query"]))
    except ValueError:
        return None


def schema_is_temporally_strict(arguments: list[str]) -> bool:
    import json

    schema = json.loads(arguments[arguments.index("--json-schema") + 1])
    candidate_required = set(schema["$defs"]["ScopeEvidence"].get("required", []))
    public_required = set(schema["$defs"]["WebSource"].get("required", []))
    return {"date", "evidence_excerpt", "source_date_text", "value"}.issubset(
        candidate_required
    ) and "date" in public_required and schema["properties"]["sources"].get("maxItems") == 20


def schema_has_one_exact_scope(arguments: list[str]) -> bool:
    import json

    schema = json.loads(arguments[arguments.index("--json-schema") + 1])
    candidates = schema["$defs"]["EvidenceAudit"]["properties"]["candidates"]
    scopes = schema["$defs"]["ScopeEvidence"]["properties"]["scope"]
    return (
        candidates.get("minItems") == 1
        and candidates.get("maxItems") == 1
        and scopes.get("enum") == ["Antigravity CLI"]
    )


def policies_are_explicit(payload: dict[str, JsonValue]) -> bool:
    return (
        payload.get("scope_policy") == ScopePolicy.COMPLETE_REQUESTED_SCOPE.value
        and payload.get("date_policy") == DatePolicy.EXPLICIT_SOURCE_ONLY.value
    )


def ordinary_response(query: ResearchQuery | None) -> dict[str, JsonValue]:
    source_url = "https://example.com/source"
    public_date = "2026-08-03" if query is ResearchQuery.EXPLICIT_DATE else None
    source: dict[str, JsonValue] = {
        "title": "Source",
        "url": source_url,
        "snippet": "Evidence",
    }
    if query is ResearchQuery.EXPLICIT_DATE:
        source["date"] = public_date
        source["last_updated"] = None
    return {
        "object": "research",
        "evidence_audit": {
            "candidates": [
                audit_candidate("primary fixture", "Evidence", source_url, public_date),
                audit_candidate("corroborating fixture", "Evidence", source_url, None),
            ],
            "coverage_complete": True,
            "conclusion": "Synthesis",
        },
        "title": "Research",
        "summary": "Synthesis",
        "findings": [] if query is ResearchQuery.EMPTY_FINDINGS else findings(source_url),
        "sources": [source],
    }


def standard_date_response(raw_query: str) -> dict[str, JsonValue] | None:
    scenarios = {
        "standard-date-month-only": ("2013-02-20", "February 2013"),
        "standard-date-iso": ("2013-02-20", "2013-02-20"),
        "standard-date-english": ("1999-06-01", "June 1, 1999"),
    }
    source_url = "https://example.com/source"
    if raw_query in scenarios:
        public_date, source_date_text = scenarios[raw_query]
        candidate = audit_candidate("primary fixture", "Evidence", source_url, public_date)
        candidate["source_date_text"] = source_date_text
        candidate["evidence_excerpt"] = f"Published {source_date_text}"
        public_source = source("Evidence", source_url, public_date)
    elif raw_query == "standard-malformed-update":
        candidate = audit_candidate("primary fixture", "Evidence", source_url, None)
        public_source = {
            "title": "Evidence",
            "url": source_url,
            "snippet": "Evidence",
            "date": None,
            "last_updated": "2026-13-40",
        }
    elif raw_query == "standard-malformed-date":
        candidate = audit_candidate("primary fixture", "Evidence", source_url, None)
        public_source = {
            "title": "Evidence",
            "url": source_url,
            "snippet": "Evidence",
            "date": "February 20, 2013",
            "last_updated": None,
        }
    elif raw_query == "standard-date-null":
        candidate = audit_candidate("primary fixture", "Evidence", source_url, None)
        public_source = {
            "title": "Evidence",
            "url": source_url,
            "snippet": "Evidence",
            "date": None,
            "last_updated": None,
        }
    else:
        return None
    return {
        "object": "research",
        "evidence_audit": {
            "candidates": [candidate],
            "coverage_complete": True,
            "conclusion": raw_query,
        },
        "title": "Research",
        "summary": "Synthesis",
        "findings": findings(source_url),
        "sources": [public_source],
    }
