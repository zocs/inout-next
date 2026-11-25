//! Share-address candidates with conservative link labels and a stable order.
//!
//! Only facts reported by the platform label a link. An interface whose type
//! cannot be confirmed stays `Unknown`; its name is never used to guess Wi-Fi.
//! The order decides which address is shown first, not which interfaces the
//! listener binds.
use serde::Serialize;
use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkKind {
    Wifi,
    Wired,
    Unknown,
    /// Bridges, tunnels, containers, virtual machines and VPN adapters.
    Virtual,
    Loopback,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareEndpoint {
    pub url: String,
    /// Platform interface name; empty for the loopback entry.
    pub interface: String,
    pub kind: LinkKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub name: String,
    pub ip: Ipv4Addr,
    pub kind: LinkKind,
    pub up: bool,
}

fn rank(candidate: &Candidate) -> u8 {
    match (candidate.kind, candidate.up) {
        // Receiving phones are normally on Wi-Fi; a cable on the same LAN
        // follows. Virtual links never outrank a real or unconfirmed one.
        (LinkKind::Wifi, true) => 0,
        (LinkKind::Wired, true) => 1,
        (LinkKind::Unknown, true) => 2,
        (LinkKind::Wifi | LinkKind::Wired | LinkKind::Unknown, false) => 3,
        (LinkKind::Virtual, _) => 4,
        (LinkKind::Loopback, _) => 5,
    }
}

/// Deterministic recommendation order, independent of enumeration order.
pub(crate) fn ordered(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates
        .sort_by(|a, b| (rank(a), &a.name, a.ip.octets()).cmp(&(rank(b), &b.name, b.ip.octets())));
    candidates.dedup_by(|later, earlier| later.ip == earlier.ip);
    let mut seen = Vec::with_capacity(candidates.len());
    candidates.retain(|candidate| {
        let fresh = !seen.contains(&candidate.ip);
        seen.push(candidate.ip);
        fresh
    });
    candidates
}

/// Classify from a Linux-style `class/net` tree. A missing or unreadable
/// entry yields `Unknown`, as on Android where apps cannot read it.
#[cfg_attr(not(any(target_os = "linux", target_os = "android")), allow(dead_code))]
pub(crate) fn classify_in(class_net: &std::path::Path, name: &str) -> LinkKind {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        || name == "."
        || name == ".."
    {
        return LinkKind::Unknown;
    }
    let entry = class_net.join(name);
    if !entry.join("type").is_file() {
        return LinkKind::Unknown;
    }
    if entry.join("wireless").is_dir() || entry.join("phy80211").exists() {
        return LinkKind::Wifi;
    }
    if !entry.join("device").exists() {
        return LinkKind::Virtual;
    }
    match std::fs::read_to_string(entry.join("type")) {
        // ARPHRD_ETHER with a backing device.
        Ok(value) if value.trim() == "1" => LinkKind::Wired,
        _ => LinkKind::Unknown,
    }
}

fn classify(interface: &if_addrs::Interface) -> LinkKind {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let kind = classify_in(std::path::Path::new("/sys/class/net"), &interface.name);
        if kind != LinkKind::Unknown {
            return kind;
        }
    }
    if interface.is_p2p() {
        LinkKind::Virtual
    } else {
        LinkKind::Unknown
    }
}

fn url(ip: Ipv4Addr, port: u16) -> String {
    format!("http://{ip}:{port}")
}

