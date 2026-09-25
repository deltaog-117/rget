// SPDX-License-Identifier: GPL-3.0-or-later
// rget - A safe, modern downloader for Linux
// Copyright (C) 2026  Aeon Ennoia
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! HTTP client construction and request decoration.

use super::error::{Error, Result};
use crate::shared::address::{is_local_name, is_public, HostPolicy};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{RETRY_AFTER, USER_AGENT};
use reqwest::redirect::Policy;
use std::error::Error as StdError;
use std::fmt;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use url::{Host, Url};

/// The most redirects one request may follow.
const MAX_REDIRECTS: usize = 10;

/// Sent as `User-Agent` when `-A`/`--user-agent` and the config file leave it unset.
const DEFAULT_USER_AGENT: &str = concat!("rget/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn StdError + Send + Sync>;

/// Carried through `reqwest`'s error chain when we refuse a destination, so it can be told
/// apart from an ordinary network failure (see [`refusal_in`]).
#[derive(Debug)]
struct Refused(String);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for Refused {}

/// The reason we refused a destination, if `error` was caused by that.
pub(super) fn refusal_in(error: &reqwest::Error) -> Option<String> {
    let mut source: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(e) = source {
        if let Some(refused) = e.downcast_ref::<Refused>() {
            return Some(refused.0.clone());
        }
        source = e.source();
    }
    None
}

/// Why `url` must not be contacted, judged by its host alone: an IP literal that is not
/// public, or a name that always means this machine. `None` means it may be.
fn destination_refusal(url: &Url) -> Option<String> {
    match url.host()? {
        Host::Ipv4(ip) if !is_public(IpAddr::V4(ip)) => {
            Some(format!("{} is a local or private address", ip))
        }
        Host::Ipv6(ip) if !is_public(IpAddr::V6(ip)) => {
            Some(format!("{} is a local or private address", ip))
        }
        Host::Domain(name) if is_local_name(name) => {
            Some(format!("{} always refers to this machine", name))
        }
        _ => None,
    }
}

/// Follows redirects, but never into a local or private destination.
fn guarded_redirects() -> Policy {
    Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        match destination_refusal(attempt.url()) {
            Some(reason) => {
                let message = format!("redirect to {} refused: {}", attempt.url(), reason);
                attempt.error(Refused(message))
            }
            None => attempt.follow(),
        }
    })
}

/// Keeps the public addresses a name resolved to. A name that resolves *only* to non-public
/// addresses is refused; one that resolves to nothing is left for the caller to report.
fn keep_public(
    host: &str,
    addrs: Vec<SocketAddr>,
) -> std::result::Result<Vec<SocketAddr>, Refused> {
    let public: Vec<SocketAddr> = addrs
        .iter()
        .copied()
        .filter(|a| is_public(a.ip()))
        .collect();
    if public.is_empty() && !addrs.is_empty() {
        return Err(Refused(format!(
            "{} resolves only to local or private addresses",
            host
        )));
    }
    Ok(public)
}

/// Turns a host name into addresses. The system resolver in production; replaceable in tests.
type Lookup = fn(&str) -> std::io::Result<Vec<SocketAddr>>;

fn system_lookup(host: &str) -> std::io::Result<Vec<SocketAddr>> {
    (host, 0u16).to_socket_addrs().map(|found| found.collect())
}

/// A DNS resolver that never hands back a non-public address. Because the connection is made
/// to exactly the addresses returned here, a name cannot be checked as harmless and then
/// re-resolved to something else (DNS rebinding), and it covers every redirect hop as well.
struct FilteringResolver {
    lookup: Lookup,
}

impl Resolve for FilteringResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let lookup = self.lookup;
        Box::pin(async move {
            if is_local_name(&host) {
                return Err(
                    Box::new(Refused(format!("{} always refers to this machine", host)))
                        as BoxError,
                );
            }
            let to_resolve = host.clone();
            let addrs = tokio::task::spawn_blocking(move || lookup(&to_resolve))
                .await
                .map_err(|e| Box::new(e) as BoxError)?
                .map_err(|e| Box::new(e) as BoxError)?;
            let kept = keep_public(&host, addrs).map_err(|r| Box::new(r) as BoxError)?;
            Ok(Box::new(kept.into_iter()) as Addrs)
        })
    }
}

/// Builds a blocking client with the given timeout, redirect behaviour and host policy.
///
/// `timeout` bounds connecting and receiving the response headers as a whole. While the
/// body is read with `Read::read`, the same duration applies afresh to *each* read, so
/// it acts as a stall limit and never caps the total transfer time.
///
/// With [`HostPolicy::BlockPrivate`], redirects into local or private destinations are
/// refused and names are only ever resolved to public addresses.
pub(super) fn build(timeout: u64, follow_redirects: bool, policy: HostPolicy) -> Result<Client> {
    build_with_lookup(timeout, follow_redirects, policy, system_lookup)
}

fn build_with_lookup(
    timeout: u64,
    follow_redirects: bool,
    policy: HostPolicy,
    lookup: Lookup,
) -> Result<Client> {
    let redirects = if !follow_redirects {
        Policy::none()
    } else if policy.blocks_private() {
        guarded_redirects()
    } else {
        Policy::limited(MAX_REDIRECTS)
    };

    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .redirect(redirects);
    if policy.blocks_private() {
        client_builder = client_builder.dns_resolver(Arc::new(FilteringResolver { lookup }));
    }

    Ok(client_builder.build()?)
}

