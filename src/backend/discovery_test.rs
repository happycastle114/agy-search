use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use tempfile::TempDir;

use super::DiscoveryCache;
use crate::error::AgyError;

pub(super) struct CatalogFixture {
    _temporary: TempDir,
    pub(super) executable: PathBuf,
    trace: PathBuf,
    fail: PathBuf,
    slow: PathBuf,
}

impl CatalogFixture {
    pub(super) fn new(delay_seconds: u8) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = TempDir::new()?;
        let executable = temporary.path().join("agy");
        let trace = temporary.path().join("trace");
        let fail = temporary.path().join("fail");
        let slow = temporary.path().join("slow");
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{}'
if [ \"$1\" = '--version' ]; then printf '1.1.20\\n'; exit 0; fi
if [ -f '{}' ]; then exit 1; fi
if [ -f '{}' ]; then while [ -f '{}' ]; do sleep 0.01; done; fi
sleep {delay_seconds}\nprintf 'gemini-3.8-flash-low\\n'\n",
                trace.display(),
                fail.display(),
                slow.display(),
                slow.display(),
            ),
        )?;
        let mut permissions = fs::metadata(&executable)?.permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions)?;
        Ok(Self {
            _temporary: temporary,
            executable,
            trace,
            fail,
            slow,
        })
    }

    pub(super) fn calls(&self) -> Result<usize, std::io::Error> {
        self.invocations().map(|invocations| invocations.len())
    }

    pub(super) fn invocations(&self) -> Result<Vec<String>, std::io::Error> {
        match fs::read_to_string(&self.trace) {
            Ok(trace) => Ok(trace.lines().map(str::to_owned).collect()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    async fn wait_for_calls(&self, expected: usize) -> Result<(), std::io::Error> {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if self.calls()? >= expected {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "fixture did not start"))?
    }
}

async fn discover(cache: &DiscoveryCache, fixture: &CatalogFixture) -> Result<(), AgyError> {
    cache
        .catalog(
            fixture
                .executable
                .to_str()
                .ok_or(AgyError::InvalidCommand)?,
            Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
            Duration::from_secs(2),
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn reuses_successful_catalog_within_ttl() -> Result<(), Box<dyn std::error::Error>> {
    // Given: one successful catalog and a reusable cache.
    let fixture = CatalogFixture::new(0)?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: two requests discover the same executable and working directory.
    discover(&cache, &fixture).await?;
    discover(&cache, &fixture).await?;

    // Then: only one real advisory discovery occurred.
    assert_eq!(fixture.calls()?, 1);
    Ok(())
}

#[tokio::test]
async fn coalesces_concurrent_discovery_for_one_key() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a slow catalog and one cache key.
    let fixture = CatalogFixture::new(1)?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: two requests discover concurrently.
    let (first, second) = tokio::join!(discover(&cache, &fixture), discover(&cache, &fixture));
    first?;
    second?;

    // Then: singleflight starts one child process.
    assert_eq!(fixture.calls()?, 1);
    Ok(())
}

#[tokio::test]
async fn refreshes_expired_catalog() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a cache whose successful values expire immediately.
    let fixture = CatalogFixture::new(0)?;
    let cache = DiscoveryCache::new(Duration::ZERO);

    // When: the same catalog is requested again after expiry.
    discover(&cache, &fixture).await?;
    discover(&cache, &fixture).await?;

    // Then: both calls perform discovery.
    assert_eq!(fixture.calls()?, 2);
    Ok(())
}

#[tokio::test]
async fn retries_after_discovery_error() -> Result<(), Box<dyn std::error::Error>> {
    // Given: the first catalog discovery fails.
    let fixture = CatalogFixture::new(0)?;
    fs::write(&fixture.fail, [])?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: the failure clears and discovery is retried.
    assert!(matches!(
        discover(&cache, &fixture).await,
        Err(AgyError::ProcessFailed)
    ));
    fs::remove_file(&fixture.fail)?;
    discover(&cache, &fixture).await?;

    // Then: the error was not cached.
    assert_eq!(fixture.calls()?, 2);
    Ok(())
}

#[tokio::test]
async fn honors_discovery_deadline() -> Result<(), Box<dyn std::error::Error>> {
    // Given: catalog discovery takes longer than its remaining request budget.
    let fixture = CatalogFixture::new(1)?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: discovery is given a bounded deadline.
    let result = cache
        .catalog(
            fixture
                .executable
                .to_str()
                .ok_or(AgyError::InvalidCommand)?,
            Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
            Duration::from_millis(20),
        )
        .await;

    // Then: the process timeout is preserved.
    assert!(matches!(result, Err(AgyError::Timeout)));
    Ok(())
}

#[tokio::test]
async fn retains_at_most_one_executable_key() -> Result<(), Box<dyn std::error::Error>> {
    // Given: two executable identities and one cache slot.
    let first = CatalogFixture::new(0)?;
    let second = CatalogFixture::new(0)?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: discovery changes keys and then returns to the first key.
    discover(&cache, &first).await?;
    discover(&cache, &second).await?;
    discover(&cache, &first).await?;

    // Then: replacing the sole entry requires rediscovery of the evicted key.
    assert_eq!(first.calls()?, 2);
    assert_eq!(second.calls()?, 1);
    Ok(())
}

#[tokio::test]
async fn cancellation_does_not_publish_or_lock_inflight_discovery()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: an in-flight slow discovery holding the singleflight lock.
    let fixture = CatalogFixture::new(0)?;
    fs::write(&fixture.slow, [])?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: the child is observed running, its caller cancels, then retries.
    let mut inflight = Box::pin(discover(&cache, &fixture));
    tokio::select! {
        result = &mut inflight => return Err(format!("discovery ended before cancellation: {result:?}").into()),
        started = fixture.wait_for_calls(1) => started?,
    }
    assert_eq!(fixture.calls()?, 1);
    drop(inflight);
    fs::remove_file(&fixture.slow)?;
    discover(&cache, &fixture).await?;

    // Then: cancellation did not retain a stale value or a locked flight.
    assert_eq!(fixture.calls()?, 2);
    Ok(())
}

#[tokio::test]
async fn waiting_singleflight_respects_caller_deadline() -> Result<(), Box<dyn std::error::Error>> {
    // Given: one slow keyed discovery already owns the singleflight lock.
    let fixture = CatalogFixture::new(1)?;
    let cache = DiscoveryCache::new(Duration::from_secs(60));

    // When: a concurrent waiter has only a short request budget.
    let slow = discover(&cache, &fixture);
    let waiter = cache.catalog(
        fixture
            .executable
            .to_str()
            .ok_or(AgyError::InvalidCommand)?,
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Duration::from_millis(20),
    );
    let (slow_result, waiter_result) = tokio::join!(slow, waiter);
    slow_result?;

    // Then: waiting for the shared discovery cannot outlive that caller's deadline.
    assert!(matches!(waiter_result, Err(AgyError::Timeout)));
    assert_eq!(fixture.calls()?, 1);
    Ok(())
}
