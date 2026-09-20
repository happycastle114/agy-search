use super::*;
use axum::body::Body;

fn policy() -> HttpSecurity {
    let options = HttpOptions {
        listen: "127.0.0.1:18092".parse().expect("test socket"),
        api_key_env: "UNUSED".to_owned(),
        allowed_hosts: vec!["api.example.test".to_owned()],
        allowed_origins: vec!["https://client.example.test".to_owned()],
        body_limit_bytes: 131_072,
    };
    HttpSecurity::new("test-bearer-token-12345".to_owned(), &options).expect("valid test policy")
}

#[test]
fn auth_rejects_missing_and_wrong_credentials() {
    // Given an allowed Host and an absent or incorrect token.
    for token in [None, Some("Bearer incorrect"), Some("Basic abc")] {
        let mut request = Request::builder()
            .uri("/search")
            .header("Host", "localhost:18092");
        if let Some(token) = token {
            request = request.header("Authorization", token);
        }
        // When authentication runs before the body or backend.
        let result = policy().validate(&request.body(Body::empty()).expect("test request"));
        // Then it rejects the request as unauthorized.
        assert!(matches!(result, Err(ServerError::Unauthorized)));
    }
}

#[test]
fn authenticated_host_and_origin_boundaries_are_enforced() {
    // Given a correct token but an untrusted authority or browser origin.
    for (host, origin) in [
        ("attacker.example", None),
        ("localhost", Some("https://attacker.example")),
        ("localhost", Some("null")),
    ] {
        let mut request = Request::builder()
            .uri("/mcp")
            .header("Host", host)
            .header("Authorization", "Bearer test-bearer-token-12345");
        if let Some(origin) = origin {
            request = request.header("Origin", origin);
        }
        // When applying the network trust boundary.
        let result = policy().validate(&request.body(Body::empty()).expect("test request"));
        // Then DNS-rebinding and cross-origin requests fail closed.
        assert!(matches!(result, Err(ServerError::Forbidden)));
    }
}

#[test]
fn configured_proxy_and_browser_origin_are_accepted() {
    // Given explicitly permitted authority, origin, and bearer token.
    let request = Request::builder()
        .uri("/mcp")
        .header("Host", "api.example.test:443")
        .header("Origin", "https://client.example.test")
        .header("Authorization", "Bearer test-bearer-token-12345")
        .body(Body::empty())
        .expect("test request");
    // When applying the policy.
    let result = policy().validate(&request);
    // Then this legitimate proxy request is accepted.
    assert!(result.is_ok());
}

#[test]
fn secret_is_redacted_from_debug_output() {
    // Given a configured token.
    let policy = policy();
    // When formatting diagnostic state.
    let debug = format!("{policy:?}");
    // Then the secret cannot leak through Debug.
    assert!(!debug.contains("test-bearer-token-12345"));
}
