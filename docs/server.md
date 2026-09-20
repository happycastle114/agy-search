# Server runtime

`agy-search-server` is the optional Rust runtime for callers that need a
network search provider or an MCP server. It reuses the same typed command
boundary, source validation, isolated AGY process, and JSON response documents
as the `agy-search` CLI. It does not replace the CLI.

Build or install a release that includes the `server` feature, then check the
surface before exposing it:

```bash
cargo build --locked --features server --bin agy-search-server
AGY_SEARCH_API_KEY='replace-with-a-random-secret' \
  ./target/debug/agy-search-server http
curl -i http://127.0.0.1:18091/healthz
```

The HTTP listener defaults to `127.0.0.1:18091`. The server requires an
Antigravity CLI installed and authenticated in the account that runs it. The
runtime never performs an AGY login or sends credentials to AGY on the command
line.

## Container image

The `Dockerfile` builds both binaries, installs verified AGY 1.2.7 for the
target Linux architecture, and runs as uid/gid `10001`. It sets the container
listener to `0.0.0.0:18091`; this overrides the binary's loopback default only
inside the image. Build it locally:

```bash
docker build --tag agy-search:0.4.0 .
docker volume create agy-search-home
docker run --detach --rm --name agy-search-server \
  --publish 127.0.0.1:18092:18091 \
  --mount type=volume,src=agy-search-home,dst=/home/agy-search \
  --env AGY_SEARCH_API_KEY='replace-with-a-random-secret' \
  agy-search:0.4.0
curl -i http://127.0.0.1:18092/healthz
```

Publishing host port `18092` keeps this runtime separate from the existing Go
A2A service on host port `18091`. For a published release, replace the local tag with `ghcr.io/happycastle114/agy-search:0.4.0`.

