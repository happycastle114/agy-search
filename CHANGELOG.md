# Changelog

## [0.4.0] - 2026-09-20

### Added

- Optional `agy-search-server` Rust binary behind the `server` feature.
- Authenticated LiteLLM-compatible `/search`, protected readiness, and
  Streamable HTTP MCP endpoints, plus local stdio MCP.
- Five schema-backed MCP tools: `agy_search`, `agy_extract`, `agy_map`,
  `agy_crawl`, and `agy_research`.
- Bounded request concurrency, cancellation-aware transports, request body
  limits, Host/Origin policy, and JSON stderr diagnostics.
- Advisory model-catalog reuse for successful unpinned discovery, with a
  bounded TTL and no cache for explicit pins, status, or failures.

### Changed

- Generated AGY agents explicitly disable inherited customizations and MCP so
  ambient skills, rules, subagents, and MCP configuration cannot alter a
  request.
- The AGY integration documents headless timeout changes through AGY 1.2.7 and
  distinguishes direct Gemini API names from AGY's dynamic effort-qualified
  catalog slugs.

### Compatibility

- AGY 1.1.20 remains the minimum supported version. The latest upstream release
  reviewed for this change is 1.2.7.
- The standalone CLI retains its existing JSON contract. The server provides
  standard verification only; temporal comparison remains a CLI operation.
