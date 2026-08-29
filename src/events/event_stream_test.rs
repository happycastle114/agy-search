use std::str::FromStr as _;

use super::*;
use crate::source_restriction::{SourceDomain, SourceRestriction};
use crate::types::{ResearchAttemptBudget, ResearchToolBudget};

fn standard_policy() -> ResearchToolPolicy {
    ResearchToolPolicy::Budget(ResearchToolBudget::StandardSearch)
}

fn restricted_research_policy() -> ResearchToolPolicy {
    let restriction = SourceRestriction::parse(
        vec![SourceDomain::from_str("example.com").expect("valid test domain")],
        Vec::new(),
    )
    .expect("valid test restriction");
    ResearchToolPolicy::Restricted {
        budget: ResearchToolBudget::Research(ResearchAttemptBudget::from_max_sources(4)),
        restriction: Box::new(restriction),
    }
}

#[test]
fn generic_mcp_tool_does_not_prove_web_research() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"call_mcp_tool"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_err());
}

#[test]
fn terminal_result_must_be_the_final_event() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}
{"event":"init"}"#;

    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_err());
}

#[test]
fn unknown_stream_event_is_rejected_before_terminal_output() {
    let stream = br#"{"event":"init"}
{"event":"unsupported"}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_err());
}

#[test]
fn completed_tool_with_error_does_not_prove_research() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","error":"private failure"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_err());
}

#[test]
fn successful_terminal_event_without_structured_output_fails_closed() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web"}}}
{"event":"result","result":{"status":"SUCCESS","response":"not the result"}}"#;

    assert!(parse_structured_run(stream, Operation::Search, &standard_policy()).is_err());
}

#[test]
fn optional_runtime_metadata_and_intermediate_errors_do_not_replace_terminal_output() {
    let stream = br#"{"event":"init","conversation_id":"current-conversation","init":{"expanded_commands":[{"name":"plan","type":"system"}]}}
{"event":"step_update","step_update":{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{"name":"search_web"}}}
{"event":"step_update","step_update":{"state":"DONE","step_type":"error_message"}}
{"event":"result","result":{"status":"SUCCESS","response":"invalid prose","structured_output":{"object":"search","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Search, &standard_policy());

    assert!(parsed.is_ok());
}

#[test]
fn restricted_direct_read_attempt_must_be_allowlisted() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"evidence site:example.com"}}}}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"read_url_content","parameters":{"Url":"https://outside.example/page"}}}}
{"event":"result","result":{"structured_output":{"object":"research","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"title":"Research","summary":"Evidence","findings":[{"title":"Finding","summary":"Evidence","citations":["https://example.com/page"]}],"sources":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Research, &restricted_research_policy());

    assert!(parsed.is_err());
}

#[test]
fn unfinished_grounding_read_fails_closed_before_async_resolution() {
    let transport = "https://vertexaisearch.cloud.google.com/grounding-api-redirect/token";
    let stream = format!(
        r#"{{"event":"init","conversation_id":"current-conversation"}}
{{"event":"step_update","step_update":{{"conversation_id":"current-conversation","state":"DONE","step_type":"tool","tool_info":{{"name":"search_web","parameters":{{"query":"evidence site:example.com"}}}}}}}}
{{"event":"step_update","step_update":{{"conversation_id":"current-conversation","state":"ACTIVE","step_type":"tool","tool_info":{{"name":"read_url_content","parameters":{{"Url":"{transport}"}}}}}}}}
{{"event":"result","result":{{"structured_output":{{"object":"research","evidence_audit":{{"candidates":[{{"scope":"primary","claim":"Evidence","url":"{transport}","date":null}}],"coverage_complete":true,"conclusion":"Evidence"}},"title":"Research","summary":"Evidence","findings":[{{"title":"Finding","summary":"Evidence","citations":["{transport}"]}}],"sources":[{{"title":"Source","url":"{transport}","snippet":"Evidence"}}]}}}}}}"#
    );

    let parsed = parse_structured_run(
        stream.as_bytes(),
        Operation::Research,
        &restricted_research_policy(),
    );

    assert!(parsed.is_err());
}

#[test]
fn restricted_read_rejects_an_untyped_grounding_transport() {
    let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","parameters":{"query":"evidence site:example.com"}}}}
{"event":"step_update","step_update":{"state":"ACTIVE","step_type":"tool","tool_info":{"name":"read_url_content","parameters":{"Url":"file:///grounding-api-redirect/token"}}}}
{"event":"result","result":{"structured_output":{"object":"research","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"title":"Research","summary":"Evidence","findings":[{"title":"Finding","summary":"Evidence","citations":["https://example.com/page"]}],"sources":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;

    let parsed = parse_structured_run(stream, Operation::Research, &restricted_research_policy());

    assert!(parsed.is_err());
}