The image embeds neither AGY credentials nor a provider selection. Its named
volume preserves the dedicated `/home/agy-search` state, but copying a macOS
home directory into that volume is not an authentication procedure: Google
account authentication relies on the operating-system keyring, and macOS
Keychain state is not a Linux container keyring. AGY has no `login` subcommand
to script here. Follow Google's
[installation and authentication guidance](https://antigravity.google/docs/cli-install?hl=en)
and [keyring troubleshooting](https://antigravity.google/docs/cli/troubleshooting/)
in the runtime environment, then verify AGY with a non-sensitive command.

Gemini API-key authentication is also runtime-only. Supply `GEMINI_API_KEY`
through the deployment secret mechanism and set `modelProvider: gemini` in the
persistent AGY settings for the dedicated runtime state, as Google documents.
Do not bake either value into an image, Dockerfile, compose file, or example.

## HTTP endpoints

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /healthz` | no | Liveness only; it does not invoke AGY. |
| `GET /readyz` | bearer token | Backend readiness through the typed `status` operation. |
| `POST /search` | bearer token | LiteLLM-compatible Perplexity Search response. |
| `/mcp` | bearer token | Streamable HTTP MCP with JSON responses. |

Send the same token in `Authorization: Bearer …` for every protected endpoint.
The token is read from the environment variable named by `--api-key-env`
(default `AGY_SEARCH_API_KEY`), never from a CLI flag. The server accepts only
ASCII, non-whitespace tokens between 16 and 1,024 bytes.

The HTTP listener is loopback-only by default. It accepts `localhost`,
`127.0.0.1`, and `::1` Host values, plus the concrete configured listen address.
For a reverse proxy or network listener, configure each permitted host with
`--allowed-host` and each exact browser origin with `--allowed-origin`. No
origin is allowed by default. Keep TLS termination and network policy outside
the process; do not make this endpoint public with a copied example token.

```bash
export AGY_SEARCH_API_KEY='replace-with-a-random-secret'
agy-search-server \
  --agy-path /usr/local/bin/agy \
  --timeout 120 \
  --max-concurrency 2 \
  --catalog-ttl-seconds 60 \
  http \
  --listen 127.0.0.1:18091 \
  --allowed-host search.internal.example \
  --allowed-origin https://console.internal.example
```

`--max-concurrency` defaults to two and has a hard maximum of 64. Excess work
fails fast with HTTP 429; the runtime does not accumulate an unbounded queue.
`--timeout` is the end-to-end deadline for a request, including batch members
and AGY discovery, and defaults to 120 seconds. A request body for `/search` or
`/mcp` defaults to 131,072 bytes and can be configured from 1,024 through
1,048,576 with `--body-limit-bytes`.

The server uses structured diagnostics on stderr. MCP SDK diagnostics are
disabled even when `RUST_LOG` enables debug or trace, because those events can
contain request/response payloads and client metadata. Sanitized application
errors and HTTP status/latency logs remain available. MCP stdio keeps stdout
for protocol NDJSON only. Do not redirect stderr into a stdio MCP client.

## LiteLLM Search provider

LiteLLM's native `perplexity` search-provider contract is a compatible client
for `POST /search`: it sends a query or batch of queries and expects a Search
response with result URL, title, snippet, and optional date fields. It does not
turn this runtime into a Perplexity service or use Perplexity credentials.

Start the server first and configure LiteLLM with
[`examples/litellm-search.yaml`](../examples/litellm-search.yaml). The example
uses an environment reference for the bearer token. Its `api_base` points to
the runtime's `/search` host, not to a model-completions endpoint.

The raw HTTP surface accepts these deliberately narrow fields:

```json
{
  "query": "IANA example domains",
  "max_results": 5,
  "search_domain_filter": ["iana.org"],
  "source_urls": ["https://www.iana.org/help/example-domains"],
  "max_tokens_per_page": 4096,
  "country": "US",
  "thinking_level": "low",
  "search_profile": "standard"
}
```

`query` accepts one non-empty string or one to five non-empty strings. Results
are deduplicated in request order and capped by `max_results` (1 through 20).
`source_urls` is an exact HTTPS allowlist; `search_domain_filter` is a domain
tree allowlist. `max_tokens_per_page` must be positive when present, and
`country` is limited to 128 bytes. Unknown fields, empty strings, oversized
queries, unsupported profiles, and temporal CLI fields are rejected.

The server currently exposes only `standard` verification. Use the standalone
CLI for temporal comparison with explicit scopes, sources, and an `as-of` date.
Successful standard search proves retained, verified result evidence; it does
not prove an answer is complete for every requested fact.

`source_urls`, `thinking_level`, and `search_profile` are raw HTTP/MCP runtime
extensions. LiteLLM 1.102's native Perplexity transformer forwards its standard
search fields only: query, maximum results, domain filter, maximum tokens per
page, and country. It does not forward those extensions. Call `/search`
directly or use MCP when an exact source allowlist or runtime-specific thinking
control is required.

## MCP

For a local MCP host, use
[`examples/mcp-stdio.json`](../examples/mcp-stdio.json). Stdio is intended for
one local client process and needs no bearer token because the operating-system
process boundary supplies access control.

For a Streamable HTTP MCP client, use
[`examples/mcp-http.json`](../examples/mcp-http.json), substitute its
environment-specific token interpolation, and keep the endpoint on a protected
network. The runtime offers five read-only, idempotent, open-world tools:

| Tool | Input |
|---|---|
| `agy_search` | the bounded Search request above |
| `agy_extract` | 1–20 exact HTTPS URLs and optional query |
| `agy_map` | one HTTPS URL, limit 1–100, optional instructions/external flag |
| `agy_crawl` | one HTTPS URL, limit 1–50, optional instructions/external flag |
| `agy_research` | non-empty query, 1–20 sources, optional domains/exact URLs |

Each tool returns the normal `agy-search` structured document rather than
model-written unbounded text. Closing an MCP transport or shutting down the
server cancels its request context; process cleanup remains the runtime's
responsibility.

## AGY compatibility

This project requires AGY 1.1.20 or newer. AGY 1.2.7 was the newest verified
release when this runtime was prepared. AGY 1.2.6 changed headless mode so its
default print timeout is unlimited; this runtime always passes an explicit,
request-bounded print deadline instead of inheriting that default. See the
[AGY headless documentation](https://antigravity.google/docs/cli/headless/) and
the [AGY changelog](https://antigravity.google/changelog) for the upstream
contract.

The generated AGY agent explicitly sets `inheritCustomizations: false` and
`inheritMcp: false`. The unified inheritance control has existed since AGY
1.1.14, and AGY 1.1.25 made Markdown custom agents inherit ambient skills,
rules, subagents, and related defaults by default. The explicit settings keep a
server request independent from ambient client customization.

`gemini-3.8-flash` is the direct Gemini API model identifier documented by
[Google AI for Developers](https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash).
AGY publishes dynamic runtime model slugs instead; verified AGY catalogs have
listed `gemini-3.8-flash-low`, `gemini-3.8-flash-medium`, and
`gemini-3.8-flash-high`. Do not pass the direct API identifier as an AGY pin.
Leave `--model` unset unless a fresh `agy models` response contains the exact
AGY slug, and match the slug suffix to `--effort`.

## Operations and rollback

The advisory model catalog cache stores only a successful unpinned catalog
lookup for the current AGY executable and working directory. Its default TTL is
60 seconds; set `--catalog-ttl-seconds 0` to disable reuse. Explicit model
pins, readiness/status, version checks, and failed or empty discovery are never
served from that cache.

The runtime is separate from the existing Go A2A service: keep that service on
its current port and route only LiteLLM search traffic to the Rust runtime after
an authenticated readiness and search readback. Roll back by restoring the
previous LiteLLM `api_base`; do not delete or modify the A2A service as part of
a provider rollback.

For GitOps environments, commit the image/configuration change, wait for the
declared reconciler revision and health, then test `/readyz`, the LiteLLM search
call, and MCP over the deployed route. Do not use an imperative cluster patch
as a replacement for the reviewed deployment path.

## Observed interoperability

The following checks used real AGY 1.2.7 and a restricted IANA query. They are
surface evidence only, not an availability or latency promise.

| Surface | Observation |
|---|---|
| Raw HTTP `/search` | Two exact-IANA responses returned HTTP 200 in 32.8375 s and 8.4382 s. They were not an isolated cache comparison. |
| LiteLLM native client | Two domain-scoped attempts returned HTTP 504 and HTTP 502; a subsequent known-good IANA query returned HTTP 200. This demonstrates one interoperable response after failures; it does not demonstrate reliable availability or a latency median. |
| MCP | The official Python SDK 2.2.0 completed HTTP `agy_search` with default modern negotiation and stdio `agy_extract` with legacy negotiation against real AGY. Both returned `is_error: false`; both transports listed all five tools with the default client. |
