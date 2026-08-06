# Performance

`agy-search` keeps wrapper work small and treats Antigravity and its web tools
as an external latency boundary. The local measurements below are from the
Apple Silicon development host on 2026-08-06. They are observations, not service
level objectives or a guarantee for another machine, account, model, or provider
state.

## Budget

| Surface | Budget |
|---|---:|
| Distribution binary on Linux x86-64 CI | < 1.40 MiB |
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
199,200 B. The 1.40 MiB Linux gate therefore leaves 72,663 B (5.21%) above the
observed 0.2.4 artifact while continuing to fail meaningful future growth.

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
removes dead or unsafe rows through the same DNS-pinned header-only path; do not
remove that source-quality and SSRF boundary to improve a benchmark number.

For ordinary low-effort standard Search, the wrapper now makes one advisory
`agy models` query, bounded to five seconds inside the existing caller deadline,
and adds `--model gemini-3.6-flash-low` only when that exact catalog entry is
returned. If it is absent or advisory discovery fails while time remains, the
content process omits `--model` and uses the provider default. Explicit pins
remain strict and run fresh full-deadline discovery; temporal/research/site
operations and medium/high Search skip the preference query. Re-run this
comparison before changing the preference because provider behavior and the
runtime catalog can change independently of this wrapper.

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
