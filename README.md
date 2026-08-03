# agy-search

`agy-search` is a standalone, source-backed web research CLI built on Google
Antigravity print mode. It gives scripts and agents a stable JSON interface for
search, extraction, URL mapping, bounded crawling, and cited research without a
separate search API key.

It uses the official `agy` CLI directly. Antigravity 1.1.8 introduced print-mode
`json`, `stream-json`, and custom JSON Schema enforcement; `agy-search` consumes
that event contract and validates it again before returning data.

## Requirements

- Python 3.12 or newer
- An installed and signed-in Google Antigravity CLI 1.1.8 or newer
- Web tools available to the selected Antigravity model

Check the downstream installation first:

```bash
agy --version
agy models
```

## Install

With `uv`:

```bash
uv tool install agy-search
```

With `pipx`:

```bash
pipx install agy-search
```

For local development:

```bash
git clone https://github.com/happycastle114/agy-search.git
cd agy-search
uv sync --all-groups
uv run agy-search status
```

## Commands

All successful commands emit canonical JSON. Use `-o/--output` to write it
atomically to disk while keeping stdout empty.

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
is already the default and only success format.

Global options go before the command:

```bash
agy-search --model gemini-3.6-flash-low --effort high --timeout 180 search "query"
agy-search --agy-path /absolute/path/to/agy status
```

`--model` is checked against the current output of `agy models`; unknown slugs
exit before a research run. `AGY_SEARCH_AGY_PATH` can set the downstream
executable without putting it in command history.

| Command | Purpose | Important bounds |
|---|---|---|
| `status` | Prove version and authenticated model discovery | 30-second discovery cap |
| `models` | Return current model slugs | Non-empty, unique slugs |
| `search` | Discover sources | 1-20 results, repeatable `--domain` |
| `extract` | Read explicit URLs | 1-20 HTTP(S) URLs |
| `map` | Discover website URLs | 1-100, same-origin by default |
| `crawl` | Read website pages | 1-50, same-origin by default |
| `research` | Synthesize cited findings | 1-20 sources |

`map` and `crawl` accept `--allow-external` when cross-origin results are
intentional. They are bounded agent operations, not promises of exhaustive site
coverage.

## Agent skill

The repository ships
[`skills/agy-search`](https://github.com/happycastle114/agy-search/tree/main/skills/agy-search),
a concise
workflow skill that teaches agents when to search, extract, map, crawl, or
escalate to research. Install that folder in the skill directory used by your
agent, or use the separately published OpenCode plugin that bundles it. The
Python wheel also contains the same folder under
`agy_search/skills/agy-search`, so wheel-only mirrors do not lose the skill.

The skill intentionally saves potentially large results under `.agy-search/`
and reads only the needed fields, reducing context use while preserving the
source URLs required for citations.

## Output and failure contract

Each content response has a distinct `object` discriminator: `search`,
`extract`, `map`, `crawl`, or `research`. Returned sources must be real HTTP(S)
URLs. Research citations must reference URLs present in the same response.

| Exit | Meaning |
|---:|---|
| 0 | Success |
| 2 | Invalid CLI input or output target |
| 3 | `agy` executable unavailable |
| 4 | Downstream timeout |
| 5 | Downstream process failed |
| 6 | Invalid, incomplete, or sourceless output |
| 7 | Requested model absent from `agy models` |

Diagnostics go to stderr; stdout never contains downstream logs. Private
session identifiers and tool payloads are not included in public output.

## Trust model

- Launches `agy` with direct argv, never through a shell.
- Uses plan mode, `stream-json`, and a generated JSON Schema.
- Requires evidence that an appropriate live web tool actually ran.
- Counts only completed built-in `search_web` or `read_url_content` steps;
  generic MCP calls and merely started tool steps do not satisfy provenance.
- Runs content work in a marker-owned temporary directory and removes only that
  exact directory.
- Does not pass `--dangerously-skip-permissions` or automate Antigravity login.
- Treats fetched pages as untrusted data in the research prompt.

The CLI validates provenance structure, URL schemes, bounds, and citation
membership. It cannot guarantee that a source is truthful or that a model's
summary is factually correct; independently verify high-stakes claims.

## Development

```bash
uv run pytest -q
uv run ruff check src tests
uv run ruff format --check src tests
uv run basedpyright
uv build
```

Real authenticated tests are opt-in so normal CI does not spend Antigravity
credits.

```bash
AGY_SEARCH_RUN_REAL_E2E=1 \
AGY_SEARCH_AGY_PATH=/absolute/path/to/agy \
AGY_SEARCH_REAL_MODEL=gemini-3.6-flash-low \
uv run pytest -q -s -m real_agy tests/test_real_agy.py
```

That gate re-discovers the model and exercises real `stream-json` events for
search, extract, map, crawl, and research. Never put Antigravity credentials in
the repository or CI variables for this test.
