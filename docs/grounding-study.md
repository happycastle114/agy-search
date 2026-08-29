# AGY and OpenCodex grounding study

Reviewed on 2026-08-29 against installed AGY `1.1.22` and OpenCodex commit
`fc4de772b58c13f7b16b5029b1e981d612a5db06`.

## Official AGY contracts used

The implementation was checked against these Google-owned sources:

- [Headless mode](https://antigravity.google/docs/cli/headless/) documents
  `stream-json`, one terminal `result` per turn, `structured_output` under
  `--json-schema`, strict `--model`, `--effort`, custom `--agent`, exit status,
  and persistent `--input-format stream-json` sessions.
- [Permissions](https://antigravity.google/docs/cli/permissions/) documents the
  deny/ask/allow precedence and fine-grained action targets.
- [Agents](https://antigravity.google/docs/cli/commands/agents/) documents local
  custom Markdown agent definitions and specialized tool permissions.
- [Tools and skills](https://antigravity.google/docs/sdk/tools/) identifies
  `search_web` and `read_url_content` as the built-in web evidence tools and
  documents enabled/disabled tool filtering. `agy-search` deliberately exposes
  only `search_web` for discovery and never exposes `read_url_content`.
- [Official CLI repository](https://github.com/google-antigravity/antigravity-cli)
  is the release and installation source.

The installed live catalog exposed all three expected slugs:
`gemini-3.7-flash-low`, `gemini-3.7-flash-medium`, and
`gemini-3.7-flash-high`. Core model discovery therefore negotiates rather than
hard-codes availability, while an explicit caller model remains strict.

The official persistent stdin mode was evaluated and deliberately not used in
the 0.3.0 request path. Search, Research, Map, and Crawl each need one isolated
schema, operation-specific least-privilege agent, source policy, deadline, and
tool budget. Multi-URL Extract also isolates each page to prevent a later turn
from broadening earlier source context. Reusing a warmed conversation would
save process startup but would mix request state and weaken those boundaries.
The documented persistent mode remains a future option only for a pool that can
prove per-request agent/schema reset, cancellation, and zero cross-turn state.

## OpenCodex comparison

The repository was pulled before inspection and was clean at the SHA above.
The relevant design is a bounded sidecar loop, not a claim that AGY Search must
copy OpenCodex internals:

- `src/web-search/gemini-executor.ts:30-140` sends one Antigravity Cloud Code
  Assist request with `tools: [{google_search:{}}]`, pins the credential-bearing
  destination to the provider registry, disables automatic redirects, bounds
  response bytes/time, and extracts deduplicated `groundingMetadata` URLs.
- `src/web-search/backends.ts:33-103` offers a sidecar only when its backend is
  active and can execute the selected model. Gemini requires usable stored
  Antigravity OAuth plus a project id. GPT/OpenAI is a separate eligible lane;
  it is not an unbounded fallback for every request.
- `src/web-search/loop.ts:638-769` caps search count, rejects repeated failed
  queries, runs batch members sequentially, preserves tool-call/result pairing,
  degrades executor failures to explicit error results, and deduplicates source
  URLs before exposing them.
- `src/sidecar/candidates.ts:29-61` limits sidecar candidates to picker-visible
  rows or authenticated fixed slots and degrades catalog failures to those
  slots rather than the whole model catalog.

agy-search adopts the reusable invariants—catalog-gated models, bounded calls,
least privilege, explicit failure, deduplication, terminal URLs, and local
verification—but keeps the official AGY CLI as the auth/tool transport. It does
not depend on Exa and does not forward user credentials to a private grounding
endpoint. Unlike a synthesis-only sidecar, it re-fetches terminal publisher
pages with public-DNS pinning and replaces model prose with same-URL body
evidence before publication.

## Resulting production decision

OpenCodex confirms that a small grounded sidecar can make ordinary agent web
search usable, but its grounding metadata alone is not sufficient for the
requested no-hallucination contract. The 0.3.0 boundary therefore treats AGY as
discovery/reasoning and the local verifier as publication authority. Exact URL
Search, Research, and Extract are prefetched through the wrapper's
public-DNS-pinned transport and run AGY with `tools: []`; unrestricted/domain
discovery, Map, and Crawl can expose only bounded `search_web`. Ordinary Search
tolerates an independently unreadable sibling page but publishes no unverified
row, while Research remains all-or-nothing.
