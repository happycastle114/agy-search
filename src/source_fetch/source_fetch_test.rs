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
