# Compatibility

## Supported boundaries

| Component | Supported contract |
|---|---|
| `agy-search` CLI | AGY 1.1.20 or newer; source-backed JSON commands |
| `agy-search-server` | Optional `server` Cargo feature; HTTP and MCP transports |
| HTTP search | LiteLLM native `perplexity` Search provider shape |
| MCP | stdio and Streamable HTTP JSON responses |
| Direct Gemini API | `gemini-3.8-flash` is a Google API identifier, not an AGY pin |

The AGY version floor is an executable contract: content operations and status
perform a strict version preflight. `models` remains a diagnostic command, so a
caller can inspect its local catalog when a capability check fails.

## AGY releases that affect this project

The latest AGY release reviewed for this change was 1.2.7. Read the upstream
[changelog](https://antigravity.google/changelog) before changing the AGY
integration; its source release notes are also pinned at
[7bb195a](https://github.com/google-antigravity/antigravity-cli/blob/7bb195acaec9e7788df5210d0dc3e15f3cefc6b3/CHANGELOG.md).

| AGY release | Relevant upstream behavior | Runtime response |
|---|---|---|
| 1.1.14 | Unified `inheritCustomizations` control | Generated agents explicitly disable inherited customization. |
| 1.1.25 | Markdown custom agents inherit ambient skills/rules/subagents by default | Generated agents retain explicit isolation fields. |
| 1.2.4 | Headless search/tool and schema-failure fixes | Typed server input continues to use structured output. |
| 1.2.6 | Headless print timeout default became unlimited; structured `AGY_ERROR` stderr behavior | Every runtime request supplies an explicit bounded deadline and returns sanitized errors. |
| 1.2.7 | Lower per-attempt backoff ceiling and headless background-progress fixes | Keep the wrapper's end-to-end timeout as the outer bound. |

These notes describe an upstream dependency, not a promise that a local AGY
installation has updated. Run `agy --version` and `agy models` in the account
that will run the workload.

## Gemini naming

Google documents the direct Gemini API model as
[`gemini-3.8-flash`](https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash).
AGY's catalog is separate and dynamic. It may expose effort-qualified slugs
such as `gemini-3.8-flash-low`, `gemini-3.8-flash-medium`, and
`gemini-3.8-flash-high`. `agy-search` verifies an explicit AGY slug against
fresh local discovery and requires its effort suffix to match the selected
effort. A direct Gemini API ID is not substituted when AGY does not advertise a
matching slug.

## LiteLLM and MCP

`/search` is designed for LiteLLM's Perplexity-compatible Search provider
interface. Configure `search_provider: perplexity` with this runtime as
`api_base`; the name selects LiteLLM's request/response transformation and does
not cause a request to Perplexity. The working configuration is in
[`examples/litellm-search.yaml`](../examples/litellm-search.yaml).

MCP stdio and HTTP both publish the same five tools and schemas. Stdio has no
HTTP bearer layer; HTTP requires a bearer token and validates Host plus any
configured Origin. See [`docs/server.md`](server.md) for transport and input
bounds.

## Deliberate limits

- The server implements standard verification only. Temporal comparison remains
  a CLI operation because it requires caller-owned scopes, exact source sets,
  and an `as-of` cutoff.
- Source/domain restrictions are caller-supplied membership constraints. They
  do not establish first-party ownership without a suitable caller-owned trust
  set.
- The existing Go A2A service is a separate deployment owner. This Rust runtime
  supplies LiteLLM Search and MCP; it does not replace A2A.
