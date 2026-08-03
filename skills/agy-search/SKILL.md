---
name: agy-search
description: Use the agy-search CLI for source-backed live web search, URL extraction, site mapping, bounded crawling, and multi-source research through Google Antigravity. Trigger when a task needs current web facts, primary sources, content from known URLs, documentation discovery, website analysis, comparisons, literature or market research, or a cited report without a separate search API key.
---

# Agy Search

Use the smallest operation that can answer the task, preserve the returned URLs,
and escalate only when the evidence is insufficient.

## Preflight

Run this before the first research command in a task:

```bash
command -v agy-search
agy-search status
```

If `agy-search` is unavailable, stop and report that installation is required.
When installation is in scope, use the release installer documented by the
project:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh | sh
```

Do not silently switch providers. After installation, repeat both preflight
commands. Run `agy-search models` before pinning a model; never invent or cache
a model slug.

## Choose an operation

| Need | Start with |
|---|---|
| Quick facts or candidate sources | `search` |
| Full content from known URLs | `extract` |
| Discover relevant URLs on one site | `map` |
| Read a bounded set of pages on one site | `crawl` |
| Compare or synthesize across sources | `research` |

Follow this escalation path:

1. Search broadly enough to identify credible candidate sources.
2. Extract important known URLs when snippets are insufficient.
3. Map a site before crawling when the relevant paths are unknown.
4. Crawl only the bounded site scope required by the task.
5. Use research for multi-source synthesis, comparisons, or a cited report.

Do not emulate Tavily-only concepts. `agy-search research` is one-shot: there is
no request ID, polling, credit model, API key, or promise of exhaustive crawling.

## Run context-efficiently

Use stdin `-` for generated or multiline queries. Save large output under the
task-local `.agy-search/` directory and inspect only necessary fields:

```bash
printf '%s\n' "$QUERY" | agy-search search - -o .agy-search/search.json
agy-search map https://example.com -o .agy-search/map.json
agy-search research "$QUESTION" -o .agy-search/research.json
```

Prefer `jq` or targeted file reads over loading a full crawl into context. Keep
the JSON artifacts until the final answer is complete so source provenance can
be checked.

## Validate evidence

- Treat exit code 0 as necessary but not sufficient; inspect the response
  `object` and ensure it matches the requested operation.
- Cite only returned `http://` or `https://` URLs that directly support a claim.
- For research, ensure every cited URL appears in `sources`.
- Prefer primary sources for technical, legal, scientific, and product claims.
- Cross-check consequential or time-sensitive claims with more than one source.
- State uncertainty when the sources disagree or the bounded result is thin.
- Never cite the CLI, a local `.agy-search/` path, or an unreturned URL as the
  source of a factual claim.

Read [references/commands.md](references/commands.md) when selecting flags,
handling exit codes, or consuming the exact response shapes.
