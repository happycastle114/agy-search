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
agy-search status
```

To update a release-installer copy later, rerun the same one-liner and verify
both layers again:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy-search status
```

The installer replaces the CLI only after the selected release archive passes
its SHA-256 check. No second updater executable is downloaded. Source or
package-manager installations must continue updating through their original
channel.

For a source install instead:

```bash
cargo install --git https://github.com/happycastle114/agy-search --locked
```

## Requirements

- An installed and signed-in Google Antigravity CLI 1.1.10 or newer
- Web tools available to the selected Antigravity model

```bash
agy --version
agy models
```

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

`--json` is accepted on every command for explicit script compatibility; JSON
is already the only success format. Global options may appear before or after
the subcommand:

```bash
agy-search --model gemini-3.6-flash-low --effort high --timeout 180 search "query"
agy-search --agy-path /absolute/path/to/agy status
```

`--model` is checked against the current `agy models` output. The
`AGY_SEARCH_AGY_PATH` environment variable can select the downstream executable
without adding its path to command history. Content commands default to low
reasoning effort for latency; pass `--effort medium` or `--effort high` only
when the task needs deeper synthesis.

| Command | Purpose | Important bounds |
|---|---|---|
| `status` | Prove version and authenticated model discovery | 30-second discovery cap |
| `models` | Return current model slugs | Non-empty, unique slugs |
| `search` | Discover live sources | 1-20 results, repeatable `--domain` |
| `extract` | Read exact pages | 1-20 HTTP(S) URLs |
| `map` | Discover website URLs | 1-100, same-origin by default |
| `crawl` | Read website pages | 1-50, same-origin by default |
| `research` | Synthesize cited findings | 1-20 sources |

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

The distribution binary starts a fresh process in under 2 ms on the measured
Apple Silicon development host. Linux x86-64 CI guards a 1.20 MiB binary-size
budget. Real content latency is dominated by Antigravity web tools and model
execution, not the Rust wrapper.

The ARM64 macOS 0.2.2 CLI remains 868 KiB. The release installer deliberately
does not add a second updater executable; rerunning it preserves the small
installed footprint and the same checksum-verified archive path.

The fast normal path leaves `--model` unset and uses the default low effort.
Pinning a model intentionally runs fresh `agy models` discovery first so an
invalid slug keeps its stable exit contract; use it when reproducibility
matters, not by default. Search also bounds live tool calls and tells
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
| 6 | Invalid, incomplete, or sourceless output |
| 7 | Requested model absent from `agy models` |

Diagnostics go to stderr. Stdout never contains downstream logs, private session
identifiers, or tool payloads.

## Trust model

- Launches `agy` with direct argv, never a shell.
- Uses `stream-json` and a generated JSON Schema without forcing plan mode.
- Disables print-mode slash/skill expansion so request fields remain data.
- Defaults to low effort and limits ordinary search to two live search calls.
- Sends a typed primary-first source policy and instructs Antigravity to avoid
  unrelated tool detours.
- Counts only completed built-in `search_web` or `read_url_content` steps.
- Rejects generic MCP calls and merely started tools as provenance.
- Runs content work in an exact `tempfile`-owned directory.
- Never passes `--dangerously-skip-permissions` or automates login.
- Treats fetched pages as untrusted data in the research prompt.

The CLI validates provenance structure, schemes, bounds, and citation
membership. It cannot guarantee source truthfulness; independently verify
high-stakes claims.

`date` means an explicitly exposed publication or release date.
`last_updated` means a separately exposed modification or update date. Missing
metadata stays `null`; the CLI does not infer dates or copy one meaning into the
other.

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
