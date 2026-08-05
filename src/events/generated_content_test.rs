use super::*;
use crate::{
    source_restriction::SourceRestriction,
    types::{HttpUrl, ResearchToolBudget},
};
use std::path::{Path, PathBuf};

const SEARCH: &str = r#"{"query":"release"}"#;
const READ: &str = r#"{"Url":"https://example.com/page"}"#;
const SEARCH_WEB: &str = "search_web";
const READ_URL: &str = "read_url_content";
const VIEW_FILE: &str = "view_file";
const GREP_SEARCH: &str = "grep_search";
const RUN_COMMAND: &str = "run_command";
const RESULT: &str = r#"{"event":"result","result":{"structured_output":{"object":"search","evidence_audit":{"candidates":[{"scope":"primary","claim":"Evidence","url":"https://example.com/page","date":null}],"coverage_complete":true,"conclusion":"Evidence"},"results":[{"title":"Source","url":"https://example.com/page","snippet":"Evidence"}]}}}"#;

fn policy(budget: ResearchToolBudget) -> ResearchToolPolicy {
    let restriction = SourceRestriction::parse(
        Vec::new(),
        vec![HttpUrl::parse("https://example.com/page").expect("valid URL")],
    )
    .expect("valid restriction");
    ResearchToolPolicy::Restricted {
        budget,
        restriction: Box::new(restriction),
    }
}

fn content_path(conversation: &str, producer: u64) -> PathBuf {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    PathBuf::from(std::env::var_os(key).expect("test HOME must identify content root"))
        .join(".gemini/antigravity-cli/brain")
        .join(conversation)
        .join(".system_generated/steps")
        .join(producer.to_string())
        .join("content.md")
}

