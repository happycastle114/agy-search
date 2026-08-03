# Command reference

## Global contract

Put global options before the subcommand:

```bash
agy-search [--agy-path PATH] [--model SLUG] [--effort low|medium|high] \
  [--timeout SECONDS] COMMAND
```

- Set `AGY_SEARCH_AGY_PATH` instead of `--agy-path` when appropriate.
- Discover `SLUG` with `agy-search models` in the current environment.
- Successful stdout is canonical JSON. Diagnostics use stderr.
- Add `-o PATH` to atomically write JSON and keep stdout empty.
- Use query `-` to read up to 100 KiB from stdin for `search` and `research`.

## Operations

```bash
agy-search status [-o PATH]
agy-search models [-o PATH]

agy-search search QUERY [-n N|--max-results N] [--domain DOMAIN]... \
  [--country COUNTRY] [--max-tokens-per-page N] [-o PATH]

agy-search extract URL... [--query TEXT] [-o PATH]

agy-search map URL [--limit N] [--instructions TEXT] [--allow-external] [-o PATH]

agy-search crawl URL [--limit N] [--instructions TEXT] [--allow-external] [-o PATH]

agy-search research QUERY [--max-sources N] [-o PATH]
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
