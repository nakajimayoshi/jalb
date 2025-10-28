use std::{collections::HashSet, net::IpAddr};
use std::fmt::{Display, Formatter};
use std::process::exit;
use serde::Deserialize;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use log::error;
use url::quirks::password;
use crate::config::NetworkTarget;

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2.hash_password(password.as_bytes(), &salt).expect("failed to hash password").to_string()
}


pub enum AuthorizationType {
    BasicUserAuth(String, String),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Security {
    ip_whitelist: HashSet<IpAddr>,
    ip_blacklist: HashSet<IpAddr>,
    user: Option<String>,
    password_hash: Option<String>,
}

impl Security {
    pub fn new(auth: AuthorizationType) -> Self {

        match auth {
            AuthorizationType::BasicUserAuth(user, password) => {
                let password_hash = hash_password(&password);

                 Self {
                    ip_blacklist: HashSet::new(),
                    ip_whitelist: HashSet::new(),
                    user: Some(user),
                    password_hash: Some(password_hash),
                }
            }
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(hash) = &self.password_hash {
            return hash == hash_password(&password).as_str();
        }

        false
    }

    pub fn is_blacklisted(&self, ip: &IpAddr) -> bool {
        if self.ip_blacklist.contains(ip) {
            return true;
        }

        false
    }

    pub fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        if self.ip_whitelist.is_empty() {
            return true;
        }

        if self.ip_whitelist.contains(ip) {
            return true;
        }

        false
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

impl Default for Security {
    fn default() -> Self {
        const DEFAULT_USER: &'static str = "admin";
        const DEFAULT_PASSWORD: &'static str = "jalb";
        Self::new(AuthorizationType::BasicUserAuth(DEFAULT_USER.to_string(), DEFAULT_PASSWORD.to_string()))
    }
}

#[cfg(test)]
mod test {

    use std::{net::Ipv4Addr, str::FromStr};

    use super::*;

    #[test]
    fn test_ip_filter_basic() {
        let mut filter = Security::default();
        filter.add_to_whitelist(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        let allowed_ip: IpAddr = "127.0.0.1".parse().unwrap();
        let disallowed_ip: IpAddr = "192.168.2.11".parse().unwrap();

        assert!(filter.is_blacklisted(&allowed_ip));
        assert!(!filter.is_blacklisted(&disallowed_ip));
    }

    #[test]
    fn test_ip_filter_no_whitelist() {
        let mut filter = Security::default();

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
            assert!(filter.is_blacklisted(&ip))
        }

        let disallowed_ip = IpAddr::V4(Ipv4Addr::new(168, 11, 12, 15));
        assert!(!filter.is_blacklisted(&disallowed_ip));
    }
}
