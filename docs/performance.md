# Performance

`agy-search` keeps wrapper work small and treats Antigravity and its web tools
as an external latency boundary. The local measurements below are from the
Apple Silicon development host. They are observations, not service level
objectives or a guarantee for another machine, account, model, or provider state.

## Version 0.4.0: runtime boundaries

The server is an optional Cargo feature and a separate binary. It does not add
the HTTP/MCP dependency graph to the default `agy-search` CLI binary. The server
shares the CLI's source-validation and process boundaries, but it adds a bounded
request semaphore, HTTP/MCP parsing, and structured diagnostics. Measure its
latency separately from the standalone CLI.

On the Apple Silicon development host, with AGY 1.2.7 and three `hyperfine`
runs on 2026-09-20, a local `agy --version` averaged **51.1 ms** and `agy
models` averaged **2.595 s** (2.406–2.897 s). These are catalog/process
observations, not a request latency SLO and not a model-quality comparison.

The server caches only a successful advisory, unpinned catalog lookup for the
same executable and working directory. Its default TTL is 60 seconds; explicit
model pins, status/readiness, version preflight, failures, and empty catalogs
remain fresh. This avoids repeatedly paying the measured catalog boundary for
eligible requests without hiding a caller-selected model or an availability
check. Provider inference, web-tool work, and publisher fetches remain the
dominant variable latency sources.

A controlled wrapper benchmark used the debug HTTP binary, public fixture
responses, `curl`, ten measured runs after two warmups per condition, and an
artificial 200 ms `agy models` delay. With `AGY_SEARCH_CATALOG_TTL_SECONDS=0`,
the mean was **455.8 ± 42.8 ms**. With the default 60-second TTL and a warm
catalog, it was **168.8 ± 3.2 ms**: **2.70 ± 0.26×** faster, or a **63.0%**
decrease. The test servers bound to ports 18094 and 18095 and were stopped by
their cleanup trap.

This isolates wrapper-side catalog reuse only. Its deliberate fixture delay
does not predict real AGY, model, publisher, or network latency. The separate
real `agy models` baseline above remains 2.595 seconds on the measured host;
neither result is projected as a production latency promise. Reproduce the
controlled case with the recorded benchmark script and JSON/log artifacts from
the release evidence, keeping the binary, fixture, warmup count, curl request,
and TTL conditions unchanged.

Use a guarded before/after method when changing this path:

```bash
hyperfine --runs 3 'agy --version' 'agy models'
AGY_SEARCH_CATALOG_TTL_SECONDS=0 ./target/debug/agy-search-server --help
AGY_SEARCH_CATALOG_TTL_SECONDS=60 ./target/debug/agy-search-server --help
```

The last two commands only confirm configuration parsing. A meaningful runtime
comparison must use the same authenticated AGY account, query, source policy,
deadline, and network conditions, and must report catalog behavior separately
from variable upstream latency.

### Runtime observations

Real AGY 1.2.7 raw HTTP requests for an exact-IANA query returned HTTP 200 in
32.8375 seconds and 8.4382 seconds. Those two requests were not an isolated
cache experiment and cannot establish a warm-cache effect or a median.

Two native LiteLLM domain-scoped attempts returned HTTP 504 and HTTP 502. A
subsequent known-good IANA query returned HTTP 200. That final response proves
one end-to-end compatibility case only; it is not evidence of reliable provider
availability or a latency claim. In separate MCP checks, the official Python
SDK 2.2.0 completed HTTP Search with default modern negotiation and stdio
Extract with legacy negotiation against real AGY, listed the five published
tools, and received `is_error: false` responses. Both transports also passed
default-client tool discovery.

## Version 0.3.1 (September 2026)

Version 0.3.1 negotiates Gemini 3.8 Flash for primary and recovery
efforts. Antigravity 1.1.26 advertised all three exact slugs on 2026-09-05.
Explicit pins remain strict; an unavailable preferred model still uses the
provider default after the bounded advisory lookup. Historical 3.7 policy and
measurements below describe the released 0.3.0 build, not this source update.

