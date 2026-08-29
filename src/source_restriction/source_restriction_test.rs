use super::*;

#[test]
fn domain_parser_canonicalizes_idna_case_and_one_trailing_dot() {
    let unicode = SourceDomain::from_str("BÜCHER.example.").expect("valid IDNA domain");
    let alabel = SourceDomain::from_str("xn--bcher-kva.example").expect("valid A-label");

    assert_eq!(unicode, alabel);
    assert_eq!(unicode.as_str(), "xn--bcher-kva.example");
}

#[test]
fn domain_parser_rejects_non_domain_authority_forms() {
    for candidate in [
        "https://example.com",
        "example.com/path",
        "example.com?query",
        "example.com#fragment",
        "user@example.com",
        "example.com:443",
        "*.example.com",
        "127.0.0.1",
        "example.com..",
        " example.com",
    ] {
        assert!(
            SourceDomain::from_str(candidate).is_err(),
            "accepted {candidate}"
        );
    }
}

#[test]
fn allowlist_uses_label_boundaries_and_exact_url_membership() {
    let restriction = SourceRestriction::parse(
        vec![SourceDomain::from_str("rust-lang.org").expect("valid domain")],
        vec![HttpUrl::parse("https://example.com/exact?q=1").expect("valid URL")],
    )
    .expect("valid restriction");

    assert!(
        restriction.allows(&HttpUrl::parse("https://doc.rust-lang.org/book").expect("valid URL"))
    );
    assert!(
        restriction.allows(&HttpUrl::parse("https://example.com/exact?q=1").expect("valid URL"))
    );
    assert!(
        !restriction
            .allows(&HttpUrl::parse("https://rust-lang.org.evil.example").expect("valid URL"))
    );
    assert!(
        !restriction.allows(&HttpUrl::parse("https://example.com/exact?q=2").expect("valid URL"))
    );
}

#[test]
fn bare_origins_are_evidence_only_when_the_caller_lists_the_exact_url() {
    let root = HttpUrl::parse("https://rust-lang.org/").expect("valid URL");
    let domain_only = SourceRestriction::parse(
        vec![SourceDomain::from_str("rust-lang.org").expect("valid domain")],
        Vec::new(),
    )
    .expect("valid restriction");
    let exact_root =
        SourceRestriction::parse(Vec::new(), vec![root.clone()]).expect("valid restriction");

    assert!(!SourceRestriction::Unrestricted.allows_evidence_url(&root));
    assert!(!domain_only.allows_evidence_url(&root));
    assert!(exact_root.allows_evidence_url(&root));
}

#[test]
fn restriction_rejects_canonical_duplicates_and_userinfo() {
    let duplicate_domains = ["BÜCHER.example", "xn--bcher-kva.example"]
        .into_iter()
        .map(SourceDomain::from_str)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid equivalent domains");
    let duplicate_urls = [
        "https://example.com:443/page#one",
        "https://example.com/page#two",
    ]
    .into_iter()
    .map(HttpUrl::parse)
    .collect::<Result<Vec<_>, _>>()
    .expect("valid equivalent URLs");
    let userinfo = HttpUrl::parse("https://user@example.com/page").expect("valid HTTP URL");

    assert!(SourceRestriction::parse(duplicate_domains, Vec::new()).is_err());
    assert!(SourceRestriction::parse(Vec::new(), duplicate_urls).is_err());
    assert!(SourceRestriction::parse(Vec::new(), vec![userinfo]).is_err());
}
