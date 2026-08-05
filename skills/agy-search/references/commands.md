# Command reference

## Global contract

Before the first content command in an agent session, use this cheap local
preflight and do not discover models unless an explicit pin is requested:

```bash
command -v agy-search
command -v agy
command -v curl
agy-search --version
agy --version
```

Global options may appear before or after the subcommand:

```bash
agy-search [--agy-path PATH] [--model SLUG] [--effort low|medium|high] \
  [--verification standard|temporal-comparison] [--timeout SECONDS] COMMAND
```

- Set `AGY_SEARCH_AGY_PATH` instead of `--agy-path` when appropriate.
- Set `AGY_SEARCH_CURL_PATH` only when the default `curl` executable is not the
  intended grounding-link resolver/source verifier.
- Discover `SLUG` with `agy-search models` in the current environment.
- Content commands default to `--effort low`. Raise effort only for deliberate
  deep synthesis; explicit effort always overrides the default.
- Ordinary work omits `--model`. For an explicit pin whose returned slug ends
  in `-low`, `-medium`, or `-high`, pass the matching `--effort`; a mismatch is
  rejected before downstream execution.
- Verification defaults to `standard`. Use `temporal-comparison` for an exact
  latest/current tuple or ordered as-of-latest work across a caller-declared
  set. Search and research then require 1-8 unique `--scope` values and 1-8
  unique canonical HTTPS `--source-url` values. One scope verifies only that
  exact tuple and requires `--as-of`; 2-8 scopes additionally select one unique
  global winner. `--scope` and `--as-of` are rejected in standard mode;
  standard `--source-url` remains an exact-URL output allowlist. Each-source current releases,
  parallel status, and tradeoffs remain standard Research.
  Exit 0 proves every declared scope/value/date tuple against a declared source
  body and proves the unique newest member of that set, not global completeness.
  Standard search stays at two research-tool calls. A primary temporal search
  may use up to eight so unresolved scopes can be discovered and then verified
  one at a time. When its safe hidden inventory is recoverable, each scope gets
  one concurrent run of at most two calls, with at most four scopes in flight
  and the original shared deadline. Temporal research remains one-shot with up
  to four research-tool calls and never uses per-scope recovery.
- Successful stdout is canonical JSON. Diagnostics use stderr. `--json` is an
  accepted explicit compatibility flag on every subcommand.
- Add `-o PATH` to atomically write JSON and keep stdout empty.
- Use query `-` to read up to 100 KiB from stdin for `search` and `research`.
- Antigravity 1.1.10 or newer is required. Before content and `status`, the
  wrapper accepts only bare `X.Y.Z` output from `agy --version`, rejects
  prefixes, release suffixes, and extra lines rather than guessing capabilities,
  compares numeric semantic components, and rejects a bare semantic version
  below 1.1.10 before model discovery. `models` is intentionally a diagnostic command without that
  guard. The wrapper disables print-mode slash/skill expansion so every request
  field remains data. Search and
  research send typed `primary_first`, `complete_requested_scope`, and
  `explicit_source_only` policies.

## Operations

```bash
agy-search status [--json] [-o PATH]
agy-search models [--json] [-o PATH]

agy-search search QUERY [-n N|--max-results N] [--domain DOMAIN]... \
  [--country COUNTRY] [--max-tokens-per-page N] [--scope SCOPE]... \
  [--source-url HTTPS_URL]... [--as-of YYYY-MM-DD] [--json] [-o PATH]

agy-search extract URL... [--query TEXT] [--json] [-o PATH]

agy-search map URL [--limit N] [--instructions TEXT] [--allow-external] [--json] [-o PATH]

agy-search crawl URL [--limit N] [--instructions TEXT] [--allow-external] [--json] [-o PATH]

agy-search research QUERY [--max-sources N] [--domain DOMAIN]... [--scope SCOPE]... \
  [--source-url HTTPS_URL]... [--as-of YYYY-MM-DD] [--json] [-o PATH]
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

`--as-of` is a typed, inclusive temporal cutoff for explicit source
publication/release dates. Use it in temporal Search or Research whenever the
caller supplies an as-of/cutoff, or when the current execution date is explicitly
known for a present-time temporal comparison. Never pass a future date. It cannot
be used for standard mode or to convert a per-source standard Research request
into a global-winner comparison.

For Search and Research, `--domain` is the canonical caller-owned domain-tree
allowlist: it permits exactly that host and its subdomains. `--source-url` is a
canonical exact-URL allowlist. In standard mode it restricts returned and
internal-audit URL membership only; it does not fetch or verify the URL body.
In temporal comparison, every declared exact HTTPS `--source-url` is fetched
and verified, and an exact URL dominates any same-domain path that `--domain`
would otherwise allow. Both flags prove membership, not official, first-party,
or project-maintained ownership. For such hard source-class requests, you MUST
pass only explicitly trusted domains/URLs and preserve the class in the query. If the
exact trust set is unavailable, stop and report mechanical enforcement is
impossible, or perform only user-permitted discovery with clearly unverified
candidates. Antigravity cannot guarantee that no third-party snippet was ever
viewed during search.

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
`last_updated` is an explicit modification/update date. `date` is `null` only
when the source lacks an explicit publication/release date; never infer it from
`last_updated`, execution, crawl, fetch, query, or cutoff time. Standard mode
treats both fields as best-effort source metadata. Temporal comparison verifies
publication dates only, requires `last_updated: null`, and rejects a non-null
update value with exit 6.

Search and research schemas also require an internal evidence audit with at
least one candidate and one candidate per requested scope. The wrapper validates
that every returned public source is represented in the audit, then omits the
audit from canonical CLI JSON so the public response shape remains stable.
Temporal comparison dynamically restricts audit labels/count to the exact
caller set and URLs to the caller allowlist. It fetches each unique HTTPS source
once with public-address DNS validation/pinning, no ambient proxy/config/cookies,
no redirects, a bounded body, UTF-8 validation, and the shared deadline. Every
candidate's scope, exact compared value, source date text, and normalized date
must bind in one deterministic source section. Temporal search
returns exactly one public unique-latest winner and may perform one bounded,
all-or-nothing recovery across the caller-owned scopes. With a cutoff, strong
caller-owned first-row facts can replace a wrong model value/date only if every
declared scope has a fact at or before it; otherwise the full scoped fallback
runs. Without a cutoff, primary-value anchoring remains. Temporal research keeps
its multiple public sources, requires every candidate's value and exact
source-date text in a same-URL source, requires each structured source date to
be ISO and audit-backed, and requires the unique latest candidate to remain
publicly visible; it is one-shot and never recovers or emits a partial report.
Google grounding transport links are resolved with bounded, HTTPS-only curl
arguments before validation; only the direct final URL can reach public JSON.
Each manual redirect hop is parsed, DNS-validated, and pinned before the next
request.

Preserve a hard caller constraint such as `only official`, `only first-party`,
or `only project-maintained` in the query. Do not infer ownership from a domain
or name; reject a source whose ownership is not established. Label a conclusion
that goes beyond returned source text as `Inference:` with its supporting facts.

## Updates

Update shell-installer releases by rerunning the same installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy --version
agy-search status
```

It verifies the selected application's same-release archive checksum before
replacement; that is integrity checking, not an independent signature or
provenance proof. It installs no second updater executable. Source and
package-manager installations update through their original channel. Never run
an update implicitly during research.

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
| 6 | Do not use partial output or repeat an identical request. Correct a demonstrably wrong declared scope/source once; otherwise report unverified. |
| 7 | Re-run `agy-search models` and select a returned slug. |
