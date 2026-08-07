use super::*;

use proptest::prelude::*;

#[test]
fn known_google_transport_and_wrapper_urls_are_never_direct() {
    for value in [
        "https://vertexaisearch.cloud.google.com./grounding-api-redirect/token",
        "https://vertexaisearch.cloud.google.com/another-transport-path",
        "https://www.google.com/url?q=https%3A%2F%2Fexample.com",
        "https://google.com/url?url=https%3A%2F%2Fexample.com",
        "https://news.google.com/articles/example",
        "https://www.google.co.kr/search?q=korean+market",
        "https://news.google.co.uk/articles/example",
        "https://googleusercontent.com/cached-source",
        "https://g.co/example",
    ] {
        let url = HttpUrl::parse(value).expect("Google transport URL must parse");
        assert_ne!(url.source_kind(), SourceUrlKind::Direct, "URL: {value}");
    }
}

#[test]
fn known_news_portal_syndication_urls_are_direct_sources_with_portal_identity() {
    for value in [
        "https://v.daum.net/v/20260807120301584",
        "https://news.daum.net/example",
        "https://n.news.naver.com/article/001/0010000000",
        "https://news.naver.com/example",
        "https://news.nate.com/view/20260807n00001",
    ] {
        let url = HttpUrl::parse(value).expect("news portal URL must parse");
        assert_eq!(url.source_kind(), SourceUrlKind::Direct, "URL: {value}");
        assert!(url.is_news_portal(), "URL: {value}");
    }
}

#[test]
fn generic_site_landing_paths_are_not_search_evidence_pages() {
    for value in [
        "https://example.com/",
        "https://example.com/main/main.jsp",
        "https://example.com/report/index.html",
        "https://example.com/home.php?locale=ko",
    ] {
        let url = HttpUrl::parse(value).expect("site landing URL must parse");
        assert!(url.is_site_root(), "URL: {value}");
    }
    let article = HttpUrl::parse("https://example.com/news/article.html?id=7")
        .expect("article URL must parse");
    assert!(!article.is_site_root());
}

proptest! {
    #[test]
    fn parsed_http_urls_with_one_host_share_an_origin(
        host in "[a-z]{1,12}",
        left_path in "[a-z0-9/]{0,20}",
        right_path in "[a-z0-9/]{0,20}",
    ) {
        let left = HttpUrl::parse(&format!("https://{host}.example/{left_path}"));
        let right = HttpUrl::parse(&format!("https://{host}.example/{right_path}"));
        prop_assert!(matches!((left, right), (Ok(left), Ok(right)) if left.same_origin(&right)));
    }
}