/// Adds the User-Agent and every custom header to `request_builder`.
pub(super) fn apply_headers(
    mut request_builder: RequestBuilder,
    user_agent: Option<&str>,
    headers: &[(String, String)],
) -> RequestBuilder {
    if let Some(ua) = user_agent {
        request_builder = request_builder.header(USER_AGENT, ua);
    } else {
        request_builder = request_builder.header(USER_AGENT, DEFAULT_USER_AGENT);
    }

    // Apply custom headers
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }

    request_builder
}

/// Turns any non-2xx response into [`Error::HttpStatus`] so that an error page is never
/// written to disk as if it were the file.
pub(super) fn ensure_success(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| super::retry::parse_retry_after(v, SystemTime::now()));
        Err(Error::HttpStatus {
            status,
            retry_after,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn addr(text: &str) -> SocketAddr {
        SocketAddr::new(text.parse().unwrap(), 0)
    }

    #[test]
    fn destinations_are_refused_by_their_host() {
        for target in [
            "http://127.0.0.1/x",
            "http://127.0.0.2:8080/x",
            "http://169.254.169.254/latest",
            "http://10.1.2.3/x",
            "http://192.168.0.1/x",
            "http://0.0.0.0/x",
            "http://[::1]/x",
            "http://[::ffff:127.0.0.1]/x",
            "http://localhost/x",
            "http://foo.localhost/x",
            "http://localhost./x",
        ] {
            assert!(destination_refusal(&url(target)).is_some(), "{target}");
        }
    }

    #[test]
    fn public_destinations_and_ordinary_names_are_left_alone() {
        for target in [
            "http://8.8.8.8/x",
            "https://example.com/x",
            "http://[2001:4860:4860::8888]/x",
            "http://internal.example.com/x",
        ] {
            assert_eq!(destination_refusal(&url(target)), None, "{target}");
        }
    }

    #[test]
    fn a_name_is_kept_only_for_its_public_addresses() {
        let mixed = vec![addr("10.0.0.1"), addr("93.184.216.34"), addr("::1")];
        assert_eq!(
            keep_public("example.com", mixed).unwrap(),
            vec![addr("93.184.216.34")]
        );
    }

    #[test]
    fn a_name_that_resolves_only_to_private_addresses_is_refused() {
        let err =
            keep_public("evil.example", vec![addr("127.0.0.1"), addr("192.168.1.1")]).unwrap_err();
        assert!(
            err.0.contains("evil.example") && err.0.contains("local or private"),
            "{}",
            err.0
        );
    }

    /// A one-shot local HTTP server, so a request that is *not* refused has somewhere to land.
    fn local_server() -> (u16, std::sync::mpsc::Receiver<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (contacted, seen) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let _ = contacted.send(());
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                );
            }
        });
        (port, seen)
    }

    fn resolves_to_loopback(_: &str) -> std::io::Result<Vec<SocketAddr>> {
        Ok(vec![addr("127.0.0.1")])
    }

    fn resolves_to_a_mix_of_private_addresses(_: &str) -> std::io::Result<Vec<SocketAddr>> {
        Ok(vec![addr("10.0.0.1"), addr("192.168.1.1"), addr("::1")])
    }

    #[test]
    fn the_installed_resolver_refuses_a_name_that_leads_to_loopback() {
        let (port, contacted) = local_server();
        let client =
            build_with_lookup(5, true, HostPolicy::BlockPrivate, resolves_to_loopback).unwrap();

        let error: Error = client
            .get(format!("http://evil.example:{port}/"))
            .send()
            .unwrap_err()
            .into();

        match error {
            Error::BlockedAddress(reason) => assert!(
                reason.contains("evil.example") && reason.contains("local or private"),
                "{reason}"
            ),
            other => panic!("expected BlockedAddress, got {other:?}"),
        }
        assert!(
            contacted.recv_timeout(Duration::from_millis(300)).is_err(),
            "the server must not have been contacted"
        );
    }

    #[test]
    fn the_installed_resolver_refuses_a_name_that_only_leads_to_private_addresses() {
        let client = build_with_lookup(
            5,
            true,
            HostPolicy::BlockPrivate,
            resolves_to_a_mix_of_private_addresses,
        )
        .unwrap();
        let error: Error = client
            .get("http://evil.example:81/")
            .send()
            .unwrap_err()
            .into();
        assert!(matches!(error, Error::BlockedAddress(_)), "{error:?}");
    }

    #[test]
    fn allowing_private_hosts_installs_no_filter_at_all() {
        // The injected lookup is never consulted, so an unknown name fails as an ordinary DNS
        // error, not as a refusal.
        let client =
            build_with_lookup(3, true, HostPolicy::AllowPrivate, resolves_to_loopback).unwrap();
        let error: Error = client
            .get("http://no-such-host.invalid:81/")
            .send()
            .unwrap_err()
            .into();
        assert!(matches!(error, Error::Network(_)), "{error:?}");
    }

    #[test]
    fn an_ordinary_network_failure_is_not_mistaken_for_a_refusal() {
        let client =
            build_with_lookup(2, true, HostPolicy::BlockPrivate, resolves_to_loopback).unwrap();
        // Port 1 on a public-looking literal is never reached: the literal skips the resolver,
        // and connecting to a documentation address fails as an ordinary network error.
        let error: Error = client
            .get("http://192.0.2.1:1/")
            .timeout(Duration::from_millis(300))
            .send()
            .unwrap_err()
            .into();
        assert!(matches!(error, Error::Network(_)), "{error:?}");
    }

    #[test]
    fn a_name_that_resolves_to_nothing_is_left_for_the_caller_to_report() {
        assert_eq!(
            keep_public("nothing.example", Vec::new()).unwrap(),
            Vec::<SocketAddr>::new()
        );
    }
}