Source-body fetching now refills each of its four worker slots as it completes,
instead of waiting for the slowest member of each four-page batch. This keeps
input order, per-page errors, DNS pinning, and the shared deadline. It benefits
requests with more than four sources and uneven response times; it does not
remove model inference or publisher latency.

The skill's Quick and Verified paths now consume independently fetched Search
body context directly when it proves all requested fields. Extract remains the
follow-up for missing evidence. This removes one redundant model call only
when the returned body context is sufficient; a valid JSON response alone does
not prove answer completeness.

### Isolated model comparison

On 2026-09-05, the installed 0.3.0 binary ran three serial trials per explicit
low-effort model, alternating 3.7 then 3.8. The binary was unchanged throughout
this comparison, so these results isolate model selection from the source
fetch scheduler changes. The shared host was also compiling the candidate.

```bash
agy-search --model MODEL --effort low --timeout 75 search \
  'According to IANA, what are example.com and example.org reserved for, and can they be registered or transferred?' \
  --domain iana.org -n 3
```

The independent [IANA oracle](https://www.iana.org/help/example-domains)
requires both documentation/example use and non-registration/non-transfer in
the public body context, canonical IANA URLs, and no invented dates.

| Model | Wall seconds, trials 1 / 2 / 3 | CLI successes | Complete answers |
|---|---|---:|---:|
| Gemini 3.7 Flash low | 65.10 / 20.83 / 17.71 | 2/3 | 0/3 |
| Gemini 3.8 Flash low | 29.55 / 52.70 / 15.79 | 3/3 | 2/3 |

The first 3.7 attempt failed closed with `agy output invalid`. Its other two
responses omitted registration/transfer. The second 3.8 response also omitted
that field. The 3.8 successful median was 29.55 seconds; the 3.7 successful
median was 19.27 seconds, but neither successful 3.7 response passed the full
question oracle. This small sample does not establish a model speedup or an
accuracy rate for general search. It does demonstrate why the skill must
extract missing fields instead of treating every successful search as a
complete answer.

The source update also retains distinct, independently verified contexts from
the same page instead of publishing only its first audit context. Additional
contexts are appended whole within the existing 480-character projection
budget; duplicate or unverified contexts add nothing. This fixes a deterministic
projection loss. The comparison above did not capture private model audits, so
it does not establish that this loss caused the observed incomplete answers.

### Final source-build smoke

The first source candidate still omitted one field on both the IANA query
(18.70 seconds) and a Korean query asking for the Gemini 3.8 Flash model ID and
all supported thinking levels (20.08 seconds). Standard Search instructions
were then strengthened to check each requested field, retain distinct audit
candidates even for a shared URL, and spend an unused restricted-search slot
on a missing fact without increasing the two-call budget.

The final build ran those same two queries once each, serially, with no explicit
model pin. A local adapter recorded only the selected model argument: both
primary calls selected `gemini-3.8-flash-low`, with no recovery invocation.

| Query | Wall time | Requested evidence in public output |
|---|---:|---|
| IANA purpose plus registration/transfer | 22.70 s | Both facts, canonical IANA pages, null dates |
| Korean Gemini model ID plus thinking levels | 20.71 s | `gemini-3.8-flash` and `low, medium, high`, official Google pages, null dates |

These are two passing smoke observations, not a statistical estimate of the
prompt change's effect. The model comparison still rules out a blanket speedup
claim. One separate exact-IANA Extract took 16.50 seconds and returned both
facts, demonstrating the available follow-up when Search context is incomplete.
The final complete Search responses can avoid that extra model call under the
updated skill. The final source changes preserve the existing source-verification
boundaries; they do not guarantee completeness for arbitrary questions.

### Validation and local installation

`cargo fmt --all -- --check`, locked Clippy across all targets/features with
warnings denied, the distribution build, and the locked test suite passed:
294 tests passed and 8 opt-in live tests remained ignored. The manual live
search/extraction observations above were run separately. The OpenCode plugin
suite passed 11 tests. The LSP daemon was unresponsive; compiler/Clippy checks
provide the Rust diagnostic evidence for this run.

The macOS ARM64 distribution binary is 1,205,360 bytes, SHA-256
`fe7c65a3755b594cc49e83851d9704e1544df8aa793801ccec90d030c1c98179`.
It was installed locally after preserving the old binary. This was the prerelease
source build retaining the 0.3.0 package version; release artifacts use 0.3.1.
The OpenCode skill and dependency rule were synchronized through a separate
local development package, preserving the original release directory.
OpenCode 1.18.29's `debug skill` resolved the updated development skill;
CLI `status` confirmed Antigravity 1.1.26 with 14 available catalog entries.
A fresh search through the installed executable used the exact IANA source,
returned the requested documentation-purpose evidence in 11.64 seconds, and
again selected `gemini-3.8-flash-low` without a caller model pin.
Long-running OpenCode sessions need a restart to load the updated guidance.

## Budget

| Surface | Budget |
|---|---:|
| Distribution binary on Linux x86-64 CI | < 1.60 MiB |
| Local latency | No portable CI gate; report the measured guarded path below |
| Captured stdout or stderr per Antigravity process | <= 16 MiB |

Shared CI runners are unsuitable for millisecond latency assertions, so CI
enforces the artifact-size budget. Run the deterministic benchmark locally when
changing process, parsing, schema, or output paths.

## Measured wrapper results

| Measurement | Result |
|---|---:|
| macOS ARM64 0.2.4 distribution binary | 1,072,064 B |
| Linux x86-64 0.2.4 CI distribution binary | 1,395,344 B |
| macOS ARM64 0.3.0 final candidate | 1,205,360 B |
| Linux x86-64 0.3.0 CI candidate | 1,582,224 B |
| Historical pre-floor wrapper-only startup measurement | 2.0 ms mean |
| Real `agy --version`, 10 runs | 41.0 ms mean, 38.9-44.2 ms range |
| Fake content including `agy --version` guard, 10 runs | 47.1 ms mean, 46.1-50.1 ms range |

### Release-size attribution

The 0.2.3 release binaries measured 888,352 B on macOS ARM64 and 1,123,752 B
on Linux x86-64. Version 0.2.4 is larger by 183,712 B (20.68%) and 271,592 B
(24.17%), respectively. This is an explicitly accepted cost of the new bounded
source fetching and DNS pinning, HTML source parsing, source/date verification,
tool-policy enforcement, and request-constrained schemas rather than a
dependency upgrade: the dependency and feature graphs are unchanged from
0.2.3.

A same-toolchain macOS `cargo bloat --release --crates` comparison attributed
the 148.1 KiB `.text` increase primarily to `agy_search` (+93.4 KiB), Tokio
(+33.8 KiB), and the standard library (+15.8 KiB). The current release profile
remained the smallest safe measured configuration. Relative to its 1,072,064 B
baseline, `opt-level=s` added 195,280 B, thin LTO added 567,712 B, 16 codegen
units added 33,552 B, retaining symbols added 906,744 B, and unwind panics added
199,200 B. Version 0.3.0 adds 186,880 B (13.39%) on Linux for Gemini 3.7 model
negotiation, same-body Search/Research/Extract projection, and the reusable
production gates. The 1.60 MiB Linux gate leaves 95,498 B (6.04%) above the
observed candidate while continuing to fail meaningful future growth.

Every content command and `status` pays one uncached local `agy --version`
preflight before model discovery or `-p`; this measurement is the full process
cost, not an inferred incremental estimate. It is cheap relative to the
observed multi-second live model and web-tool work, but it is intentionally not
hidden by a cache.

The 2.0 ms result predates the 1.1.10 version-floor preflight and measures only
the old wrapper path. It is retained as historical context, not a current
guarded-content performance claim; do not compare it to the deterministic
47.1 ms path.

The release installer adds no standalone updater binary. It reruns the same
checksum-verified release archive, so there is no separate updater footprint to
measure.

## Live observations

These wall times include external model and web-tool work. They measure a
particular live execution, not Rust wrapper overhead or a latency promise.

| Scenario | Observed wall time | Outcome |
|---|---:|---|
| Quick standard search S1, median | 20.183 s | Returned validated JSON |
| Quick standard search X1, median | 16.104 s | Returned validated JSON |
| Final two-case official-source Quick median | 15.873 s | Explicit-date and legitimate-null-date oracles both passed |
| Final X1 official-only Quick search | 17.26 s | Returned validated JSON |
| T1 first-party synthesis | 34.77 s | Returned validated JSON |
| Final C1 official-only synthesis with labeled inference | 61.24 s | Returned validated JSON |
| Temporal v26 comparison | 40.51 s | Returned validated JSON |
| Final one-scope exact-source temporal search | 29.05 s | Corrected to CLI 1.1.10 / 2026-08-03 from the verified source body |

### Light-search model comparison

On 2026-08-06, the installed 0.2.5 CLI and Antigravity CLI 1.1.10 ran one
stable IANA lookup three times per arm in rotated serial order. Every attempt
recorded latency when it exited successfully and emitted one successful terminal
event. An oracle pass additionally required null unlabeled dates, an IANA source,
and a public answer covering both reservation and registration.

| Arm | Oracle passes | Median | Maximum (3 runs) |
|---|---:|---:|---:|
| `gemini-3.6-flash-low` | 3/3 | 20.27 s | 21.06 s |
| `gemini-3.5-flash-low` | 3/3 | 24.51 s | 25.83 s |
| Implicit low-effort default | 2/3 | 21.72 s | 22.48 s |
| Flash Lite | Not run | — | — |

`gemini-3.6-flash-low` was the fastest eligible arm: its median was 4.24
seconds (17.3%) below `gemini-3.5-flash-low`. The implicit arm was not eligible
because one public result omitted the requested registration answer, regardless
of its latency. Antigravity exposed no Flash Lite slug through `agy models`, so
the benchmark did not invent one or substitute another model.

An explicit model pays dynamic discovery before content: five local runs put
`agy models` at 3.020 seconds mean, versus 45.8 milliseconds for `agy
--version`. The 3.6 arm still had the lowest eligible end-to-end median. The
remaining common post-model time includes fail-closed Standard Search
terminal-URL validation. It replaces Google transports, probes direct URLs, and
removes dead or unsafe rows through the same DNS-pinned path. HEAD-rejecting
publishers receive one range-requested GET capped at 2 MiB; do not remove that
source-quality and SSRF boundary to improve a benchmark number.

For ordinary low-effort standard Search, the wrapper now makes one advisory
`agy models` query, bounded to five seconds inside the existing caller deadline,
and adds `--model gemini-3.7-flash-low` only when that exact catalog entry is
returned. If it is absent or advisory discovery fails while time remains, the
content process omits `--model` and uses the provider default. Explicit pins
remain strict and run fresh full-deadline discovery; temporal/research/site
operations and medium/high Search now use the same bounded negotiation for the
matching Gemini 3.7 Flash effort. Re-run this
comparison before changing the preference because provider behavior and the
runtime catalog can change independently of this wrapper.

### Gemini 3.7 Flash policy measurement

Early on 2026-08-29, before same-URL body projection was enabled, Antigravity
CLI 1.1.22 and the 0.3.0 candidate ran the same IANA
lookup three times per low-effort arm. Gemini 3.7 Flash completed in 27.58,
17.61, and 18.04 seconds (18.04-second median). Gemini 3.6 Flash completed in
13.17, 14.23, and 17.56 seconds (14.23-second median). All six runs returned an
IANA URL, but one 3.7 response also invented an unrelated `example.edu` row and
therefore failed the semantic quality oracle. This pre-verification measurement
is retained as provider behavior evidence; it is not the 0.3.0 routing policy.

Version 0.3.0 negotiates Gemini 3.7 Flash for every Search effort, temporal
Search, Research, Extract, Map, and Crawl. Standard Search then fetches the
terminal publisher page, removes candidates whose values do not bind to that
same page, replaces model titles with fetched document titles, and publishes
locally sliced body context. That deterministic boundary rejects the invented
row class instead of relying on model prose. Exact-URL Research now prefetches
bounded query-relevant source windows for the primary and recovery attempts and
reuses that immutable body snapshot for final binding. Extract likewise
replaces model content with query-relevant windows from the independently
fetched exact page, preventing nested JSON or hallucinated prose from being
published. The production gate in
`tests/production_quality_gate.rs` checks known-page extraction and multi-source official Google research,
local same-URL exact-value proof plus locally projected body context for every
Research audit candidate, projected finding/report summaries, and a second
extraction of every retained research source. Its four independent
claims must resolve to the corresponding current Google AI overview, model,
thinking, and Search-grounding documentation pages. A release candidate does
not pass when any source is a bare origin, search wrapper, unretained citation,
unread Research source, body-mismatched exact value, omitted requested claim,
or source body missing the requested markers.

### Current-affairs and research release gates

On 2026-08-29, the final 0.3.0 candidate passed three consecutive runs of the
opt-in current-affairs gate against three independently established
official-source oracles: nine of nine searches passed. Each result had to match
a known exact evidence URL (query-parameter order ignored), expose only
source-backed date metadata, and contain the listed same-page facts.

| Oracle | Three AGY Search runs | Median | Exact official source outcome |
|---|---:|---:|---|
| Bank of Korea policy rate | 18.32 / 17.51 / 28.08 s | 18.32 s | 3.00 and prior 2.75 on the official 2026-08-27 decision page |
| Latest stable Rust release | 19.18 / 74.96 / 16.13 s | 19.18 s | Rust 1.98.0, published 2026-08-20 |
| Latest WHO item as of 2026-08-29 | 18.77 / 20.18 / 16.08 s | 18.77 s | 2026-08-28 Ebola Bundibugyo IHR Emergency Committee meeting report |

The median across all nine AGY samples was 18.77 seconds; the slowest was a
74.96-second successful Rust recovery. Exact terminal-page hit rate, factual
oracle pass rate, and reachable-link rate were each 9/9. No bare origin,
search-result URL, unsupported date, or dead public link passed.

The same three queries were issued once as one batch through the independent
browser search tool used during development. It returned in 2.1 seconds and
contained the correct dated facts for all three queries. It selected the exact
deep Bank of Korea and Rust pages, but selected WHO's current Ebola situation
listing rather than the requested terminal meeting-report page: exact
terminal-page hit rate 2/3, factual marker coverage 3/3. This is a single
interactive comparison, not a benchmark of that external service. It confirms
the tradeoff visible in this candidate: ordinary browser search is much faster,
while AGY Search adds bounded model work plus independent terminal-page fetch,
DNS/link validation, and same-URL evidence projection. The release claim is
therefore production-grade verified search, not latency parity with a hosted
search index.

The final body-projected candidate passed the separate production Research gate
in 169.76 seconds: known IANA extraction took 27.95 seconds, four-source Gemini
3.7 research took 43.34 seconds, and independent re-extraction of all four
retained sources took 98.46 seconds total. The local verifier also requires each
finding to repeat an exact value that was independently bound to the body of the
same cited URL; a correct claim attached to a different official page fails closed.
These are finite live observations, not an
availability or latency SLO. The direct official links found through the
external comparison search and through AGY were equivalent at the oracle level;
the external comparison search was generally faster at discovery, while AGY's
release gate added local body binding and fail-closed publication checks.

The authenticated AGY 1.1.22 compatibility gate also passed all five public
operations on the final candidate: Search 19.65 seconds, Extract 17.75 seconds,
Map 16.12 seconds, Crawl 17.07 seconds, and Research 17.93 seconds (94.58 seconds
total). Search and Research used one exact IANA source for deterministic
operation compatibility; the independent current-affairs and four-source
production gates above retain the open-web and multi-source quality burden.

Multi-URL Extract is split into at most four concurrent isolated single-URL AGY
runs and merged in caller order. Each result's public content is projected from
its independently fetched exact page rather than model prose. This avoids one
long conversation broadening its local-tool behavior after several page reads,
while retaining all-or-nothing output and the original shared deadline.
Standard Search verifies retained pages independently, publishes only candidates
whose own HTML bodies remain readable and support the declared value, and fails
when no page survives. A deadline still aborts the whole request. This prevents
one unreadable PDF or sibling page from discarding an otherwise proven HTML
result. It also discards a run containing a balanced failed web-tool attempt and
spends a configured recovery tier; the mixed-error result itself is never
published.

The 0.2.9 correction keeps the low fast path and diversifies bounded recovery
through the discovered medium and high tiers. Its exact `한국 오늘 증시` serial
regression exited 0 in all five runs at 18.95, 55.82, 26.67, 58.03, and 21.15
seconds (26.67-second median). Three runs used only low; two used low then
medium. Every public URL was a terminal non-Google HTTPS publisher page. This
also covered publishers that returned 404 to HEAD but 200 to the bounded GET
fallback. As with the earlier benchmark, this is finite incident evidence, not
an availability or latency SLO.

The final 0.2.6 Korean-market regression then ran the exact broad query `한국 오늘
증시` five times serially through that default path after direct-source terminal
validation was enabled. All five commands exited 0. Wall times were 16.891,
33.728, 16.853, 23.730, and 30.933 seconds, for a 23.730-second median versus the
prior 26.990-second incident baseline. The nine returned URLs were non-Google
terminal HTTPS responses returning HTTP 200 with no redirect, and every final
date was exactly same-URL source-bound or `null`. This is a finite incident
regression, not an availability or latency SLO.

Provider variability is material. Earlier attempts in the same evidence series
failed closed for model/effort mismatch, source-class violations, insufficiently
labeled inference, or temporal evidence rejection; a failed attempt is not a
successful latency sample. The checks establish evidence and safety behavior,
not accuracy, availability, provider reliability, or latency guarantees.

Accuracy and latency are separate measurements. A source-constrained or temporal
request can take longer because it verifies declared sources and dates; a faster
response is not more accurate. Conversely, a validated result proves only its
declared evidence contract, not universal source truth.

## Reproduce

Build the exact public distribution profile:

```bash
cargo build --profile dist --locked
stat -f '%z bytes' target/dist/agy-search
```

Measure startup and the deterministic Antigravity fixture:

```bash
hyperfine --shell=none --warmup 20 --runs 200 \
  'target/dist/agy-search --version'

hyperfine --shell=none --warmup 3 --runs 10 \
  --command-name 'real agy --version' 'agy --version' \
  --command-name 'fake content with version guard' \
  'target/dist/agy-search --agy-path tests/fixtures/fake_agy.py search fixture -n 1'
```

Run live commands only when an authenticated Antigravity account and the
associated usage are in scope. Capture the command, model, effort, output, exit
code, and wall time, then distinguish successful observations from fail-closed
attempts before reporting any median or final timing.

Run the reusable production quality gate with the normal unpinned model policy:

```bash
AGY_SEARCH_AGY_PATH=/absolute/path/to/agy \
cargo test --test production_quality_gate --locked -- --ignored --nocapture
```
