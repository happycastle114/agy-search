---
name: agy-search
description: Use the agy-search CLI for source-backed live web search, URL extraction, site mapping, bounded crawling, and multi-source research through Google Antigravity. Trigger when a task needs current or latest web facts, as-of-date verification, primary sources, content from known URLs, documentation discovery, website analysis, comparisons, literature or market research, or a cited report without a separate search API key.
---

# Agy Search

Use the smallest operation that can answer the task, preserve returned URLs, and
escalate only when the evidence is insufficient.

## Preflight

Run only the cheap local checks once before the first research command in the
current agent session. Do not run a separate model-discovery command on the
normal unpinned search path:

```bash
command -v agy-search
command -v agy
command -v curl
agy-search --version
agy --version
```

If `agy-search` is unavailable, stop and report that installation is required.
When installation is in scope, use the release installer documented by the
project:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh | sh
```

Do not silently switch providers. After installation, repeat the cheap
preflight. Run `agy-search status` only to diagnose availability or verify an
install/update, not before each search. Run `agy-search models` once before
explicitly pinning a model; omit `--model` on ordinary searches and never invent
or cache a model slug. On unpinned low-effort Standard Search, the CLI itself
performs one bounded advisory catalog lookup and prefers exact
`gemini-3.6-flash-low` only when currently advertised; it otherwise uses the
provider default. Do not duplicate that lookup in the agent plan. If an
explicitly selected returned slug ends in `-low`, `-medium`, or `-high`, pass the
matching `--effort` value. A mismatched suffix is invalid.

The supported Antigravity floor is 1.1.10. Verify `agy --version` once in this
preflight; the core accepts only bare `X.Y.Z` output and rejects a bare semantic
version below 1.1.10 before content or `status`, without model discovery. It
also rejects prefixes, suffixes, and extra lines rather than guessing. If the user explicitly asks to
update a release-installer copy of `agy-search`, rerun its release installer and
then repeat preflight:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/happycastle114/agy-search/releases/latest/download/agy-search-installer.sh \
  | sh
agy-search --version
agy --version
agy-search status
```

The installer verifies the selected application archive and installs no second
updater executable. Never update either `agy-search` or `agy` implicitly.
Antigravity manages its own background updates during regular runs.

## Choose an operation

| Need | Start with |
|---|---|
| Quick facts or candidate sources | `search` |
| Full content from known URLs | `extract` |
| Discover relevant URLs on one site | `map` |
| Read a bounded set of pages on one site | `crawl` |
| Compare or synthesize across sources | `research` |

Decide in this order: explicit URL or requested site operation first, evidence
topology second, then effort. A supplied canonical URL for a literal or exact
field uses `extract`; one canonical evidence page uses `search`; independent
pages whose claims must be combined use `research`. Freshness, consequence,
ambiguity, and disagreement raise effort within that operation. They do not by
themselves turn a bounded extraction into broad research.

When the user supplies one or more exact canonical URLs and asks only for their
text, current value, exact version, or explicitly printed date, start with one
low-effort `extract URL... --query "exact fields"` call. This rule takes
precedence over the word latest or current. Claim a date only when the extracted
body explicitly labels it; never substitute `last_updated`, the execution date,
or a search snippet. If the body lacks the field, report that it was not present
instead of searching again unless the user asked for broader verification. A
decision that must reconcile that page with independent evidence still uses
`research`.

Choose the operation and depth before the first content call. The depth decision
is binding: Quick and Verified start with `search`; Synthesis and Deep start
with `research` and must not prepend a discovery search. Use `extract` directly
when the task already supplies a known URL. Use `map` before `crawl` only when
the relevant paths on one site are unknown.

## Choose research depth

Evaluate freshness, breadth, consequence, ambiguity, and source quality before
running a command. Choose the highest level triggered by any axis:

