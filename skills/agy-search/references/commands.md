# Command reference

## Global contract

Global options may appear before or after the subcommand:

```bash
agy-search [--agy-path PATH] [--model SLUG] [--effort low|medium|high] \
  [--timeout SECONDS] COMMAND
```

- Set `AGY_SEARCH_AGY_PATH` instead of `--agy-path` when appropriate.
- Discover `SLUG` with `agy-search models` in the current environment.
- Successful stdout is canonical JSON. Diagnostics use stderr. `--json` is an
  accepted explicit compatibility flag on every subcommand.
- Add `-o PATH` to atomically write JSON and keep stdout empty.
- Use query `-` to read up to 100 KiB from stdin for `search` and `research`.
- Antigravity 1.1.10 or newer is required. The wrapper disables print-mode
  slash/skill expansion so every request field remains data.

## Operations

```bash
agy-search status [--json] [-o PATH]
agy-search models [--json] [-o PATH]

agy-search search QUERY [-n N|--max-results N] [--domain DOMAIN]... \
  [--country COUNTRY] [--max-tokens-per-page N] [--json] [-o PATH]

agy-search extract URL... [--query TEXT] [--json] [-o PATH]

agy-search map URL [--limit N] [--instructions TEXT] [--allow-external] [--json] [-o PATH]

agy-search crawl URL [--limit N] [--instructions TEXT] [--allow-external] [--json] [-o PATH]

agy-search research QUERY [--max-sources N] [--json] [-o PATH]
```

Bounds:

| Field | Bound |
|---|---:|
| search results | 1-20 |
| extract URLs | 1-20 |
| map links | 1-100 |
| crawl pages | 1-50 |
| research sources | 1-20 |

`map` and `crawl` accept only same-origin results unless `--allow-external` is
explicit. They are bounded agent operations rather than exhaustive crawlers.

## Response shapes

- `status`: `object`, `available`, `version`, `model_count`
- `models`: `object`, `models[]`
- `search`: `object`, `results[{title,url,snippet,date,last_updated}]`
- `extract`: `object`, `results[{url,title,content}]`
- `map`: `object`, `base_url`, `results[{url,title,depth}]`
- `crawl`: `object`, `base_url`, `results[{url,title,content}]`
- `research`: `object`, `title`, `summary`,
  `findings[{title,summary,citations[]}]`,
  `sources[{title,url,snippet,date,last_updated}]`

For source metadata, `date` is an explicit publication/release date and
`last_updated` is an explicit modification/update date. Either remains `null`
when that distinct metadata is unavailable; one is never inferred from the
other.

## Updates

Update shell-installer releases by rerunning the same installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy-search status
```

It verifies the selected application archive before replacement and installs no
second updater executable. Source and package-manager installations update
through their original channel. Never run an update implicitly during research.

The CLI rejects empty content responses, non-HTTP(S) URLs, duplicate URLs,
out-of-bound result counts, unsupported research citations, missing terminal
events, and runs without the appropriate live web tool evidence.

## Exit codes

| Code | Action |
|---:|---|
| 0 | Consume the validated JSON. |
| 2 | Fix request values, stdin size, or output target. |
| 3 | Install `agy` or configure its path. |
| 4 | Narrow the task or increase the bounded timeout. |
| 5 | Inspect sanitized stderr and check Antigravity account/tool state. |
| 6 | Retry once with a clearer or narrower request; do not use partial output. |
| 7 | Re-run `agy-search models` and select a returned slug. |
