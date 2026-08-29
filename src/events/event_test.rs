use super::*;
use crate::source_restriction::SourceRestriction;
use crate::types::{NonEmptyText, RequiredSearchQuery, ResearchAttemptBudget, ResearchToolBudget};

fn standard_policy() -> ResearchToolPolicy {
    ResearchToolPolicy::Budget(ResearchToolBudget::StandardSearch)
}

fn research_policy() -> ResearchToolPolicy {
    ResearchToolPolicy::Budget(ResearchToolBudget::Research(
        ResearchAttemptBudget::from_max_sources(2),
    ))
}

fn scoped_policy() -> ResearchToolPolicy {
    let query = NonEmptyText::parse("release status").expect("test query must be valid");
    ResearchToolPolicy::ScopedTemporalSearch(RequiredSearchQuery::for_exact_scope(
        &query,
        "alpha",
        &[],
        None,
    ))
}

fn exact_url_research_policy() -> ResearchToolPolicy {
    let restriction = SourceRestriction::parse(
        Vec::new(),
        vec![
            crate::types::HttpUrl::parse("https://doc.rust-lang.org/book/")
                .expect("valid exact test URL"),
        ],
    )
    .expect("valid exact test restriction");
    ResearchToolPolicy::Restricted {
        budget: ResearchToolBudget::PrefetchedEvidence,
        restriction: Box::new(restriction),
    }
}