| Level | Use when | Start with |
|---|---|---|
| Quick | One stable, low-stakes fact; no comparison; one primary snippet can prove it | `agy-search --effort low --timeout 45 search "query" -n 3` |
| Verified | Latest/current/as-of, exact version or date, ambiguous scope, or several sections on one canonical source | `agy-search --effort low --timeout 75 search "query" -n 3` |
| Synthesis | Evidence must be combined across independent sources, entities, domains, or material claims | `agy-search --effort medium --timeout 120 research "question" --domain trusted.example --max-sources 4` |
| Deep | High-stakes decisions, conflicting evidence, literature/market review, or broad multi-angle analysis | `agy-search --effort high --timeout 180 research "question" --domain vendor.example --domain docs.vendor.example --max-sources 8` |

Issue the table's start command for the selected level. Do not announce one
level and execute another level's command. A Synthesis or Deep task may use an
`extract` follow-up for a returned canonical URL, but repeated search retries do
not substitute for its required research call.

Commit to this bounded call plan before the first content call and do not add a
discovery phase:

- Quick: exactly one `search` and no follow-up content call.
- Verified: one standard `search`; make at most one additional
  `temporal-comparison` search only when the first result supplies the exact
  scope and canonical source URL needed for source-body verification. Never
  repeat the standard search or use `research`.
- Synthesis: exactly one `research`, followed by at most three `extract` calls
  for canonical URLs returned by that research. Never add `search` or a second
  `research`.
- Deep: one `research`; only when its returned evidence names a material thin
  claim or conflict, make one narrower `research`, then at most two `extract`
  calls for returned canonical URLs. Never add `search` or exceed four content
  calls total.

Stop as soon as the selected plan proves the answer. A timeout, malformed
result, or failed call is an error, not permission to repeat or broaden it.

Quick and Verified both start at low effort and use `-n 3`. Their difference is
the evidence contract and timeout, not extra model thinking. Source-body verification,
exact scopes, and canonical URLs provide temporal accuracy. Reserve medium for
Synthesis and high for Deep unless the user explicitly overrides effort.
Ordinary work omits `--model`; only an explicit model pin needs fresh discovery
and its suffix-matched effort.

Routing is deliberately lowest-sufficient-depth: default to Quick for one
stable factual lookup, choose Verified for a factual/current claim that needs a
date, version, or fuller canonical evidence, choose Synthesis only for a real
comparison or combination of independent sources, and choose Deep only for
broad, high-stakes, exhaustive-ambiguity, or materially conflicting work. Do
not reflexively call `research` because the user uses the word "research". If a
higher-level trigger no longer applies after inspecting the task, de-escalate to
the lowest matching level before the first content call. Keep Quick/Verified at
`--effort low` and their small search budget. Bound the selected level from its
first call: Quick 45 seconds, Verified 75, Synthesis 120, and Deep 180.
Synthesis uses `--max-sources 4` and Deep uses `--max-sources 8`. A user may
request a different explicit timeout, but never silently lengthen a timed-out
call or repeat the same broad request.

The CLI derives the internal Research web-tool attempt ceiling as
`min(max_sources + 2, 12)` so discovery and source reads fit the requested
evidence set. Failed and unfinished attempts consume that ceiling. Treat it as
a safety limit, never as a target: do not pad calls merely because budget
remains.

Set `--max-sources` to the smallest independent evidence set that can prove the
material claims. Two linked official pages need two sources, not eight.
Synthesis normally needs 2-4 and Deep normally needs 6-8. Expand the budget once
only when a required claim remains thin or sources conflict; never increase it
merely to make the answer longer.

Judge breadth by the number of independent evidence sources required, not by
the number of bullets in the answer. Comparing several releases or product
tracks on one authoritative changelog remains Verified when that page proves
the complete scope. Escalate it to Synthesis only when the answer must combine
independent canonical pages or sources.

Preserve the user's evidence constraints in the first query. For Verified
temporal or exact-field work, pass the full user question when practical;
otherwise retain every named entity, requested scope, cutoff date, and exact
field. Never shorten it to a generic discovery query such as "official
changelog release notes".

Treat `only official`, `only first-party`, and `only project-maintained` as hard
caller constraints. Keep the exact source-class restriction in the command
query and pass every explicitly trusted domain tree with `--domain DOMAIN` or
every exact trusted page with `--source-url HTTPS_URL`; Search and Research use
both flags as caller-owned allowlists. `--domain` is the canonical domain-tree
allowlist and admits only its host and
subdomains, while standard `--source-url` admits only that exact canonical URL
and does not fetch it. These flags prove membership only, never ownership: do
not infer ownership from a name, URL, domain, or search result. Antigravity
cannot guarantee that it never viewed a third-party snippet during search.