fn quoted(path: &Path) -> String {
    serde_json::to_string(&path.to_string_lossy().to_string()).expect("path must serialize")
}
fn view(path: &Path) -> String {
    format!(r#"{{"AbsolutePath":{}}}"#, quoted(path))
}
fn grep(path: &Path) -> String {
    format!(r#"{{"Query":"release","SearchPath":{}}}"#, quoted(path))
}

fn step(conversation: &str, index: u64, state: &str, tool: &str, parameters: &str) -> String {
    let info = format!(r#"{{"name":"{tool}","parameters":{parameters}}}"#);
    let update = format!(
        r#"{{"conversation_id":"{conversation}","step_index":{index},"state":"{state}","step_type":"tool","tool_name":"{tool}","tool_info":{info}}}"#
    );
    format!(r#"{{"event":"step_update","step_update":{update}}}"#)
}

fn pair(conversation: &str, index: u64, tool: &str, parameters: &str) -> [String; 2] {
    [
        step(conversation, index, "ACTIVE", tool, parameters),
        step(conversation, index, "DONE", tool, parameters),
    ]
}

fn stream(steps: impl IntoIterator<Item = String>) -> String {
    std::iter::once(r#"{"event":"init","conversation_id":"current-conversation"}"#.to_owned())
        .chain(steps)
        .chain(std::iter::once(RESULT.to_owned()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn search_and_read() -> Vec<String> {
    [
        pair("current-conversation", 1, SEARCH_WEB, SEARCH),
        pair("current-conversation", 3, READ_URL, READ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn inspection_case(conversation: &str, active: &Path, done: &Path) -> String {
    let mut steps = search_and_read();
    steps.extend(pair(conversation, 4, VIEW_FILE, &view(active)));
    if active != done {
        steps.pop();
        steps.push(step(conversation, 4, "DONE", VIEW_FILE, &view(done)));
    }
    stream(steps)
}

#[test]
fn valid_generated_inspection_is_budget_neutral() {
    // Given successful web evidence and inspections of its generated artifact.
    let path = content_path("current-conversation", 3);
    let mut steps = search_and_read();
    steps.extend(pair("current-conversation", 4, VIEW_FILE, &view(&path)));
    steps.extend(pair("current-conversation", 5, GREP_SEARCH, &grep(&path)));

    // When the web-only budget is exactly consumed by search and read.
    let parsed = parse_structured_run(
        stream(steps).as_bytes(),
        Operation::Search,
        &policy(ResearchToolBudget::StandardSearch),
    );

    // Then generated inspections remain budget-neutral and the positive run passes.
    assert!(parsed.is_ok());
}

#[test]
fn valid_generated_grep_step_directory_is_budget_neutral() {
    // Given Antigravity's live grep shape: SearchPath is the completed read's step directory.
    let content = content_path("current-conversation", 3);
    let directory = content
        .parent()
        .expect("content path must have a step directory");
    let mut steps = search_and_read();
    steps.extend(pair(
        "current-conversation",
        4,
        GREP_SEARCH,
        &grep(directory),
    ));

    // When/Then the exact producer directory is safe and remains web-budget neutral.
    let parsed = parse_structured_run(
        stream(steps).as_bytes(),
        Operation::Search,
        &policy(ResearchToolBudget::StandardSearch),
    );
    assert!(parsed.is_ok());
}

#[test]
fn generated_grep_rejects_broader_sibling_and_workspace_directories() {
    let content = content_path("current-conversation", 3);
    let step = content
        .parent()
        .expect("content path must have a step directory");
    let steps_root = step.parent().expect("step path must have a generated root");
    let sibling = steps_root.join("4");
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for unsafe_path in [steps_root, sibling.as_path(), workspace.as_path()] {
        let mut steps = search_and_read();
        steps.extend(pair(
            "current-conversation",
            4,
            GREP_SEARCH,
            &grep(unsafe_path),
        ));
        let parsed = parse_structured_run(
            stream(steps).as_bytes(),
            Operation::Search,
            &policy(ResearchToolBudget::StandardSearch),
        );
        assert!(parsed.is_err(), "accepted unsafe grep path {unsafe_path:?}");
    }
}

#[test]
fn invalid_generated_content_attempts_fail_closed() {
    // Given unsafe paths, provenance, ordering, lifecycle, and unrelated-tool cases.
    let valid = content_path("current-conversation", 3);
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let mut cases = vec![
        inspection_case(
            "current-conversation",
            Path::new("/etc/passwd"),
            Path::new("/etc/passwd"),
        ),
        inspection_case("current-conversation", &workspace, &workspace),
        inspection_case(
            "sibling-conversation",
            &content_path("sibling-conversation", 3),
            &content_path("sibling-conversation", 3),
        ),
        inspection_case(
            "current-conversation",
            &content_path("current-conversation", 4),
            &content_path("current-conversation", 4),
        ),
        inspection_case("current-conversation", &valid, Path::new("/etc/passwd")),
    ];

    let mut before_done = vec![
        pair("current-conversation", 1, SEARCH_WEB, SEARCH)[0].clone(),
        pair("current-conversation", 1, SEARCH_WEB, SEARCH)[1].clone(),
        step("current-conversation", 3, "ACTIVE", READ_URL, READ),
    ];
    before_done.push(step(
        "current-conversation",
        4,
        "ACTIVE",
        VIEW_FILE,
        &view(&valid),
    ));
    before_done.push(step("current-conversation", 3, "DONE", READ_URL, READ));
    before_done.push(step(
        "current-conversation",
        4,
        "DONE",
        VIEW_FILE,
        &view(&valid),
    ));
    cases.push(stream(before_done));

    let mut unrelated = search_and_read();
    unrelated.extend(pair("current-conversation", 6, RUN_COMMAND, r"{}"));
    cases.push(stream(unrelated));

    // When every candidate is parsed through the stable event boundary.
    for candidate in cases {
        let parsed = parse_structured_run(
            candidate.as_bytes(),
            Operation::Search,
            &policy(ResearchToolBudget::TemporalSearch),
        );

        // Then every unsafe attempt fails closed.
        assert!(parsed.is_err());
    }
}
