# agy-search

`agy-search` is a small standalone Rust CLI that turns Google Antigravity print
mode into a source-backed JSON interface for web search, extraction, URL
mapping, bounded crawling, and cited research. It needs no separate search API
key and no Python or Node.js runtime.

Antigravity 1.1.8 added print-mode `json`, `stream-json`, and custom JSON Schema
enforcement. Releases 1.1.9 and 1.1.10 added print-mode slash expansion and
fixed headless model/effort selection. `agy-search` uses the structured contract,
disables slash expansion so input stays data, and validates the event evidence
and result again before anything reaches stdout.

## Install

Install the latest GitHub release on macOS or Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh | sh
```

The installer selects the matching Apple Silicon, Intel Mac, Linux ARM64, or
Linux x86-64 archive and verifies its checksum. Then check both layers:

```bash
agy-search --version
agy --version
agy-search status
```

To update a release-installer copy later, rerun the same one-liner and verify
both layers again:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy --version
agy-search status
```

The installer replaces the CLI only after the selected release archive passes
its SHA-256 check. That checksum is a same-release integrity check, not an
independent signature or provenance attestation. No second updater executable
is downloaded. Source or package-manager installations must continue updating
through their original channel.

For a source install instead:

```bash
cargo install --git https://github.com/happycastle114/agy-search --locked
```

## Requirements

- An installed and signed-in Google Antigravity CLI 1.1.10 or newer
- `curl`, used for bounded Standard Search terminal-URL validation and opt-in
  temporal source-body verification
- Web tools available to the selected Antigravity model

```bash
command -v agy-search
command -v agy
command -v curl
agy-search --version
agy --version
```

Run `agy models` yourself only to choose an explicit model pin. An unpinned
low-effort Standard Search owns its single bounded advisory lookup internally.

Before every content command, and before `status` claims availability,
`agy-search` runs the cheap `agy --version` preflight. It accepts exactly the
official bare release shape `X.Y.Z`, compares numeric semantic components, and
rejects a bare semantic version below 1.1.10 before content or status can start.
Prefixes, pre-release/build suffixes, and extra lines are rejected rather than
guessed, so an ambiguous capability report cannot reach `-p` or model discovery.
`models` remains an explicit diagnostic command and does not run the floor
check.

Antigravity manages its own background updates during regular runs. `agy-search`
never updates either executable implicitly; run update workflows only when that
mutation is intentional.

## Commands

Every successful command emits canonical JSON. `-o/--output` atomically writes
that JSON to disk and keeps stdout empty.

```bash
agy-search status
agy-search models

agy-search search "Antigravity structured output" --max-results 5
printf '%s\n' "recent agent search tooling" | agy-search search -

agy-search extract https://example.com/docs https://example.com/changelog
agy-search map https://example.com --limit 50
agy-search crawl https://example.com --instructions "API documentation" --limit 20
agy-search research "Compare current agent web research CLIs" --max-sources 10

agy-search research "topic" -o .agy-search/research.json
```

Research derives its attempted web-tool call budget from `--max-sources`: two
discovery/verification attempts beyond the requested source count, capped at 12.

`--json` is accepted on every command for explicit script compatibility; JSON
is already the only success format. Global options may appear before or after
the subcommand:

```bash
agy-search --effort high --timeout 180 search "query"
agy-search --effort low --timeout 75 --verification temporal-comparison \
  search "latest Antigravity CLI release" \
  --scope "Antigravity CLI" \
  --source-url https://antigravity.google/changelog \
  --as-of 2026-08-06
agy-search --effort low --timeout 75 --verification temporal-comparison \
  search "latest across tracks" \
  --scope "Product A" --scope "Product B" \
  --source-url https://vendor.example/changelog \
  --as-of 2026-08-06
agy-search --agy-path /absolute/path/to/agy status
```