#[test]
fn restricted_research_accepts_prefetched_evidence_without_a_tool() {
    // Given: an exact caller-owned URL whose body was prefetched by the wrapper.
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"result","result":{"structured_output":{"object":"research","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://doc.rust-lang.org/book/","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"title":"Research","summary":"Evidence","findings":[{"title":"Finding","summary":"Evidence","citations":["https://doc.rust-lang.org/book/"]}],"sources":[{"title":"Source","url":"https://doc.rust-lang.org/book/","snippet":"Evidence"}]}}}"#;

    // When: the Research terminal result is validated.
    let parsed = parse_structured_run(stream, Operation::Research, &exact_url_research_policy());

    // Then: no model-controlled network tool is required for prefetched evidence.
    assert!(parsed.is_ok());
}

#[test]
fn restricted_research_rejects_an_unpaired_exact_url_read_without_search() {
    // Given: an exact caller-owned URL but only an unmatched DONE direct-read event.
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"read_url_content","parameters":{"Url":"https://doc.rust-lang.org/book/"}}}}
{"event":"result","result":{"structured_output":{"object":"research","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://doc.rust-lang.org/book/","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"title":"Research","summary":"Evidence","findings":[{"title":"Finding","summary":"Evidence","citations":["https://doc.rust-lang.org/book/"]}],"sources":[{"title":"Source","url":"https://doc.rust-lang.org/book/","snippet":"Evidence"}]}}}"#;

    // When: the Research terminal result is validated.
    let parsed = parse_structured_run(stream, Operation::Research, &exact_url_research_policy());

    // Then: a direct read is evidence only after its matching ACTIVE event.
    assert!(parsed.is_err());
}

#[test]
fn research_rejects_a_model_controlled_url_read_after_discovery() {
    // Given: discovery followed by a model-controlled URL read.
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"ACTIVE","step_type":"tool","tool_info":{"name":"read_url_content","parameters":{"Url":"https://example.com/first"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"read_url_content","parameters":{"Url":"https://example.com/first"}}}}
{"event":"result","result":{"structured_output":{"object":"research","evidence_audit":{"candidates":[{"scope":"first","claim":"First","url":"https://example.com/first","date":null},{"scope":"second","claim":"Second","url":"https://example.com/second","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"title":"Research","summary":"Evidence","findings":[{"title":"Finding","summary":"Evidence","citations":["https://example.com/first","https://example.com/second"]}],"sources":[{"title":"First","url":"https://example.com/first","snippet":"Evidence"},{"title":"Second","url":"https://example.com/second","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Research, &research_policy());

    assert!(parsed.is_err());
}

#[test]
fn balanced_failed_finish_allows_a_valid_corrected_terminal_result() {
    // Given: AGY rejects one invalid structured-output attempt before the model corrects it.
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":1,"state":"ACTIVE","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":1,"state":"DONE","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":2,"state":"ACTIVE","step_type":"tool","tool_name":"finish","tool_info":{"name":"finish","parameters":{"unexpected":true}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":2,"state":"ERROR","step_type":"tool","tool_name":"finish","tool_info":{"name":"finish","parameters":{"unexpected":true},"error":{"type":"TOOL_ERROR","message":"invalid arguments"}}}}
{"event":"result","result":{"status":"SUCCESS","structured_output":{"object":"search","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"results":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Search, &standard_policy());

    assert!(parsed.is_ok());
}

#[test]
fn balanced_failed_web_tool_is_recoverable_only_for_standard_search() {
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":1,"state":"ACTIVE","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web"}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":1,"state":"ERROR","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web","error":{"type":"TOOL_ERROR","message":"invalid arguments"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":2,"state":"ACTIVE","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":2,"state":"DONE","step_type":"tool","tool_name":"search_web","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"result","result":{"status":"SUCCESS","structured_output":{"object":"search","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"results":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;

    let search = parse_structured_run(stream, Operation::Search, &standard_policy());
    assert!(matches!(
        search,
        Err(StructuredRunError::RecoverableFailedWebTool(_))
    ));

    let research = parse_structured_run(stream, Operation::Research, &research_policy());
    assert!(matches!(research, Err(StructuredRunError::Invalid(_))));
}

#[test]
fn terminal_result_closes_one_current_finish_but_rejects_a_foreign_finish() {
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":1,"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"evidence"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","step_index":2,"state":"ACTIVE","step_type":"tool","tool_info":{"name":"finish"}}}
{"event":"result","result":{"status":"SUCCESS","structured_output":{"object":"search","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"results":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;
    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_ok());

    let foreign = std::str::from_utf8(stream)
        .expect("test stream must be UTF-8")
        .replace(
            r#"conversation_id":"current-conversation","step_index":2"#,
            r#"conversation_id":"foreign-conversation","step_index":2"#,
        );
    assert!(
        parse_structured_run(foreign.as_bytes(), Operation::Search, &standard_policy()).is_err()
    );
}

#[test]
fn scoped_temporal_search_rejects_missing_or_mutated_active_query() {
    for parameters in [
        "{}",
        r#"{"query":"release status"}"#,
        r#"{"query":"release status \"alpha\" release date"}"#,
    ] {
        let stream = format!(
            r#"{{"event":"init"}}
{{"event":"step_update","step_update":{{"state":"ACTIVE","step_type":"tool","tool_info":{{"name":"search_web","parameters":{parameters}}}}}}}
{{"event":"step_update","step_update":{{"state":"DONE","step_type":"tool","tool_info":{{"name":"search_web","parameters":{parameters}}}}}}}
{{"event":"result","result":{{"structured_output":{{"object":"search","results":[{{"title":"Source","url":"https://example.com/","snippet":"Evidence"}}]}}}}}}"#
        );

        let parsed = parse_structured_run(stream.as_bytes(), Operation::Search, &scoped_policy());

        assert!(parsed.is_err());
    }
}

#[test]
fn scoped_temporal_search_rejects_read_url_attempt_after_a_valid_search() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"read_url_content"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Search, &scoped_policy());

    assert!(parsed.is_err());
}

#[test]
fn scoped_temporal_search_allows_a_single_value_followup() {
    let stream = br#"{"event":"init","conversation_id":"current-conversation"}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status 0.1.9"}}}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status 0.1.9"}}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    let events =
        stream::parse_events(std::str::from_utf8(stream).expect("test stream must be UTF-8"))
            .expect("test stream must parse");
    let policy = scoped_policy();
    let ResearchToolPolicy::ScopedTemporalSearch(required_query) = policy else {
        panic!("test policy must be scoped");
    };

    assert!(research_tool_policy::scoped_search_attempts_are_valid(
        &events,
        &required_query
    ));
}

#[test]
fn scoped_temporal_search_rejects_a_poisoned_followup() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status 0.1.9 July 29, 2026 https://example.com/release"}}}}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status 0.1.9 July 29, 2026 https://example.com/release"}}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    let events =
        stream::parse_events(std::str::from_utf8(stream).expect("test stream must be UTF-8"))
            .expect("test stream must parse");
    let policy = scoped_policy();
    let ResearchToolPolicy::ScopedTemporalSearch(required_query) = policy else {
        panic!("test policy must be scoped");
    };

    assert!(!research_tool_policy::scoped_search_attempts_are_valid(
        &events,
        &required_query
    ));
}

#[test]
fn scoped_temporal_search_requires_a_successful_search_and_rejects_failed_reads() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"}}}}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"For exact scope \"alpha\" only, find its latest release, exact version, and source-published date; do not use another scope's value. Original request constraints: release status"},"error":"network"}}}
{"event":"step_update","step_update":{"state":"ERROR","step_type":"tool","tool_info":{"name":"read_url_content","error":"denied"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Search, &scoped_policy());

    assert!(parsed.is_err());
}
