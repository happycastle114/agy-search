//! Opt-in current-affairs accuracy gate against a signed-in real Antigravity CLI.

use std::{env, io, process::Command, time::Instant};

use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ResponseObject {
    Search,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    date: Option<String>,
    snippet: String,
    title: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    object: ResponseObject,
    results: Vec<SearchResult>,
}

#[derive(Clone, Copy)]
struct SourceOracle {
    url: &'static str,
    date: DateOracle,
}

#[derive(Clone, Copy)]
enum DateOracle {
    Exact(&'static str),
    ExactOrNull(&'static str),
    Null,
}

impl DateOracle {
    fn matches(self, actual: Option<&str>) -> bool {
        match self {
            Self::Exact(expected) => actual == Some(expected),
            Self::ExactOrNull(expected) => actual.is_none_or(|actual| actual == expected),
            Self::Null => actual.is_none(),
        }
    }
}

impl SourceOracle {
    fn matches_url(self, actual: &str) -> bool {
        let Ok(mut expected) = Url::parse(self.url) else {
            return false;
        };
        let Ok(mut actual) = Url::parse(actual) else {
            return false;
        };
        let mut expected_query = expected.query_pairs().into_owned().collect::<Vec<_>>();
        let mut actual_query = actual.query_pairs().into_owned().collect::<Vec<_>>();
        expected_query.sort_unstable();
        actual_query.sort_unstable();
        expected.set_query(None);
        actual.set_query(None);
        expected == actual && expected_query == actual_query
    }
}

#[derive(Clone, Copy, Debug)]
enum CurrentAffairsOracle {
    BankOfKoreaRate,
    RustRelease,
    WorldHealthOrganizationHeadline,
}

impl CurrentAffairsOracle {
    const ALL: [Self; 3] = [
        Self::BankOfKoreaRate,
        Self::RustRelease,
        Self::WorldHealthOrganizationHeadline,
    ];

    const fn query(self) -> &'static str {
        match self {
            Self::BankOfKoreaRate => {
                "2026년 8월 29일 현재 한국은행 기준금리와 가장 최근 변경일은? 공식 원문"
            }
            Self::RustRelease => {
                "As of August 29 2026, what is the latest stable Rust release and its release date?"
            }
            Self::WorldHealthOrganizationHeadline => {
                "As of August 29 2026, what is the latest WHO headline and its publication date? original source article"
            }
        }
    }

    const fn domain(self) -> &'static str {
        match self {
            Self::BankOfKoreaRate => "bok.or.kr",
            Self::RustRelease => "blog.rust-lang.org",
            Self::WorldHealthOrganizationHeadline => "who.int",
        }
    }

    const fn sources(self) -> &'static [SourceOracle] {
        match self {
            Self::BankOfKoreaRate => &[
                SourceOracle {
                    url: "https://www.bok.or.kr/portal/bbs/P0000559/view.do?depth=200690&menuNo=200690&nttId=11064191&programType=newsData&relate=Y",
                    date: DateOracle::ExactOrNull("2026-08-27"),
                },
                SourceOracle {
                    url: "https://www.bok.or.kr/portal/singl/baseRate/list.do?dataSeCd=01&menuNo=200643",
                    date: DateOracle::Null,
                },
            ],
            Self::RustRelease => &[SourceOracle {
                url: "https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/",
                date: DateOracle::Exact("2026-08-20"),
            }],
            Self::WorldHealthOrganizationHeadline => &[SourceOracle {
                url: "https://www.who.int/news/item/28-08-2026-second-meeting-of-the-ihr-emergency-committee-on-the-epidemic-of-ebola-bundibugyo-virus-disease-in-the-democratic-republic-of-the-congo---meeting-report",
                date: DateOracle::Exact("2026-08-28"),
            }],
        }
    }

    const fn markers(self) -> &'static [&'static str] {
        match self {
            Self::BankOfKoreaRate => &["3.00", "2.75", "2026"],
            Self::RustRelease => &["1.98.0", "rustup update stable"],
            Self::WorldHealthOrganizationHeadline => &[
                "second meeting",
                "ihr emergency committee",
                "ebola bundibugyo virus disease",
            ],
        }
    }
}

fn run_search(
    agy_path: &str,
    oracle: CurrentAffairsOracle,
) -> Result<f64, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_agy-search"))
        .args([
            "--agy-path",
            agy_path,
            "--effort",
            "low",
            "--timeout",
            "120",
        ])
        .args([
            "search",
            oracle.query(),
            "--max-results",
            "3",
            "--domain",
            oracle.domain(),
        ])
        .output()?;
    let elapsed = started.elapsed().as_secs_f64();
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "{oracle:?} failed after {elapsed:.2}s: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    if !output.stderr.is_empty() {
        return Err(io::Error::other("successful search emitted stderr").into());
    }
    let response: SearchResponse = serde_json::from_slice(&output.stdout)?;
    if response.object != ResponseObject::Search {
        return Err(io::Error::other("search response discriminator mismatch").into());
    }
    let (expected, source) = response
        .results
        .iter()
        .find_map(|result| {
            oracle
                .sources()
                .iter()
                .find(|source| source.matches_url(&result.url))
                .map(|source| (result, source))
        })
        .ok_or_else(|| io::Error::other(format!("{oracle:?} omitted the exact official URL")))?;
    if !source.date.matches(expected.date.as_deref()) {
        return Err(io::Error::other(format!("{oracle:?} returned the wrong date")).into());
    }
    let normalized = format!("{} {}", expected.title, expected.snippet).to_lowercase();
    for marker in oracle.markers() {
        if !normalized.contains(marker) {
            return Err(io::Error::other(format!(
                "{oracle:?} omitted same-page marker `{marker}`"
            ))
            .into());
        }
    }
    Ok(elapsed)
}

#[test]
#[ignore = "spends real Antigravity usage; production releases run this gate explicitly"]
fn current_affairs_search_gate() -> Result<(), Box<dyn std::error::Error>> {
    let agy_path = env::var("AGY_SEARCH_AGY_PATH").unwrap_or_else(|_| "agy".to_owned());
    for oracle in CurrentAffairsOracle::ALL {
        let elapsed = run_search(&agy_path, oracle)?;
        println!("current-affairs PASS oracle={oracle:?} elapsed={elapsed:.2}s");
    }
    Ok(())
}
