---
name: agy-search
description: Use the agy-search CLI for source-backed live web search, URL extraction, site mapping, bounded crawling, and multi-source research through Google Antigravity. Trigger when a task needs current web facts, primary sources, content from known URLs, documentation discovery, website analysis, comparisons, literature or market research, or a cited report without a separate search API key.
---

# Agy Search

Use the smallest operation that can answer the task, preserve the returned URLs,
and escalate only when the evidence is insufficient.

## Preflight

Run this once before the first research command in the current agent session.
Do not repeat it for every query after it succeeds:

```bash
command -v agy-search
agy --version
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

The supported Antigravity floor is 1.1.10. If the user explicitly asks to
update a release-installer copy of `agy-search`, rerun the release installer and
then repeat preflight:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy-search status
```

The installer verifies the selected application archive before replacement and
does not install a second updater executable. Never update either `agy-search`
or `agy` implicitly. Antigravity manages its own background updates during
regular runs.

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

Fast default: run one `agy-search search "query" -n 3` and answer immediately
when its primary-source snippets directly support the claims. The CLI already
uses low effort by default. Do not extract or research merely to improve prose;
escalate only when snippets are insufficient, the user requests comparison or
deep synthesis, sources conflict, or the task is consequential/high-stakes.
Use `--effort medium` or `--effort high` only for that deliberate escalation.

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
- Interpret `date` only as an explicit publication/release date and
  `last_updated` only as an explicit modification/update date; `null` means the
  structured result did not supply that metadata.
- Prefer primary sources for technical, legal, scientific, and product claims.
- Cross-check consequential or time-sensitive claims with more than one source.
- State uncertainty when the sources disagree or the bounded result is thin.
- Never cite the CLI, a local `.agy-search/` path, or an unreturned URL as the
  source of a factual claim.

Read [references/commands.md](references/commands.md) when selecting flags,
handling exit codes, or consuming the exact response shapes.
