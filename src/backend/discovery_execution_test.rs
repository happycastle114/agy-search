use std::{path::PathBuf, time::Duration};

use clap::Parser;

use super::{DiscoveryCache, execute_cached};
use crate::{
    backend,
    cli::Cli,
    error::AgyError,
    invocation::{Invocation, InvocationCommand},
};

fn invocation(
    executable: &str,
    command: InvocationCommand,
) -> Result<Invocation, Box<dyn std::error::Error>> {
    Ok(Invocation {
        agy_path: executable.to_owned(),
        model: None,
        effort: None,
        timeout: "2".parse()?,
        command,
        output: None,
    })
}

#[tokio::test]
async fn cached_executor_keeps_diagnostic_commands_fresh() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a warmed advisory cache for one executable.
    let fixture = super::tests::CatalogFixture::new(0)?;
    let executable = fixture.executable.to_str().ok_or("fixture path")?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));
    cache
        .catalog(
            executable,
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Duration::from_secs(2),
        )
        .await?;

    // When: cached server execution handles models and status commands.
    execute_cached(invocation(executable, InvocationCommand::Models)?, &cache).await?;
    execute_cached(invocation(executable, InvocationCommand::Status)?, &cache).await?;

    // Then: both diagnostics bypass the advisory cache.
    assert_eq!(
        fixture.invocations()?,
        ["models", "models", "--version", "models"]
    );
    Ok(())
}

#[tokio::test]
async fn standalone_executor_keeps_model_discovery_fresh() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: the unchanged standalone executor and one executable.
    let fixture = super::tests::CatalogFixture::new(0)?;
    let executable = fixture.executable.to_str().ok_or("fixture path")?;

    // When: two standalone model invocations run.
    backend::execute(invocation(executable, InvocationCommand::Models)?).await?;
    backend::execute(invocation(executable, InvocationCommand::Models)?).await?;

    // Then: the CLI path performs two fresh discoveries.
    assert_eq!(fixture.calls()?, 2);
    Ok(())
}

#[tokio::test]
async fn explicit_model_validation_bypasses_advisory_cache()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a warmed advisory cache and an explicit model absent from the catalog.
    let fixture = super::tests::CatalogFixture::new(0)?;
    let executable = fixture.executable.to_str().ok_or("fixture path")?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));
    cache
        .catalog(
            executable,
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Duration::from_secs(2),
        )
        .await?;

    // When: strict explicit-model validation runs twice through cached execution.
    for _ in 0..2 {
        let cli = Cli::try_parse_from([
            "agy-search",
            "--agy-path",
            executable,
            "--model",
            "missing-model",
            "search",
            "cache policy",
        ])?;
        let result = execute_cached(cli.into_invocation()?, &cache).await;
        assert!(matches!(result, Err(AgyError::UnknownModel)));
    }

    // Then: every pin gets a fresh version guard and model catalog.
    assert_eq!(
        fixture.invocations()?,
        ["models", "--version", "models", "--version", "models"]
    );
    Ok(())
}
