use super::*;

#[test]
fn falls_back_to_a_bounded_get_when_a_publisher_rejects_head()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a grounded publisher article that reports 404 to HEAD but 200 to GET.
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    // When: the CLI validates the terminal publisher URL.
    let assertion = command(&curl, &trace, "head-rejected-terminal")
        .args(["search", "grounding-redirect"])
        .assert()
        .success();
    let search: Value = serde_json::from_slice(&assertion.get_output().stdout)?;

    // Then: the public URL survives only after a pinned, body-bounded GET probe.
    assert_eq!(
        search.pointer("/results/0/url"),
        Some(&json!("https://example.com/head-rejected"))
    );
    let records = trace_records(&trace)?;
    assert_eq!(records.len(), 3);
    let get = records.last().and_then(|record| record["argv"].as_array());
    assert!(get.is_some_and(|argv| {
        !argv.contains(&json!("--head"))
            && argv.contains(&json!("--range"))
            && argv.contains(&json!("--max-filesize"))
            && argv.iter().any(|value| value == "--resolve")
            && argv.windows(2).any(|pair| {
                pair.first() == Some(&json!("--connect-timeout"))
                    && pair.get(1).and_then(Value::as_str).is_some_and(|value| {
                        value.parse::<f64>().is_ok_and(|seconds| seconds > 0.0)
                    })
            })
    }));
    Ok(())
}

#[test]
fn bounded_get_never_follows_a_restricted_out_of_scope_redirect()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an allowed publisher that rejects HEAD and redirects GET out of scope.
    let (_temporary, curl) = fake_curl()?;
    let trace = curl.with_extension("trace");

    // When: every bounded Search attempt reaches that GET redirect.
    command(&curl, &trace, "head-rejected-disallowed-redirect")
        .args(["search", "grounding-redirect", "--domain", "example.com"])
        .assert()
        .code(6)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::eq("error: agy output invalid\n"));

    // Then: the disallowed target is rejected before any request reaches it.
    let records = trace_records(&trace)?;
    assert_eq!(records.len(), 9);
    assert!(records.iter().all(|record| {
        record
            .pointer("/url")
            .and_then(Value::as_str)
            .is_some_and(|url| !url.contains("iana.org"))
    }));
    Ok(())
}
