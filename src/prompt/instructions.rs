//! Stable operation-specific prompt instruction selection.

use crate::types::{Operation, VerificationMode};

pub(super) const fn tool_instruction(
    operation: Operation,
    verification: VerificationMode,
) -> &'static str {
    match (operation, verification) {
        (Operation::Search, VerificationMode::Standard) => {
            "Begin immediately with search_web; perform no preparatory tool discovery. Use exactly \
             one search_web call and do not use any other tool. Start its query with \
             INPUT_JSON.query byte-for-byte. For an unrestricted search, append a short \
             query-language phrase meaning original evidence article (for Korean use exactly \
             ` 원문 기사`; for English use exactly ` original source article`), followed by \
             ` -site:google.com -site:google.co.kr -site:v.daum.net -site:n.news.naver.com \
             -site:news.nate.com`. A restricted search must keep its caller-owned site expression \
             instead. For unrestricted input, set every URL field to an exact \
             vertexaisearch.cloud.google.com/grounding-api-redirect URL copied from the completed \
             tool result, never to a publisher URL; the wrapper resolves it. Reject placeholders, \
             publisher slugs, and any token that was not copied verbatim. When max_results is \
             at least two, return at least two distinct result items and audit candidates copied \
             from different completed search results. Use distinct grounding transports so one \
             dead publisher redirect can be discarded without another model call. Return after \
             that call when the direct-publisher or primary-source snippets prove the answer."
        }
        (Operation::Search, VerificationMode::TemporalComparison) => {
            "Use this bounded sequence: (1) search_web with the exact scoped query; (2) when \
             source_restriction.urls contains an exact source, read the relevant literal member and \
             never its search-result grounding transport. Otherwise confirm the selected canonical \
             page covers the whole requested comparison scope. A winner-specific \
             release file, README, download page, or single-product changelog is not the comparison \
             page. If call 1 lacks a scope-wide page, use one focused search for the named official \
             changelog plus `all product tracks tabs`; (3) read_url_content exactly once on that \
             scope-wide comparison page. From the fetched page enumerate \
             every named tab, track, category, and dated candidate in scope. When the artifact is \
             raw HTML, inspect only that artifact once. Scope names must be the exact visible tab or \
             button labels, never invented aliases. When buttons use data-tab=KEY and content uses \
             data-list-panel=KEY, pair each label only with the first release row in its matching \
             panel; take that row's version-link and adjacent explicit date text, never a later row \
             or a value from another panel. Do not repeatedly view or grep it. Preserve every exact \
             tab label as an unresolved scope until its own value and date are grounded. Do not \
             spend a follow-up on a scope already grounded by the canonical page. For each remaining \
             scope, search separately with `\"EXACT_SCOPE\" latest version release date \
             site:OFFICIAL_HOST`. If that call finds a value but omits its date or uses a non-official \
             source, make one final exact-value call: `\"EXACT_SCOPE\" \"EXACT_VALUE\" release date \
             site:OFFICIAL_HOST`. Never combine scopes, borrow a value from another panel, or \
             repeat/prepend call 1. Stop as soon as every scope is verified and never exceed eight \
             attempted research-tool calls total. Do not read a redirect and then read its direct \
             target again; one completed read is enough. Wait for every call to finish."
        }
        (Operation::Research, VerificationMode::Standard) => {
            "When INPUT_JSON.source_restriction.urls supplies the complete exact evidence set, \
             read each literal member directly and do not search for it. Otherwise use search_web \
             for the independent sources needed by the synthesis. Honor exactly \
             INPUT_JSON.tool_call_budget as the maximum number of attempted research-tool calls. \
             Do not prepend a separate discovery phase."
        }
        (Operation::Research, VerificationMode::TemporalComparison) => {
            "When INPUT_JSON.source_restriction.urls supplies the complete exact evidence set, \
             read each literal member directly and do not search for it. Otherwise use search_web \
             to inventory every compared scope, then verify canonical evidence pages with \
             read_url_content. Honor exactly INPUT_JSON.tool_call_budget as the maximum number of \
             attempted research-tool calls and wait for each."
        }
        (Operation::Extract | Operation::Crawl, _) => {
            "Use only read_url_content and wait for it to complete."
        }
        (Operation::Map, _) => {
            "Use only search_web or read_url_content and wait for it to complete."
        }
    }
}

pub(super) const fn wire_instruction(operation: Operation) -> &'static str {
    match operation {
        Operation::Search => {
            "Set object=search. Results keys are only title,url,snippet,date,last_updated. \
             evidence_audit keys are only candidates,coverage_complete,conclusion; candidate keys \
             are only scope,claim,url,date,value,source_date_text,evidence_excerpt. Put versions and track names in \
             title, snippet, claim, or value. Before emitting results, require every results[i].url \
             to equal at least one evidence_audit.candidates[j].url. For every non-null \
             results[i].date, one same-URL candidate must repeat that normalized date, carry the \
             exact source_date_text, and include that text in its contiguous evidence_excerpt; \
             otherwise set results[i].date to null. \
             Never emit aliases such as search_result, source_url, canonical_url, \
             explicit_source_date, version_date, or scopes_checked."
        }
        Operation::Research => {
            "Set object=research. Source keys are only title,url,snippet,date,last_updated; finding \
             keys are only title,summary,citations. evidence_audit keys are only candidates, \
             coverage_complete,conclusion; candidate keys are only scope,claim,url,date,value, \
             source_date_text,evidence_excerpt. Never emit \
             source_url, canonical_url, explicit_source_date, version_date, or scopes_checked."
        }
        Operation::Extract | Operation::Map | Operation::Crawl => {
            "Use only the exact field names and object discriminator defined by the JSON schema."
        }
    }
}

pub(super) const fn verification_instruction(
    operation: Operation,
    verification: VerificationMode,
) -> &'static str {
    match (operation, verification) {
        (_, VerificationMode::Standard)
        | (
            Operation::Extract | Operation::Map | Operation::Crawl,
            VerificationMode::TemporalComparison,
        ) => "",
        (Operation::Search, VerificationMode::TemporalComparison) => {
            "For temporal_comparison, audit every exact caller-named scope. Every candidate \
             needs value set to the exact compared version or value, date normalized to YYYY-MM-DD, \
             source_date_text copied with the source's exact date spelling, and evidence_excerpt set \
             to a short, contiguous source excerpt containing both value and source_date_text. Copy \
             the normalized candidate date into exactly one public result date. For one scope, \
             mechanically verify its exact temporal tuple. For multiple scopes, compare all source \
             dates subject to the requested cutoff and rank the unique newest verified candidate first. Set \
             coverage_complete=true only after every named tab or scope was checked. If any scope, \
             exact version, or date remains missing, set coverage_complete=false and do not claim a \
             global winner."
        }
        (Operation::Research, VerificationMode::TemporalComparison) => {
            "For temporal_comparison Research, audit every exact caller-named scope. Every \
             candidate needs its exact compared value, normalized YYYY-MM-DD date, exact \
             source_date_text, and a contiguous evidence_excerpt containing both. Each candidate's \
             value and source_date_text must appear together in the title or snippet of a public \
             source with the same URL. Every public source date must be an audit-backed YYYY-MM-DD \
             candidate date for that URL. For one scope, mechanically verify its exact temporal tuple. \
             For multiple scopes, the unique latest candidate must be visible in a public source with \
             the same URL, date, and value. One canonical source may prove multiple \
             differently dated candidates. Set coverage_complete=true only after every scope is \
             fully bound. Research is one-shot: do not request recovery or retry."
        }
    }
}
