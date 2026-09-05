# Command reference

## Global contract

Before the first content command in an agent session, require agy-search 0.3.1
or newer and use this cheap local preflight. Do not invoke `agy-search models`
unless an explicit pin is requested:

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
- Discover `SLUG` with `agy-search models` only for an explicit model pin in
  the current environment.
- Content commands default to `--effort low`. Raise effort only for deliberate
  deep synthesis; explicit effort always overrides the default.
- Ordinary work omits both `agy-search models` and `--model`. CLI 0.3.1
  performs a bounded advisory catalog lookup internally and prefers exact
  `gemini-3.8-flash-low` when present without creating a caller model pin.
  Temporal/research/content operations negotiate the matching Gemini 3.8 Flash
  effort when advertised. For an explicit pin whose returned slug ends in
  `-low`, `-medium`, or `-high`, pass
  the matching `--effort`; a mismatch is rejected before downstream execution.
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
  and the original shared deadline. Temporal research remains one-shot with
  `min(2 * max_sources + 1, 12)` attempted research-tool calls and never uses
  per-scope recovery. Exact-source prefetched operations expose no network tools.
- Successful stdout is canonical JSON. Diagnostics use stderr. `--json` is an
  accepted explicit compatibility flag on every subcommand.
- In Standard Search and Standard Research, `date` is optional: normalize a
  valid date that cannot bind to same-URL audit evidence to `null`, while
  rejecting malformed dates. Temporal comparison keeps strict date handling.
- Add `-o PATH` to atomically write JSON and keep stdout empty.
- Use query `-` to read up to 100 KiB from stdin for `search` and `research`.
- Antigravity 1.1.20 or newer is required. Before content and `status`, the
  wrapper accepts only bare `X.Y.Z` output from `agy --version`, rejects
  prefixes, release suffixes, and extra lines rather than guessing capabilities,
  compares numeric semantic components, and rejects a bare semantic version
  below 1.1.20 before model discovery. `models` is intentionally a diagnostic command without that
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

## Depth-routed Research templates

For the skill's Synthesis and Deep levels, effort, timeout, and `--max-sources`
are one atomic routing contract, not suggestions. Keep exactly one of every
required flag on every Research invocation; do not rely on defaults, omit them
when using stdin, or change them for a narrower Deep follow-up. Returning fewer
sources is allowed and preferred when they prove the material claims.

```bash
# depth-budget: synthesis
agy-search --effort medium --timeout 120 research "$QUESTION" --max-sources 4

# depth-budget: deep-primary
agy-search --effort high --timeout 180 research "$QUESTION" --max-sources 8

# depth-budget: deep-follow-up
agy-search --effort high --timeout 180 research "$NARROWER_QUESTION" --max-sources 8
```

Before each Research call, compare its tokens with the matching template and do
not execute until `--effort`, `--timeout`, and `--max-sources` each occur exactly
once with the required value, including stdin-query forms. Then check that the
query preserves the material claim or conflict being investigated.

`--as-of` is a typed, inclusive temporal cutoff for explicit source
publication/release dates. Use it in temporal Search or Research whenever the
caller supplies an as-of/cutoff, or when the current execution date is explicitly
known for a present-time temporal comparison. Never pass a future date. It cannot
be used for standard mode or to convert a per-source standard Research request
into a global-winner comparison.

For Search and Research, `--domain` is the canonical caller-owned domain-tree
allowlist: it permits exactly that host and its subdomains. `--source-url` is a
canonical exact-URL allowlist. In standard Search an exact-URL-only request
disables every AGY network tool; the wrapper independently fetches the supplied
pages and provides bounded evidence for local body projection.
A bare origin is rejected unless it is an exact caller-supplied source URL.
Standard Research uses `search_web` only for discovery, then the wrapper must
independently fetch every retained URL; every finding citation must equal one
retained source URL.
A complete exact URL set is also independently prefetched, supplied as bounded
query-relevant source windows, and reused for same-snapshot body binding.
In temporal comparison, every declared exact HTTPS `--source-url` is fetched
and verified, and an exact URL dominates any same-domain path that `--domain`
would otherwise allow. Both flags prove membership, not official, first-party,
or project-maintained ownership. Preference wording (`prefer`, `prioritize`,
`favor`, or a primary-source preference) affects query prose and result ranking
only; it MUST NOT create either allowlist, even for a named organization or
page. Pass an allowlist only for an explicit exclusive hard constraint or an
explicit caller-supplied trusted domain/URL set. For hard source-class requests,
you MUST pass only explicitly trusted domains/URLs and preserve the class in
the query. If the exact trust set is unavailable, stop and report mechanical
enforcement is impossible, or perform only user-permitted discovery with
clearly unverified candidates. Antigravity cannot guarantee that no third-party
snippet was ever viewed during search.

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

For source metadata, `date` is an explicit publication/release date. In
Standard Search and Standard Research, a valid `date` that cannot bind to
same-URL audit evidence becomes `null`, and a malformed date is rejected. The
current contract always publishes `last_updated: null` because it has no
dedicated modification-date audit binding. Never infer a date from execution,
crawl, fetch, query, or cutoff time. Temporal comparison verifies publication
dates strictly and also requires `last_updated: null`.

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
all-or-nothing recovery across the caller-owned scopes. Exact-URL-only Search
selects unique latest strongly structured first-row facts locally for every
scope; with a cutoff, only rows at or before it are eligible. Ambiguous or
incomplete exact rows fail closed without a model network call. Mixed/domain
discovery retains primary-value anchoring before scoped fallback. Temporal research keeps
its multiple public sources, requires every candidate's value and exact
source-date text in a same-URL source, requires each structured source date to
be ISO and audit-backed, and requires the unique latest candidate to remain
publicly visible; it is one-shot and never recovers or emits a partial report.
Every Standard Search result and audit URL is validated with bounded,
HTTPS-only, DNS-pinned curl arguments. Validation starts with HEAD; a direct
publisher that rejects HEAD receives one range-requested GET capped at 2 MiB
under the same deadline and redirect policy. Google grounding transports are
resolved; dead, unsafe, regional Google search, and cache rows are discarded
with their audit rows. Each redirect hop is parsed, DNS-validated, and pinned
before the next request. When no publishable result survives, Standard Search
uses the available low, medium, and high tiers at most once each under the
original command deadline. Do not add an outer retry for the same broad command.

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

The CLI rejects empty content responses, non-HTTPS URLs, duplicate URLs,
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
