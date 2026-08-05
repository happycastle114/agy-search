use super::*;

#[test]
fn temporal_comparison_does_not_treat_a_local_first_row_as_truth_when_primary_value_differs()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: strong local rows whose values differ from both primary candidates.
    let temporary = TempDir::new()?;
    let (command, agy_trace) = traced_command(&temporary);
    let sources = [
        "https://example.com/local",
        "https://example.com/alpha",
        "https://example.com/beta",
    ];

    // When: recovery evaluates the locally parsed facts against primary values.
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

    // Then: no local value is promoted; the existing all-scope fallback runs.
    assert_eq!(
        response.pointer("/results/0/title"),
        Some(&json!("alpha-v2"))
    );
    assert_recovery_trace(&trace_scopes(&agy_trace)?);
    Ok(())
}
