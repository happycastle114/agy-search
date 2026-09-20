//! Bearer authentication and HTTP origin boundaries.

use super::{ServerError, config::HttpOptions};
use axum::{
    extract::{Request, State},
    http::{header, uri::Authority},
    middleware::Next,
    response::Response,
};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};
use secrecy::{ExposeSecret, SecretString};
use std::{collections::HashSet, sync::Arc};
use subtle::ConstantTimeEq;

#[derive(Debug)]
pub(super) struct HttpSecurity {
    token: SecretString,
    hosts: HashSet<String>,
    origins: HashSet<String>,
}

impl HttpSecurity {
    pub(super) fn load(options: &HttpOptions) -> Result<Self, ServerError> {
        let token = std::env::var(&options.api_key_env).map_err(|_| ServerError::Configuration)?;
        Self::new(token, options)
    }

    pub(super) fn new(token: String, options: &HttpOptions) -> Result<Self, ServerError> {
        if token.len() < 16
            || token.len() > 1024
            || !token.is_ascii()
            || token.bytes().any(|byte| byte.is_ascii_whitespace())
        {
            return Err(ServerError::Configuration);
        }
        let mut hosts: HashSet<String> = ["localhost", "127.0.0.1", "::1"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        if !options.listen.ip().is_unspecified() {
            hosts.insert(options.listen.ip().to_string());
        }
        for host in &options.allowed_hosts {
            let authority: Authority = host.parse().map_err(|_| ServerError::Configuration)?;
            if authority.port().is_some() || authority.host().contains('*') {
                return Err(ServerError::Configuration);
            }
            hosts.insert(
                authority
                    .host()
                    .trim_matches(['[', ']'])
                    .to_ascii_lowercase(),
            );
        }
        let mut origins = HashSet::new();
        for origin in &options.allowed_origins {
            let parsed = url::Url::parse(origin).map_err(|_| ServerError::Configuration)?;
            if crate::source_url::HttpScheme::parse(parsed.scheme()).is_none()
                || parsed.origin().ascii_serialization() != *origin
            {
                return Err(ServerError::Configuration);
            }
            origins.insert(origin.clone());
        }
        Ok(Self {
            token: token.into(),
            hosts,
            origins,
        })
    }

    pub(super) fn hosts(&self) -> Vec<String> {
        self.hosts.iter().cloned().collect()
    }
    pub(super) fn origins(&self) -> Vec<String> {
        self.origins.iter().cloned().collect()
    }

    fn validate(&self, request: &Request) -> Result<(), ServerError> {
        let authority = request
            .headers()
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Authority>().ok())
            .or_else(|| request.uri().authority().cloned())
            .ok_or(ServerError::Forbidden)?;
        let host = authority
            .host()
            .trim_matches(['[', ']'])
            .to_ascii_lowercase();
        if !self.hosts.contains(&host) {
            return Err(ServerError::Forbidden);
        }
        if let Some(origin) = request.headers().get(header::ORIGIN) {
            let origin = origin.to_str().map_err(|_| ServerError::Forbidden)?;
            if !self.origins.contains(origin) {
                return Err(ServerError::Forbidden);
            }
        }
        let credential = request
            .headers()
            .typed_get::<Authorization<Bearer>>()
            .ok_or(ServerError::Unauthorized)?;
        if !bool::from(
            credential
                .token()
                .as_bytes()
                .ct_eq(self.token.expose_secret().as_bytes()),
        ) {
            return Err(ServerError::Unauthorized);
        }
        Ok(())
    }
}

pub(super) async fn authorize(
    State(security): State<Arc<HttpSecurity>>,
    request: Request,
    next: Next,
) -> Result<Response, ServerError> {
    security.validate(&request)?;
    Ok(next.run(request).await)
}

#[cfg(test)]
#[path = "security_test.rs"]
mod tests;
