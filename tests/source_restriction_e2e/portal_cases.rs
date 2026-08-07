//! Portal-source ownership across unrestricted and caller-restricted Research.

use super::command;

#[test]
fn unrestricted_research_rejects_a_news_portal_source() {
    command()
        .args(["research", "source-research-news-portal"])
        .assert()
        .code(6);
}

#[test]
fn restricted_research_preserves_an_explicit_news_portal_source() {
    for restriction in [
        ["--source-url", "https://v.daum.net/v/20260807120301584"],
        ["--domain", "v.daum.net"],
    ] {
        command()
            .args(["research", "source-research-news-portal"])
            .args(restriction)
            .assert()
            .success();
    }
}
