use std::{
    fs,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::PathBuf,
    time::Duration,
};

use tokio::time::Instant;

use super::{PinnedSource, SafeSourceUrl, SourceFetcher};
use crate::{
    calendar_date::CalendarDate,
    source_contract::SourceContract,
    source_document::{CandidateBinding, SourceDocument},
};

#[tokio::test]
async fn starts_queued_fetch_before_slow_first_fetch_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{
        future::poll_fn,
        sync::{Arc, Mutex},
        task::{Poll, Waker},
    };

    enum Gate {
        Waiting(Option<Waker>),
        Open,
    }

    // Given: the first request can only finish after a queued request starts.
    let gate = Arc::new(Mutex::new(Gate::Waiting(None)));
    let count = super::MAX_FETCH_CONCURRENCY + 1;
    let requests = (0..count).map(|index| {
        let gate = Arc::clone(&gate);
        async move {
            if index == 0 {
                poll_fn(|context| {
                    let mut state = gate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    match &mut *state {
                        Gate::Waiting(waker) => {
                            *waker = Some(context.waker().clone());
                            Poll::Pending
                        }
                        Gate::Open => Poll::Ready(()),
                    }
                })
                .await;
            } else if index == super::MAX_FETCH_CONCURRENCY {
                let previous = std::mem::replace(
                    &mut *gate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                    Gate::Open,
                );
                match previous {
                    Gate::Waiting(Some(waker)) => waker.wake(),
                    Gate::Waiting(None) | Gate::Open => {}
                }
            }
            index
        }
    });

    // When: bounded retrieval runs, with a timeout only to detect deadlock.
    let fetched =
        tokio::time::timeout(Duration::from_secs(1), super::collect_bounded(requests)).await??;

    // Then: the queued request releases the first, with input order preserved.
    assert_eq!(fetched, (0..count).collect::<Vec<_>>());
    Ok(())
}

#[tokio::test]
async fn bounds_active_fetches_and_preserves_error_positions()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    // Given: more requests than slots, including one failed source.
    let active = Arc::new(AtomicUsize::new(0));
    let count = super::MAX_FETCH_CONCURRENCY * 2;
    let requests = (0..count).map(|index| {
        let active = Arc::clone(&active);
        async move {
            let running = active.fetch_add(1, Ordering::SeqCst) + 1;
            assert!(running <= super::MAX_FETCH_CONCURRENCY);
            tokio::task::yield_now().await;
            active.fetch_sub(1, Ordering::SeqCst);
            match index {
                0 => Err(super::SourceFetchError::EmptyBody),
                other => Ok(other),
            }
        }
    });

    // When: each completed request allows another to begin.
    let fetched = super::collect_bounded(requests).await?;

    // Then: errors retain their input position and every other request completes.
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert_eq!(fetched.len(), count);
    assert!(matches!(
        fetched.first(),
        Some(Err(super::SourceFetchError::EmptyBody))
    ));
    for (index, result) in fetched.into_iter().enumerate().skip(1) {
        assert_eq!(result?, index);
    }
    Ok(())
}

fn fake_curl() -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let executable = temporary.path().join("curl");
    fs::copy("tests/fixtures/fake_source_curl.sh", &executable)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
    }
    Ok((temporary, executable))
}

fn public_pin(path: &str) -> Result<PinnedSource, Box<dyn std::error::Error>> {
    let source = SafeSourceUrl::parse(&format!("https://example.com/{path}"))?;
    Ok(PinnedSource::from_dns_answers(
        source,
        &[IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))],
    )?)
}

#[tokio::test]
async fn safe_source_fetch_pins_public_dns_and_verifies_one_exact_binding()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: one caller-owned HTTPS source, a public DNS answer, and fake curl.
    let (_temporary, curl) = fake_curl()?;
    let argv_log = curl.with_extension("argv");
    let pinned = public_pin("good")?;
    let fetcher = SourceFetcher::new(curl);

    // When: the pinned source is fetched and its exact tuple is verified.
    let response = fetcher
        .fetch_pinned(&pinned, Instant::now() + Duration::from_secs(2))
        .await?;
    let contract = SourceContract::from_documents(vec![SourceDocument::parse(response)?])?;
    let date = CalendarDate::parse("2026-08-03")?;
    let binding = CandidateBinding::new(
        pinned.url(),
        "Example CLI",
        "1.2.3",
        &date,
        "August 3, 2026",
    )?;
    contract.verify(&binding)?;

    // Then: curl is hardened and pinned, and the tuple passes locally.
    let argv = fs::read_to_string(argv_log)?;
    let arguments: Vec<_> = argv.lines().collect();
    assert_eq!(arguments.first().copied(), Some("--disable"));
    assert!(arguments.windows(2).any(|pair| pair == ["--noproxy", "*"]));
    assert!(arguments.windows(2).any(|pair| {
        pair.first() == Some(&"--resolve") && pair.get(1) == Some(&"example.com:443:8.8.8.8")
    }));
    assert!(!arguments.contains(&"--location"));
    Ok(())
}

#[test]
fn rejects_unsafe_url_and_dns_inputs() -> Result<(), Box<dyn std::error::Error>> {
    // Given: unsafe URL authorities and unsafe DNS address classes.
    let unsafe_urls = [
        "http://example.com/release",
        "https://user@example.com/release",
        "https://example.com:8443/release",
        "https://example.com/release?view=all",
        "https://example.com/release#latest",
        "https://localhost/release",
    ];
    let unsafe_addresses = [
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
        IpAddr::V4(Ipv4Addr::new(192, 0, 0, 8)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        "fc00::1".parse()?,
        "fe80::1".parse()?,
        "ff02::1".parse()?,
        "2001:db8::1".parse()?,
    ];

    // When/Then: URL parsing and DNS pinning reject every unsafe input.
    assert!(
        unsafe_urls
            .iter()
            .all(|url| SafeSourceUrl::parse(url).is_err())
    );
    let source = SafeSourceUrl::parse("https://example.com/release")?;
    assert!(
        unsafe_addresses.iter().all(|address| {
            PinnedSource::from_dns_answers(source.clone(), &[*address]).is_err()
        })
    );
    assert!(
        PinnedSource::from_dns_answers(
            source,
            &[
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
                IpAddr::V4(Ipv4Addr::LOCALHOST),
            ],
        )
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn rejects_redirect_oversize_non_utf8_and_empty_transport_bodies()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a fake curl that produces each unsafe transport response.
    let (_temporary, curl) = fake_curl()?;
    let fetcher = SourceFetcher::new(curl);

    // When/Then: every unsafe response is rejected under the same deadline API.
    for mode in ["redirect", "oversize", "nonutf8", "empty"] {
        let pinned = public_pin(mode)?;
        let result = fetcher
            .fetch_pinned(&pinned, Instant::now() + Duration::from_secs(2))
            .await;
        assert!(result.is_err(), "unsafe transport mode passed: {mode}");
    }
    Ok(())
}
