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


//! Which network addresses an untrusted URL may point at.
//!
//! A downloader that follows links can be turned against the machine it runs on: a URL,
//! a redirect, or a DNS name that leads to `127.0.0.1`, a LAN host or the cloud metadata
//! service `169.254.169.254` would read something the URL's author could not reach. This
//! module decides, in one place and without I/O, which addresses count as "public".

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Whether hosts on loopback, private and other non-public addresses may be contacted.
///
/// The default, [`HostPolicy::BlockPrivate`], protects against server-side request forgery.
/// [`HostPolicy::AllowPrivate`] is for a development server or a machine on the local network,
/// and is only ever chosen explicitly (`--allow-private`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostPolicy {
    #[default]
    BlockPrivate,
    AllowPrivate,
}

impl HostPolicy {
    /// Whether an address the policy protects against should be refused.
    pub fn refuses(self, ip: IpAddr) -> bool {
        self == HostPolicy::BlockPrivate && !is_public(ip)
    }

    pub fn blocks_private(self) -> bool {
        self == HostPolicy::BlockPrivate
    }
}

/// `localhost` and every name under it always mean the local machine (RFC 6761), so they are
/// refused without asking DNS. A trailing dot and letter case do not matter.
pub fn is_local_name(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost" || host.ends_with(".localhost")
}

/// Whether `ip` is an ordinary, globally routable address.
///
/// Refused: loopback, private, link-local, carrier-grade NAT, unspecified, multicast, broadcast,
/// reserved and documentation ranges. IPv6 addresses that embed an IPv4 address (IPv4-mapped,
/// NAT64 and 6to4) are judged by the address they embed.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0 // "this network" (0.0.0.0/8)
        || ip.is_private() // 10/8, 172.16/12, 192.168/16
        || (a == 100 && (b & 0xC0) == 64) // carrier-grade NAT (100.64/10)
        || ip.is_loopback() // 127/8
        || ip.is_link_local() // 169.254/16, which includes the cloud metadata address
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments (192.0.0/24)
        || ip.is_documentation() // 192.0.2/24, 198.51.100/24, 203.0.113/24
        || (a == 198 && (b & 0xFE) == 18) // benchmarking (198.18/15)
        || ip.is_multicast() // 224/4
        || a >= 240) // reserved (240/4), which includes the broadcast address
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(embedded) = embedded_v4(ip) {
        return is_public_v4(embedded);
    }
    let first = ip.segments()[0];
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast() // ff00::/8
        || ip.is_unique_local() // fc00::/7
        || ip.is_unicast_link_local() // fe80::/10
        || (first & 0xFFC0) == 0xFEC0 // deprecated site-local (fec0::/10)
        || (first == 0x2001 && ip.segments()[1] == 0x0DB8)) // documentation (2001:db8::/32)
}