/// Addresses for the actual listener port. LAN candidates appear only when the
/// listener accepts other devices; this device is always the final entry.
pub(crate) fn endpoints(port: u16, network: bool) -> Vec<ShareEndpoint> {
    let mut result = Vec::new();
    if network {
        let candidates = if_addrs::get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|interface| match interface.ip() {
                std::net::IpAddr::V4(ip)
                    if !ip.is_loopback()
                        && !ip.is_unspecified()
                        && !ip.is_link_local()
                        && !ip.is_multicast()
                        && !ip.is_broadcast() =>
                {
                    Some(Candidate {
                        kind: classify(&interface),
                        up: interface.oper_status != if_addrs::IfOperStatus::Down
                            && interface.oper_status != if_addrs::IfOperStatus::NotPresent,
                        name: interface.name,
                        ip,
                    })
                }
                _ => None,
            })
            .collect();
        result.extend(
            ordered(candidates)
                .into_iter()
                .map(|candidate| ShareEndpoint {
                    url: url(candidate.ip, port),
                    interface: candidate.name,
                    kind: candidate.kind,
                }),
        );
    }
    result.push(ShareEndpoint {
        url: url(Ipv4Addr::LOCALHOST, port),
        interface: String::new(),
        kind: LinkKind::Loopback,
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, ip: [u8; 4], kind: LinkKind, up: bool) -> Candidate {
        Candidate {
            name: name.into(),
            ip: Ipv4Addr::from(ip),
            kind,
            up,
        }
    }

    fn names(candidates: Vec<Candidate>) -> Vec<String> {
        ordered(candidates)
            .into_iter()
            .map(|candidate| candidate.name)
            .collect()
    }

    #[test]
    fn physical_links_lead_and_virtual_links_never_displace_them() {
        let order = names(vec![
            candidate("docker0", [172, 17, 0, 1], LinkKind::Virtual, false),
            candidate("tailscale0", [100, 64, 0, 2], LinkKind::Virtual, true),
            candidate("enp3s0", [192, 168, 1, 20], LinkKind::Wired, true),
            candidate("mystery0", [10, 0, 0, 9], LinkKind::Unknown, true),
            candidate("wlan0", [192, 168, 1, 173], LinkKind::Wifi, true),
        ]);
        assert_eq!(
            order,
            ["wlan0", "enp3s0", "mystery0", "docker0", "tailscale0"]
        );
    }

    #[test]
    fn order_is_stable_across_enumeration_order_and_duplicate_addresses() {
        let forward = vec![
            candidate("wlan1", [192, 168, 2, 5], LinkKind::Wifi, true),
            candidate("wlan0", [192, 168, 1, 5], LinkKind::Wifi, true),
            candidate("wlan0", [192, 168, 1, 5], LinkKind::Wifi, true),
            candidate("eth0", [10, 0, 0, 2], LinkKind::Wired, false),
            candidate("br0", [192, 168, 1, 5], LinkKind::Virtual, true),
        ];
        let mut reversed = forward.clone();
        reversed.reverse();
        let expected = ordered(forward);
        assert_eq!(expected, ordered(reversed));
        assert_eq!(
            expected
                .iter()
                .map(|candidate| candidate.name.as_str())
                .collect::<Vec<_>>(),
            ["wlan0", "wlan1", "eth0"],
            "a disconnected cable follows working links; a repeated address keeps its best link"
        );
    }

    #[test]
    fn classification_uses_platform_facts_and_stays_unknown_without_them() {
        let temp = tempfile::tempdir().unwrap();
        let net = temp.path();
        let make = |name: &str, kind: &str, parts: &[&str]| {
            let entry = net.join(name);
            std::fs::create_dir(&entry).unwrap();
            std::fs::write(entry.join("type"), kind).unwrap();
            for part in parts {
                std::fs::create_dir(entry.join(part)).unwrap();
            }
        };
        make("wlan0", "1\n", &["wireless", "device"]);
        make("enp3s0", "1\n", &["device"]);
        make("docker0", "1\n", &["bridge"]);
        make("tun0", "65534\n", &[]);
        make("usb0", "803\n", &["device"]);
        assert_eq!(classify_in(net, "wlan0"), LinkKind::Wifi);
        assert_eq!(classify_in(net, "enp3s0"), LinkKind::Wired);
        assert_eq!(classify_in(net, "docker0"), LinkKind::Virtual);
        assert_eq!(classify_in(net, "tun0"), LinkKind::Virtual);
        assert_eq!(classify_in(net, "usb0"), LinkKind::Unknown);
        // A Wi-Fi-looking name without platform facts is not called Wi-Fi.
        assert_eq!(classify_in(net, "wlan9"), LinkKind::Unknown);
        assert_eq!(classify_in(net, "../wlan0"), LinkKind::Unknown);
        assert_eq!(classify_in(net, ""), LinkKind::Unknown);
    }

    #[test]
    fn local_only_sharing_lists_this_device_alone() {
        let local = endpoints(4321, false);
        assert_eq!(
            local,
            [ShareEndpoint {
                url: "http://127.0.0.1:4321".into(),
                interface: String::new(),
                kind: LinkKind::Loopback,
            }]
        );
        let shared = endpoints(4321, true);
        assert_eq!(shared.last(), local.last());
        assert!(shared
            .iter()
            .all(|endpoint| endpoint.url.ends_with(":4321")));
    }
}