When a Research task already has the complete exact canonical page set, pass
each page with `--source-url` in the first `research` call and skip discovery
search. Direct page reads are the evidence path in that case. Use `--domain`
and search only when exact evidence pages are not yet known.

For a hard source-class request, you MUST pass that explicit trust set. If the exact trusted domain/URL set is unavailable, stop and report that
mechanical enforcement is impossible. Do user-permitted discovery only when it
is explicitly in scope, and label every candidate unverified; never present it
as official, first-party, or project-maintained. When trusted evidence is split
across official domains, pass each known domain or exact URL rather than guessing
one broad domain. If an answer includes a conclusion beyond what a returned
source states, label it `Inference:` and explain its supporting facts.

Use `--verification temporal-comparison` when the question asks for one exact
latest/current source-published tuple from a known scope and canonical page, or
asks to order 2-8 known scopes and return one unique as-of-latest winner. One
scope is temporal source verification, not a comparison or a global inventory
claim. A request for each of several independent sources' current releases,
parallel status, or tradeoffs remains standard `research`; it does not ask for
one global winner.

For a true temporal comparison, pass every exact caller-owned scope plus every
canonical HTTPS source page. A scope label must either be supplied exactly by
the user or be visibly confirmed on a canonical source during the single
bounded discovery step below. Never manufacture a scope label from a domain,
URL, product name, hostname, or model alias. The wrapper proves completeness
only relative to the declared set; never describe an undeclared or discovered
subset as globally exhaustive. Do not add temporal verification to stable Quick
facts or a single-scope fact that does not need an exact source date.

`--as-of YYYY-MM-DD` is a typed, inclusive source-date cutoff for temporal
Search or Research only. Standard mode rejects it. A one-scope temporal lookup
requires this cutoff so local source facts can replace a wrong model value
without trusting that value. When the caller supplies an as-of/cutoff, pass it.
When the current execution date is explicitly known for a present-time lookup
or comparison, pass that date too. Never invent a cutoff or pass a future date.
Do not add `--as-of` to standard per-source research: the date does not change
that routing. Examples:

```bash
agy-search --effort low --timeout 75 --verification temporal-comparison \
  search "$QUESTION" -n 3 \
  --scope "$SCOPE" \
  --source-url "https://vendor.example/changelog" \
  --as-of 2026-08-06

agy-search --effort low --timeout 75 --verification temporal-comparison \
  search "$QUESTION" -n 3 \
  --scope "$SCOPE_A" --scope "$SCOPE_B" \
  --source-url "https://vendor.example/changelog" \
  --as-of 2026-08-06
```

Temporal mode requires 1-8 unique exact `--scope` values and at least one unique
`--source-url`; exactly one scope also requires `--as-of`. Repeat the source flag
when evidence is split across canonical pages. It rejects missing, duplicate,
extra, or model-invented scope labels and checks each scope/value/source-date
tuple against the bounded source body. An exact temporal `--source-url` is
fetched and verified and dominates a same-domain path otherwise admitted by
`--domain`. A successful one-scope search returns that verified candidate;
2-8 scopes return the unique newest declared candidate. Research retains the
required report shape but remains one-shot.

With a cutoff, strongly structured caller-owned first rows may replace a wrong
model value or date only all-or-nothing: every declared scope must have an exact
fact at or before the cutoff. Otherwise the rows are discarded and Search uses
the complete scoped fallback. Without a cutoff, current primary-value anchoring
remains; do not promote a mismatched local row.

If a task otherwise routes to Verified and its exact scope inventory or
canonical source URL is unknown, use the one standard search already declared
by the Verified plan. Only when that response supplies the exact inventory and
canonical URL may its one allowed temporal search verify them. The discovery
result is not itself exhaustive.

