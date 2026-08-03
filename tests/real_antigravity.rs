//! Opt-in compatibility gate against a signed-in real Antigravity CLI.

use std::{env, io, process::Command};

use serde_json::Value;

fn run(arguments: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_agy-search"))
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
#[ignore = "spends real Antigravity usage; run explicitly with the documented environment"]
fn real_stream_json_model_and_all_content_operations() -> Result<(), Box<dyn std::error::Error>> {
    let agy_path = env::var("AGY_SEARCH_AGY_PATH").unwrap_or_else(|_| "agy".to_owned());
    let model = env::var("AGY_SEARCH_REAL_MODEL")?;
    let common = ["--agy-path", agy_path.as_str(), "--timeout", "180"];

    let mut status_args = common.to_vec();
    status_args.extend(["status", "--json"]);
    assert_eq!(
        run(&status_args)?.get("available"),
        Some(&Value::Bool(true))
    );

    let mut model_args = common.to_vec();
    model_args.extend(["models", "--json"]);
    let models = run(&model_args)?;
    let available = models
        .get("models")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.as_str() == Some(model.as_str()))
        });
    assert!(available, "selected model must be dynamically discoverable");

    let scenarios: [(&[&str], &str); 5] = [
        (
            &["search", "IANA Example Domain official website", "-n", "2"],
            "search",
        ),
        (&["extract", "https://example.com/"], "extract"),
        (
            &[
                "map",
                "https://antigravity.google/",
                "--limit",
                "1",
                "--instructions",
                "Return the official changelog URL",
            ],
            "map",
        ),
        (&["crawl", "https://example.com/", "--limit", "1"], "crawl"),
        (
            &[
                "research",
                "Explain the purpose of IANA Example Domain using primary sources",
                "--max-sources",
                "3",
            ],
            "research",
        ),
    ];
    for (operation_args, expected) in scenarios {
        let mut arguments = common.to_vec();
        arguments.extend(["--model", model.as_str(), "--effort", "low"]);
        arguments.extend_from_slice(operation_args);
        assert_eq!(
            run(&arguments)?.get("object").and_then(Value::as_str),
            Some(expected)
        );
    }
    Ok(())
}
