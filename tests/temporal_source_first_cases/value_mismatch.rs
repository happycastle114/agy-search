use super::*;

#[test]
fn exact_temporal_comparison_uses_unique_latest_local_rows_when_primary_values_differ()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: strong local rows whose values differ from both primary candidates.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let sources = [
        "https://example.com/local",
        "https://example.com/alpha",
        "https://example.com/beta",
    ];

    // When: exact-source recovery evaluates the locally parsed latest facts.
    let assertion = temporal_search(
        command,
        TemporalSearchFixture {
            scopes: ["alpha", "beta"],
            sources: &sources,
            query: "temporal-local-value-mismatch",
        },
    )
    .assert()
    .success();
    let response: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the unique latest exact-source value is promoted without a scoped model call.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha alpha-v2"))
    );
    assert_eq!(trace_scopes(&agy_trace)?, vec![None]);
    Ok(())
}
