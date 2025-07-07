use std::{collections::HashSet, net::IpAddr};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Security {
    ip_whitelist: HashSet<IpAddr>,
    ip_blacklist: HashSet<IpAddr>,
}

impl Security {
    pub fn new() -> Self {
        Security {
            ip_blacklist: HashSet::new(),
            ip_whitelist: HashSet::new(),
        }
    }

    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        if self.ip_blacklist.contains(ip) {
            return false;
        }

        if !self.ip_whitelist.is_empty() {
            if !self.ip_whitelist.contains(ip) {
                return false;
            }
        }

        true
    }

    pub fn add_to_whitelist(&mut self, ip: IpAddr) {
        self.ip_whitelist.insert(ip);
    }

    pub fn add_to_blacklist(&mut self, ip: IpAddr) {
        self.ip_blacklist.insert(ip);
    }

    pub fn remove_from_whitelist(&mut self, ip: &IpAddr) {
        self.ip_whitelist.remove(ip);
    }

    pub fn remove_from_blacklist(&mut self, ip: &IpAddr) {
        self.ip_blacklist.remove(ip);
    }
}

#[cfg(test)]
mod test {

    use std::{net::Ipv4Addr, str::FromStr};

    use super::*;

    #[test]
    fn test_ip_filter_basic() {
        let mut filter = Security::new();
        filter.add_to_whitelist(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        let allowed_ip: IpAddr = "127.0.0.1".parse().unwrap();
        let disallowed_ip: IpAddr = "192.168.2.11".parse().unwrap();

        assert!(filter.is_allowed(&allowed_ip));
        assert!(!filter.is_allowed(&disallowed_ip));
    }

    #[test]
    fn test_ip_filter_no_whitelist() {
        let mut filter = Security::new();

        filter.add_to_blacklist("168.11.12.15".parse().unwrap());

        let allowed_ips: Vec<IpAddr> = vec![
            "168.10.12.15".parse().unwrap(),
            "168.13.12.15".parse().unwrap(),
            "168.14.12.15".parse().unwrap(),
            "112.10.12.55".parse().unwrap(),
            "148.10.15.15".parse().unwrap(),
            "158.10.12.15".parse().unwrap(),
            "168.10.125.5".parse().unwrap(),
            "158.10.12.15".parse().unwrap(),
            "92.10.12.15".parse().unwrap(),
            "127.0.0.1".parse().unwrap(),
        ];

        for ip in allowed_ips {
            assert!(filter.is_allowed(&ip))
        }

        let disallowed_ip = IpAddr::V4(Ipv4Addr::new(168, 11, 12, 15));
        assert!(filter.is_allowed(&disallowed_ip));
    }
}