/// The IPv4 address hidden inside an IPv6 one, for the forms that carry one: IPv4-mapped
/// (`::ffff:a.b.c.d`), the deprecated IPv4-compatible form (`::a.b.c.d`), NAT64
/// (`64:ff9b::/96`) and 6to4 (`2002::/16`). `::` and `::1` are not treated as compatible.
fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = ip.segments();
    let low = Ipv4Addr::new((s[6] >> 8) as u8, s[6] as u8, (s[7] >> 8) as u8, s[7] as u8);
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return Some(mapped);
    }
    let high_zero = s[..6].iter().all(|&x| x == 0);
    if high_zero && !ip.is_unspecified() && !ip.is_loopback() {
        return Some(low);
    }
    if s[0] == 0x0064 && s[1] == 0xFF9B && s[2..6].iter().all(|&x| x == 0) {
        return Some(low);
    }
    if s[0] == 0x2002 {
        return Some(Ipv4Addr::new((s[1] >> 8) as u8, s[1] as u8, (s[2] >> 8) as u8, s[2] as u8));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn v4(text: &str) -> IpAddr {
        IpAddr::V4(text.parse().unwrap())
    }
    fn v6(text: &str) -> IpAddr {
        IpAddr::V6(text.parse().unwrap())
    }

    #[test]
    fn ordinary_addresses_are_public() {
        for ip in ["8.8.8.8", "1.1.1.1", "93.184.216.34", "172.32.0.1", "172.15.255.255", "100.63.255.255", "100.128.0.1", "198.17.0.1", "198.20.0.1", "223.255.255.255"] {
            assert!(is_public(v4(ip)), "{ip}");
        }
        for ip in ["2001:4860:4860::8888", "2606:4700:4700::1111", "2a00:1450:4001:81b::200e"] {
            assert!(is_public(v6(ip)), "{ip}");
        }
    }

    #[test]
    fn every_non_public_ipv4_range_is_refused() {
        for ip in [
            "0.0.0.0", "0.255.255.255", "10.0.0.1", "10.255.255.255", "100.64.0.1", "100.127.255.255",
            "127.0.0.1", "127.0.0.2", "127.255.255.255", "169.254.0.1", "169.254.169.254",
            "172.16.0.1", "172.31.255.255", "192.0.0.1", "192.0.2.1", "192.168.0.1", "192.168.255.255",
            "198.18.0.1", "198.19.255.255", "198.51.100.1", "203.0.113.1", "224.0.0.1", "239.255.255.255",
            "240.0.0.1", "255.255.255.255",
        ] {
            assert!(!is_public(v4(ip)), "{ip} must be refused");
        }
    }

    #[test]
    fn every_non_public_ipv6_range_is_refused() {
        for ip in ["::", "::1", "fc00::1", "fd00::1", "fe80::1", "febf::1", "fec0::1", "ff02::1", "2001:db8::1"] {
            assert!(!is_public(v6(ip)), "{ip} must be refused");
        }
    }

    #[test]
    fn ipv6_addresses_carrying_an_ipv4_address_are_judged_by_it() {
        for ip in ["::ffff:127.0.0.1", "::ffff:7f00:1", "::ffff:169.254.169.254", "::ffff:10.0.0.1", "::127.0.0.2", "64:ff9b::7f00:1", "64:ff9b::a00:1", "2002:7f00:1::", "2002:a9fe:a9fe::"] {
            assert!(!is_public(v6(ip)), "{ip} must be refused");
        }
        for ip in ["::ffff:8.8.8.8", "64:ff9b::808:808", "2002:808:808::"] {
            assert!(is_public(v6(ip)), "{ip} embeds a public address");
        }
    }

    #[test]
    fn local_names_are_recognised_whatever_their_spelling() {
        for name in ["localhost", "LOCALHOST", "localhost.", "foo.localhost", "a.b.Localhost.", "x.localhost"] {
            assert!(is_local_name(name), "{name}");
        }
        for name in ["example.com", "localhost.example.com", "notlocalhost", "mylocalhost", "localhost.evil.com.", ""] {
            assert!(!is_local_name(name), "{name}");
        }
    }

    #[test]
    fn the_policy_decides_whether_a_refusal_applies() {
        assert!(HostPolicy::BlockPrivate.refuses(v4("127.0.0.1")));
        assert!(!HostPolicy::BlockPrivate.refuses(v4("8.8.8.8")));
        assert!(!HostPolicy::AllowPrivate.refuses(v4("127.0.0.1")));
        assert_eq!(HostPolicy::default(), HostPolicy::BlockPrivate);
    }

    /// The same table as the code, written the other way round: `(network, prefix length)`.
    const REFERENCE: [(u32, u32); 15] = [
        (0x0000_0000, 8),  // 0/8
        (0x0A00_0000, 8),  // 10/8
        (0x6440_0000, 10), // 100.64/10
        (0x7F00_0000, 8),  // 127/8
        (0xA9FE_0000, 16), // 169.254/16
        (0xAC10_0000, 12), // 172.16/12
        (0xC000_0000, 24), // 192.0.0/24
        (0xC000_0200, 24), // 192.0.2/24
        (0xC0A8_0000, 16), // 192.168/16
        (0xC612_0000, 15), // 198.18/15
        (0xC633_6400, 24), // 198.51.100/24
        (0xCB00_7100, 24), // 203.0.113/24
        (0xE000_0000, 4),  // 224/4
        (0xF000_0000, 4),  // 240/4
        (0xFFFF_FFFF, 32), // broadcast (inside 240/4; listed on its own for clarity)
    ];

    fn reference_is_public(ip: u32) -> bool {
        !REFERENCE.iter().any(|&(net, prefix)| ip >> (32 - prefix) == net >> (32 - prefix))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20_000))]

        #[test]
        fn ipv4_classification_matches_an_independent_cidr_table(bits in any::<u32>()) {
            prop_assert_eq!(is_public(IpAddr::V4(Ipv4Addr::from(bits))), reference_is_public(bits));
        }

        #[test]
        fn a_mapped_address_is_judged_like_the_ipv4_address_inside_it(bits in any::<u32>()) {
            let inner = Ipv4Addr::from(bits);
            let mapped = IpAddr::V6(inner.to_ipv6_mapped());
            prop_assert_eq!(is_public(mapped), is_public(IpAddr::V4(inner)));
        }

        #[test]
        fn nat64_and_6to4_wrappers_are_judged_like_the_ipv4_address_inside_them(bits in any::<u32>()) {
            let inner = Ipv4Addr::from(bits);
            let o = inner.octets();
            let nat64 = Ipv6Addr::new(0x64, 0xFF9B, 0, 0, 0, 0, u16::from_be_bytes([o[0], o[1]]), u16::from_be_bytes([o[2], o[3]]));
            let sixtofour = Ipv6Addr::new(0x2002, u16::from_be_bytes([o[0], o[1]]), u16::from_be_bytes([o[2], o[3]]), 0, 0, 0, 0, 1);
            prop_assert_eq!(is_public(IpAddr::V6(nat64)), is_public(IpAddr::V4(inner)));
            prop_assert_eq!(is_public(IpAddr::V6(sixtofour)), is_public(IpAddr::V4(inner)));
        }

        #[test]
        fn no_address_makes_the_classifier_panic(bits in any::<u128>()) {
            let _ = is_public(IpAddr::V6(Ipv6Addr::from(bits)));
        }
    }
}