`--model` is checked strictly against the current `agy models` output. A slug ending in
`-low`, `-medium`, or `-high` must use the matching `--effort`. When `--model`
is omitted for standard Search at low effort, `agy-search` makes one advisory
catalog query, bounded to five seconds within the caller deadline, and selects
`gemini-3.6-flash-low` only when that exact catalog entry is present. If it is
absent or the advisory query fails while time remains, content omits `--model`
and uses the provider default. Temporal Search, Research, Extract, Map, Crawl,
and medium/high effort Search do not make that preference query. The
same catalog may supply `gemini-3.6-flash-medium` for the first bounded recovery
and `gemini-3.6-flash-high` for the final recovery. Missing tiers fall back
without adding an attempt, and every attempt shares the original deadline. The
`AGY_SEARCH_AGY_PATH` environment variable can select the downstream executable
without adding its path to command history. `AGY_SEARCH_CURL_PATH` can select a
non-default curl executable for the bounded source-link resolver and temporal
source verifier. Content commands default to low
reasoning effort for latency; pass `--effort medium` or `--effort high` only
when the task needs deeper synthesis. For Search and Research, `--domain` is a
caller-owned domain-tree allowlist (the named host plus its subdomains), while
`--source-url` is a canonical exact-URL allowlist. In standard mode, either flag
restricts returned/audited membership only and `--source-url` does not fetch a
source body. Standard Search still performs a metadata-only terminal HTTPS
reachability check. Use `--verification temporal-comparison` as temporal source
verification across 1-8 exact caller-owned `--scope` values and 1-8 canonical
HTTPS `--source-url` values. One scope verifies that exact latest tuple and
requires `--as-of`; 2-8 scopes additionally select the unique newest member of
the declared set. Ordinary searches keep the faster `standard` default and
perform no source-body fetch.
Search and research accept `--as-of YYYY-MM-DD` only in temporal mode. It is an
inclusive cutoff for explicit source publication/release dates, not a crawl,
query, or execution date; standard mode rejects it. Never use a future cutoff.
Temporal mode fetches distinct declared sources concurrently after the primary
response, caches each unique URL once, and shares the command deadline.
There, an exact `--source-url` is fetched and verified and dominates any
same-domain path allowed by `--domain`.
Temporal search may make one
bounded, all-or-nothing recovery wave across the declared scopes. Temporal
research remains one-shot and fails closed instead of retrying.

| Command | Purpose | Important bounds |
|---|---|---|
| `status` | Prove supported version and authenticated model discovery | Shared invocation deadline |
| `models` | Return current model slugs for diagnostics | Non-empty, unique slugs; no version guard |
| `search` | Discover live sources | 1-20 results, repeatable `--domain`/`--source-url` |
| `extract` | Read exact pages | 1-20 HTTP(S) URLs |
| `map` | Discover website URLs | 1-100, same-origin by default |
| `crawl` | Read website pages | 1-50, same-origin by default |
| `research` | Synthesize cited findings | 1-20 sources, repeatable `--domain`/`--source-url` |

`map` and `crawl` accept `--allow-external` only when cross-origin results are
intentional. They are bounded agent operations, not exhaustive-site promises.

## Agent skill and OpenCode

