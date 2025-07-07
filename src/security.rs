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

    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn test_ip_filter() {
        let mut filter = Security::new();
        filter.add_to_whitelist(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }
}