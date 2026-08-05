//! Strict caller-source URL parsing and conservative public DNS pinning.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs as _};

use thiserror::Error;
use tokio::{
    task,
    time::{self, Instant},
};
use url::{Host, Url};

use crate::source_url::{HttpScheme, HttpUrl};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SafeSourceUrl {
    source: HttpUrl,
    host: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PinnedSource {
    url: SafeSourceUrl,
    address: IpAddr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueryPolicy {
    Reject,
    Allow,
}

#[derive(Clone, Copy, Debug, Error)]
pub(crate) enum SourceNetworkError {
    #[error("source URL is not a safe HTTPS URL")]
    InvalidUrl,
    #[error("source host did not resolve exclusively to public addresses")]
    UnsafeAddress,
    #[error("source DNS resolution failed")]
    Dns,
    #[error("source deadline elapsed")]
    Deadline,
}

impl SafeSourceUrl {
    pub(crate) fn parse(value: &str) -> Result<Self, SourceNetworkError> {
        Self::parse_with_query_policy(value, QueryPolicy::Reject)
    }

    pub(crate) fn parse_redirect(value: &str) -> Result<Self, SourceNetworkError> {
        Self::parse_with_query_policy(value, QueryPolicy::Allow)
    }

    fn parse_with_query_policy(
        value: &str,
        query_policy: QueryPolicy,
    ) -> Result<Self, SourceNetworkError> {
        let parsed = Url::parse(value.trim()).map_err(|_| SourceNetworkError::InvalidUrl)?;
        let source = HttpUrl::parse(value).map_err(|_| SourceNetworkError::InvalidUrl)?;
        if source.scheme() != Some(HttpScheme::Https)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.port_or_known_default() != Some(443)
            || matches!(query_policy, QueryPolicy::Reject) && parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(SourceNetworkError::InvalidUrl);
        }
        let host = match parsed.host() {
            Some(Host::Domain(domain)) if !is_local_name(domain) => domain.to_ascii_lowercase(),
            Some(Host::Ipv4(address)) if is_public_ip(IpAddr::V4(address)) => address.to_string(),
            Some(Host::Ipv6(address)) if is_public_ip(IpAddr::V6(address)) => address.to_string(),
            Some(Host::Domain(_) | Host::Ipv4(_) | Host::Ipv6(_)) | None => {
                return Err(SourceNetworkError::InvalidUrl);
            }
        };
        Ok(Self { source, host })
    }

    pub(crate) fn join_redirect(&self, location: &str) -> Result<Self, SourceNetworkError> {
        let base = Url::parse(self.as_str()).map_err(|_| SourceNetworkError::InvalidUrl)?;
        let joined = base
            .join(location.trim())
            .map_err(|_| SourceNetworkError::InvalidUrl)?;
        Self::parse_redirect(joined.as_str())
    }

    pub(crate) fn as_str(&self) -> &str {
        self.source.as_str()
    }

    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) const fn source(&self) -> &HttpUrl {
        &self.source
    }
}

impl PinnedSource {
    pub(crate) fn from_dns_answers(
        url: SafeSourceUrl,
        answers: &[IpAddr],
    ) -> Result<Self, SourceNetworkError> {
        if answers.is_empty() || answers.iter().any(|address| !is_public_ip(*address)) {
            return Err(SourceNetworkError::UnsafeAddress);
        }
        let mut public = answers.to_vec();
        public.sort_unstable();
        public.dedup();
        let address = public
            .first()
            .copied()
            .ok_or(SourceNetworkError::UnsafeAddress)?;
        Ok(Self { url, address })
    }

    pub(crate) const fn url(&self) -> &SafeSourceUrl {
        &self.url
    }

    pub(crate) const fn address(&self) -> IpAddr {
        self.address
    }
}

pub(crate) async fn resolve(
    url: SafeSourceUrl,
    deadline: Instant,
) -> Result<PinnedSource, SourceNetworkError> {
    let lookup_host = url.host.clone();
    let lookup = task::spawn_blocking(move || {
        (lookup_host.as_str(), 443)
            .to_socket_addrs()
            .map(|addresses| addresses.map(|address| address.ip()).collect::<Vec<_>>())
    });
    let addresses = time::timeout(remaining(deadline)?, lookup)
        .await
        .map_err(|_| SourceNetworkError::Deadline)?
        .map_err(|_| SourceNetworkError::Dns)?
        .map_err(|_| SourceNetworkError::Dns)?;
    PinnedSource::from_dns_answers(url, &addresses)
}

fn remaining(deadline: Instant) -> Result<std::time::Duration, SourceNetworkError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(SourceNetworkError::Deadline)
}

fn is_local_name(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    lower == "localhost" || lower.ends_with(".localhost")
}

fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 31 && c == 196)
        || (a == 192 && b == 52 && c == 193)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 175 && c == 48)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    let segments = address.segments();
    let first = segments[0];
    (0x2000..=0x3fff).contains(&first)
        && !(first == 0x2001 && segments[1] <= 0x01ff)
        && !(first == 0x2001 && segments[1] == 0x0db8)
        && first != 0x2002
        && !(first == 0x3fff && segments[1] <= 0x0fff)
}
