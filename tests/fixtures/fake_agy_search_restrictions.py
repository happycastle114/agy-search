"""Caller-owned source restriction fixtures."""

from fake_agy_search_types import JsonValue


def restricted_query(query: str, payload: dict[str, JsonValue]) -> str | None:
    restriction = payload.get("source_restriction")
    if not isinstance(restriction, dict):
        return None
    domains = restriction.get("domains")
    if not isinstance(domains, list) or not domains:
        return query
    if query == "source-search-missing-site":
        return query
    if query == "source-search-mutated-site":
        return f"{query} site:rust-lang.org.evil.example"
    return " ".join([query, *(f"site:{domain}" for domain in domains)])
