# Performance

`agy-search` keeps its own work small and treats Antigravity as a separate
latency boundary. Measurements below were taken on the same Apple Silicon host
on 2026-08-03 with Hyperfine 1.20.0, Antigravity 1.1.10, and Tavily CLI 0.1.4.

## Budget

| Surface | Budget |
|---|---:|
| Distribution binary on Linux x86-64 CI | < 1.20 MiB |
| Rust CLI fresh-process startup on the measured host | < 5 ms |
| Deterministic one-process search | < 30 ms |
| Captured stdout or stderr per Antigravity process | <= 16 MiB |

Shared CI runners are unsuitable for millisecond latency assertions, so CI
enforces the artifact-size budget. The deterministic benchmark is run locally
when changing process, parsing, schema, or output paths.

## Measured results

| Measurement | Before | Optimized | Result |
|---|---:|---:|---:|
| macOS ARM64 distribution binary | 1,280,976 B | 888,352 B | -30.7% |
| macOS ARM64 archive | 438,720 B | ~365.7 kB | -16.6% |
| Fresh-process `--version`, 200 runs | 1.60 ms | 1.49 ms | no regression |
| Fake one-process search, 30 runs | 22.87 ms | 23.22 ms | within noise |
| Tavily CLI 0.1.4 fresh-process `--version` | 82.91 ms | n/a | reference only |

The local Tavily uv environment occupies 19 MiB. The comparison is limited to
CLI startup and installation footprint; Tavily API latency and Antigravity
model latency are different downstream systems and must not be compared as if
they were wrapper overhead.

Real discovery isolated the actual external bottleneck:

| Command | Mean, 7 runs |
|---|---:|
| `agy --version` | 40.8 ms |
| `agy models` | 3.116 s |
| `agy-search status` | 2.956 s |

`status` remains fresh because it proves current authentication and model
discovery. Caching that answer would make the command fast but untrustworthy.
Likewise, `--model` performs a fresh discovery before content execution to keep
unknown-model failures deterministic. Normal agent calls should omit it unless
model reproducibility is required.

## Reproduce

Build the exact public distribution profile:

```bash
cargo build --profile dist --locked
stat -f '%z bytes' target/dist/agy-search
```

Measure startup and the deterministic Antigravity fixture:

```bash
hyperfine --shell=none --warmup 20 --runs 200 \
  'target/dist/agy-search --version' \
  'tvly --version'

hyperfine --warmup 5 --runs 30 \
  'target/dist/agy-search --agy-path tests/fixtures/fake_agy.py search fixture -n 1 > /dev/null'
```

Measure discovery without spending content-generation usage:

```bash
hyperfine --warmup 1 --runs 7 \
  'agy --version > /dev/null' \
  'agy models > /dev/null' \
  'agy-search status > /dev/null'
```
