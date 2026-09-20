use super::super::config::ServerCommand;
use super::*;
use axum::{
    body::Body,
    http::{Method, Request as HttpRequest},
};
use clap::Parser;
use tower::ServiceExt;

fn app() -> Result<Router, Box<dyn std::error::Error>> {
    let cli =
        super::super::ServerCli::try_parse_from(["server", "--agy-path", "/missing-agy", "http"])?;
    let ServerCommand::Http(options) = cli.command else {
        return Err("expected HTTP".into());
    };
    let security = HttpSecurity::new("test-bearer-token-12345".to_owned(), &options)?;
    Ok(router(
        Runtime::new(cli.runtime)?,
        &options,
        security,
        CancellationToken::new(),
    )?)
}

#[tokio::test]
async fn unauthenticated_health_and_guarded_search() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let health = app
        .clone()
        .oneshot(HttpRequest::builder().uri("/healthz").body(Body::empty())?)
        .await?;
    assert_eq!(health.status(), StatusCode::OK);
    let search = app
        .oneshot(
            HttpRequest::builder()
                .method(Method::POST)
                .uri("/search")
                .header("host", "localhost")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(search.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        search
            .headers()
            .get("www-authenticate")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer")
    );
    Ok(())
}

#[tokio::test]
async fn malformed_and_oversized_bodies_fail_before_backend()
-> Result<(), Box<dyn std::error::Error>> {
    for (body, expected) in [
        (r#"{"query":""}"#.to_owned(), StatusCode::BAD_REQUEST),
        ("x".repeat(131_073), StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let response = app()?
            .oneshot(
                HttpRequest::builder()
                    .method(Method::POST)
                    .uri("/search")
                    .header("host", "localhost")
                    .header("authorization", "Bearer test-bearer-token-12345")
                    .header("content-type", "application/json")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    Ok(())
}
