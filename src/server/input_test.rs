use super::*;
use serde_json::json;

#[test]
fn search_rejects_empty_queries_and_unknown_fields() {
    // Given invalid wire-level requests.
    for value in [
        json!({"query":" "}),
        json!({"query":"valid","executable":"sh"}),
        json!({"query":"valid","search_domain_filter":["127.0.0.1"]}),
        json!({"query":"valid","source_urls":["http://example.com"]}),
    ] {
        // When the shared boundary parses them.
        let parsed = serde_json::from_value::<SearchInput>(value);
        // Then no unchecked input reaches the runtime.
        assert!(parsed.is_err());
    }
}

#[test]
fn search_bounds_batch_and_result_count() {
    // Given boundary-invalid search requests.
    for value in [
        json!({"query":[]}),
        json!({"query":["a","b","c","d","e","f"]}),
        json!({"query":"x","max_results":0}),
        json!({"query":"x","max_results":21}),
    ] {
        let input: SearchInput = serde_json::from_value(value).expect("valid JSON shape");
        // When converted to existing typed CLI commands.
        let prepared = input.prepare();
        // Then range errors fail before any subprocess starts.
        assert!(prepared.is_err());
    }
}

#[test]
fn stdin_marker_is_literal_query_for_network_clients() {
    // Given an HTTP/MCP query that resembles the CLI stdin shortcut.
    let input: SearchInput = serde_json::from_value(json!({"query":"-"})).expect("wire shape");
    // When converted at the shared boundary.
    let prepared = input.prepare().expect("literal query is valid");
    // Then network requests cannot read the server's stdin.
    assert!(
        matches!(prepared.commands.first(), Some(crate::cli::Command::Search(args)) if matches!(&args.query, crate::cli::QueryArgument::Text(text) if text.as_str() == "-"))
    );
}

#[test]
fn exact_source_restrictions_are_kept_in_shared_command() {
    // Given an explicit source restriction and non-default result limit.
    let input: SearchInput = serde_json::from_value(json!({"query":"example domains","max_results":2,"source_urls":["https://www.iana.org/help/example-domains"]})).expect("wire shape");
    // When prepared for execution.
    let prepared = input.prepare().expect("valid request");
    // Then the same CLI source boundary receives the request intact.
    assert!(
        matches!(prepared.commands.first(), Some(crate::cli::Command::Search(args)) if args.max_results == 2 && args.source_urls.first().is_some_and(|url| url.as_str() == "https://www.iana.org/help/example-domains"))
    );
}

#[test]
fn extraction_rejects_empty_and_oversized_url_sets() {
    // Given zero URLs and more than the existing CLI limit.
    for urls in [
        vec![],
        vec!["https://www.iana.org/help/example-domains"; 21],
    ] {
        let input: ExtractInput = serde_json::from_value(json!({"urls":urls})).expect("wire shape");
        // When creating the typed request.
        let command = input.into_command();
        // Then the runtime cannot bypass Clap's URL-count boundary.
        assert!(command.is_err());
    }
}