Do not add this discovery-then-temporal sequence to Synthesis or Deep. Their
declared Research call must carry any caller-owned temporal scopes and canonical
URLs from the start. If a multi-source exhaustive temporal claim needs an
inventory the caller has not supplied, return a bounded synthesis or stop as
unverified; do not issue an extra temporal call, guess labels, or turn the
model's hidden audit into caller-owned truth.

Stay Quick only when the returned primary-source snippet directly proves the
whole answer. Latest/current/as-of questions are at least Verified. If they span
multiple independent products, jurisdictions, or time periods whose facts live
on different sources, use Synthesis. In either case, list the complete scope,
compare explicit source dates, and do not treat the first tab or result as
globally latest. Treat a user-supplied cutoff date as a constraint, never as
source metadata.

Before the first content call, choose one level higher when the task already
shows that a canonical page may be missing, exact fields require body evidence,
sources conflict, or one bounded result will be insufficient. After the first
call, stay inside the selected level's declared follow-ups; a newly discovered
gap is reported as insufficient unless that bounded plan already permits its
exact follow-up. Prefer extracting a returned canonical URL over broadening the
search. Stop as soon as the selected level proves every material claim; do not
run research merely to improve prose. Manual `--effort` remains an override,
not a substitute for choosing the correct operation.

An exit 6 means the wrapper rejected the evidence, so consume no partial claim.
Temporal `search` already attempts one bounded recovery for every caller-owned
scope and fails all-or-nothing; temporal `research` is one-shot and never
recovers. Never rerun an identical broad command. Correct a demonstrably wrong
scope/source contract once only before restarting an explicitly malformed
caller contract; otherwise report that the comparison is not verified. Choose
Synthesis before the first content call only when independent sources are
genuinely required, not to force a prettier answer.

Do not emulate Tavily-only concepts. `agy-search research` is one-shot: there is
no request ID, polling, credit model, API key, or promise of exhaustive crawling.

## Save context

Use stdin `-` for generated or multiline queries. Save large output under the
task-local `.agy-search/` directory and inspect only needed fields:

```bash
printf '%s\n' "$QUERY" | agy-search search - -o .agy-search/search.json
agy-search extract https://example.com/page -o .agy-search/extract.json
agy-search map https://example.com -o .agy-search/map.json
agy-search crawl https://example.com --limit 20 -o .agy-search/crawl.json
agy-search research "$QUESTION" -o .agy-search/research.json
```

Prefer `jq` or targeted reads over loading an entire crawl into context. Keep
artifacts until the answer is complete so provenance can be checked.

## Validate evidence

- Require exit code 0 and the response `object` matching the command.
- Cite only returned URLs that directly support a claim. Standard Search
  exposes only terminal public HTTPS URLs.
- The CLI validates direct URLs and resolves Google grounding transports before
  Standard Search output; never expose or reconstruct a discarded URL. Each
  redirect hop is independently parsed, DNS-validated, and pinned before it can
  be followed.
- For research, require every cited URL to appear in `sources`.
- Interpret `date` only as an explicit publication/release date and
  `last_updated` only as an explicit modification/update date. In Standard
  Search, `date` is `null` when the source lacks an explicit date or the returned
  same-URL evidence cannot bind its complete source date text. Never replace it
  with an execution, crawl, fetch, query, cutoff, or inferred date. Standard
  Research and malformed metadata remain fail-closed. In
  `temporal-comparison`, the verified audit binds publication dates only, so
  `last_updated` must be `null`; any non-null update value fails closed with exit
  6.
- For multi-scope latest/current/as-of claims, require the exact declared scopes,
  canonical source pages, exact version or value, and explicit source dates.
  Temporal exit 0 proves source-body binding for that declared set, not that the
  source itself is truthful or that an unknown global inventory is complete.
- Prefer primary sources and cross-check consequential or time-sensitive claims.
- State uncertainty when sources disagree or the bounded result is thin.
- Never cite the CLI, a local `.agy-search/` path, or an unreturned URL.

This is the end-to-end accuracy gate: do not report a factual answer until the
chosen command exits 0, its matching public JSON response has been checked, and
each material claim is tied to a returned URL under the selected trust boundary.
The allowlist proves caller-specified membership only, never ownership.

Read [references/commands.md](references/commands.md) for flags, response shapes,
and stable failure handling.
