"""Single-scope temporal search scenarios."""

import json

from fake_agy_search_types import Emitter, JsonValue, SingleScopeQuery


def emit_single_scope(
    payload: dict[str, JsonValue],
    arguments: list[str],
    emit: Emitter,
    query: SingleScopeQuery,
) -> int:
    if payload.get("scope") is None:
        schema = json.loads(arguments[arguments.index("--json-schema") + 1])
        candidates = schema["$defs"]["EvidenceAudit"]["properties"]["candidates"]
        scopes = schema["$defs"]["ScopeEvidence"]["properties"]["scope"]
        if (
            candidates.get("minItems") != 1
            or candidates.get("maxItems") != 1
            or scopes.get("enum") != ["Antigravity CLI"]
        ):
            return 28
    suffix = {
        SingleScopeQuery.COMPLETE: "antigravity-releases",
        SingleScopeQuery.AMBIGUOUS: "antigravity-ambiguous",
        SingleScopeQuery.MISSING: "antigravity-missing",
        SingleScopeQuery.AFTER_CUTOFF: "antigravity-after-cutoff",
    }[query]
    source_url = f"https://example.com/{suffix}"
    after_cutoff = query is SingleScopeQuery.AFTER_CUTOFF
    value = "1.1.11" if after_cutoff else "2.5.0"
    date = "2026-08-04" if after_cutoff else "2026-08-03"
    source_date_text = "August 4, 2026" if after_cutoff else "August 3, 2026"
    emit(
        {
            "object": "search",
            "evidence_audit": {
                "candidates": [
                    {
                        "scope": "Antigravity CLI",
                        "claim": f"Antigravity CLI {value}",
                        "url": source_url,
                        "date": date,
                        "value": value,
                        "source_date_text": source_date_text,
                        "evidence_excerpt": f"Antigravity CLI {value} {source_date_text}",
                    }
                ],
                "coverage_complete": True,
                "conclusion": f"Antigravity CLI {value}",
            },
            "results": [
                {
                    "title": f"Antigravity CLI {value}",
                    "url": source_url,
                    "snippet": f"Antigravity CLI {value}",
                    "date": date,
                    "last_updated": None,
                }
            ],
        },
        "search_web",
        1,
        payload.get("required_search_query"),
    )
    return 0