The repository includes
[`skills/agy-search`](https://github.com/happycastle114/agy-search/tree/main/skills/agy-search),
which teaches an agent when to search, extract, map, crawl, or escalate to a
multi-source report. The separate
[`opencode-agy-search`](https://github.com/happycastle114/opencode-agy-search)
plugin bundles the skill, registers it with OpenCode, and forwards the documented
executable override while invoking this same binary.

The skill saves potentially large responses under `.agy-search/` and reads only
the needed fields, limiting context use without losing citation URLs.

## Performance

The 0.2.4 distribution binaries measure 1,072,064 B on Apple Silicon and
1,395,344 B on Linux x86-64. On the measured host on 2026-08-06, real
`agy --version` averaged 41.0 ms over 10 runs and a deterministic fake search
including the required version guard averaged 47.1 ms over 10 runs. The guard
is one local process before content, not a cache; its cost remains small next to
real model/web-tool execution. Linux x86-64 CI guards a 1.40 MiB binary-size
budget. See [docs/performance.md](docs/performance.md) for the historical
comparison and size attribution.

The release installer deliberately does not add a second updater executable;
rerunning it preserves the checksum-verified archive path.

The fast normal path uses the short standard prompt and keeps the default low
effort. It opportunistically pins the catalog-advertised fast Search model only
when its bounded advisory discovery succeeds; otherwise it leaves `--model`
unset for the provider default. Temporal comparison adds canonical-page and
complete-scope checks only when explicitly selected. Pinning a model
intentionally runs fresh `agy models` discovery first so an invalid slug keeps
its stable exit contract; use it when reproducibility matters. Search also bounds live tool calls and tells
Antigravity to emit structured output as soon as sufficient evidence exists. See
[docs/performance.md](docs/performance.md) for the benchmark method and measured
boundaries.

## Output and failure contract

Each content response has an `object` discriminator: `search`, `extract`, `map`,
`crawl`, or `research`. Sources must be unique HTTP(S) URLs. Research citations
must reference sources in the same response.

| Exit | Meaning |
|---:|---|
| 0 | Success |
| 2 | Invalid CLI input or output target |
| 3 | `agy` executable unavailable |
| 4 | Downstream timeout |
| 5 | Downstream process failed |
| 6 | Invalid, incomplete, sourceless, or unsupported-version output |
| 7 | Requested model absent from `agy models` |

Diagnostics go to stderr. Stdout never contains downstream logs, private session
identifiers, or tool payloads.

## Trust model

- Launches `agy` with direct argv, never a shell.
- Uses `stream-json` and a generated JSON Schema without forcing plan mode.
- Disables print-mode slash/skill expansion so request fields remain data.
- Defaults standard search to low effort: one primary attempt followed, only if
  no publishable result survives, by at most two bounded recovery attempts. A
  complete catalog uses low, then medium, then high. All attempts share the
  original invocation deadline.
- Sends typed primary-first, complete-scope, and explicit-source-date policies.
- Requires an internal evidence audit, validates public URLs against it, and
  removes the audit from the stable public JSON response. Typed temporal
  comparison mode additionally requires exact equality with the caller-owned
  scope set and source allowlist. Each scope/value/source-date tuple is checked
  against one deterministic section in a safely fetched source body, and the
  normalized date must agree with the source date text. Search exposes exactly
  one unique newest declared winner. Research may expose several supporting
  sources, but every candidate must remain bound to a declared source and the
  unique latest candidate must remain publicly visible.
- Fetches temporal source bodies with HTTPS-only URLs, no URL credentials,
  public-address DNS validation and pinning, no redirects or ambient proxy,
  config, or cookies, bounded UTF-8 bodies, and the invocation's shared
  deadline. Standard mode never uses this fetch path.
- Preserves exact search constraints and uses the second bounded tool call only
  to locate or read the canonical evidence page when verification needs it.
- Validates every Standard Search result and audit URL through shell-free fixed
  curl arguments: HTTPS-only protocols, five redirects, bounded
  connect/request time, and one DNS-pinned terminal target. Validation starts
  with HEAD; publishers that reject HEAD receive one range-requested GET capped
  at 2 MiB under the same deadline and redirect policy.
  Google search, transport, and cache origins are never public sources. A
  failed or unsafe row is removed with its audit row; if no publishable result
  survives, standard search may use up to two bounded recovery attempts. Each
  redirect hop is parsed, public-address validated,
  and pinned before the next request.
- Counts every attempted built-in `search_web` or `read_url_content` lifecycle
  toward the budget and requires all started calls to complete successfully.
- Rejects generic MCP calls and merely started tools as provenance.
- Runs content work in an exact `tempfile`-owned directory.
- Never passes `--dangerously-skip-permissions` or automates login.
- Treats fetched pages as untrusted data in the research prompt.

The CLI validates provenance structure, declared-set coverage, source-body
binding, schemes, bounds, and citation membership. Standard mode remains a fast,
best-effort web answer and does not disambiguate an exact latest track from source
body structure; use temporal mode with explicit `--scope`, `--source-url`, and
`--as-of` for that claim. Temporal exit 0 is exhaustive
only relative to the caller-declared scopes and sources. It cannot prove that an
unknown global inventory is complete or guarantee source truthfulness;
independently verify high-stakes claims. `--domain` and `--source-url` prove
caller-specified membership, never official, first-party, or project ownership;
Antigravity also cannot guarantee that no third-party snippet was ever viewed
during its search process. A hard official/first-party/project-maintained request
MUST pass explicitly trusted domains or exact URLs and keep the ownership
constraint in the query. If that exact trust set is unavailable, stop and report
that mechanical enforcement is impossible, or do only user-permitted discovery
and label its candidates unverified.

`date` means an explicitly exposed publication or release date.
`last_updated` means a separately exposed modification or update date. Missing
publication/release dates stay `null`. Standard Search also downgrades an exact
date to `null` when the returned same-URL evidence does not bind its complete
source date text; it preserves the source result instead of exposing unsupported
metadata. Malformed dates still fail, and Standard Research remains strict. The
CLI never substitutes execution, crawl, fetch, query, or cutoff time, infers a
date, or copies one meaning into the other. Temporal comparison instead requires
strict ISO dates for every ordered candidate and rejects missing or null dates
with exit 6. Its current source audit does not bind modification dates, so
temporal public results must use `last_updated: null`; any non-null value also
fails closed with exit 6.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --profile dist --locked
```

Real authenticated tests are ignored by default so CI cannot spend Antigravity
usage. Run the complete discovery plus five-operation gate explicitly:

```bash
AGY_SEARCH_AGY_PATH=/absolute/path/to/agy \
AGY_SEARCH_REAL_MODEL=gemini-3.6-flash-low \
cargo test --test real_antigravity --locked -- --ignored --nocapture
```
