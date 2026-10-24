//! Login manager from `hydrus/client/networking/ClientNetworkingLogin.py`.

use std::collections::BTreeMap;

use crate::networking::client_networking_functions::convert_domain_into_all_applicable_domains;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLogin {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub access_key: Option<Vec<u8>>,
    pub expires: Option<i64>,
}

impl NetworkLogin {
    pub fn new(
        domain: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            domain: domain.into().to_lowercase(),
            username: username.into(),
            password: password.into(),
            access_key: None,
            expires: None,
        }
    }

    pub fn set_access_key(&mut self, access_key: Vec<u8>) {
        self.access_key = Some(access_key);
    }

    pub fn is_expired(&self, now: i64) -> bool {
        self.expires.is_some_and(|expires| expires <= now)
    }

    pub fn get_connection_string(&self) -> String {
        match &self.access_key {
            Some(access_key) => format!(
                "{}@{}",
                access_key
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                self.domain
            ),
            None => self.domain.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NetworkLoginManager {
    logins: BTreeMap<String, NetworkLogin>,
}

impl NetworkLoginManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_login(&mut self, login: NetworkLogin) {
        self.logins.insert(login.domain.clone(), login);
    }

    pub fn get_login(&self, domain: &str) -> Option<&NetworkLogin> {
        self.logins.get(&domain.to_lowercase())
    }

    pub fn get_login_mut(&mut self, domain: &str) -> Option<&mut NetworkLogin> {
        self.logins.get_mut(&domain.to_lowercase())
    }

    pub fn delete_login(&mut self, domain: &str) -> bool {
        self.logins.remove(&domain.to_lowercase()).is_some()
    }

    pub fn get_domains(&self) -> Vec<&str> {
        self.logins.keys().map(String::as_str).collect()
    }

    /// Finds the most specific login for a domain, walking up its labels.
    pub fn get_login_for_domain(&self, domain: &str) -> Option<&NetworkLogin> {
        for candidate in convert_domain_into_all_applicable_domains(domain, true) {
            if let Some(login) = self.logins.get(&candidate) {
                return Some(login);
            }
        }

        None
    }

    pub fn clear_expired(&mut self, now: i64) -> usize {
        let previous = self.logins.len();
        self.logins.retain(|_, login| !login.is_expired(now));
        previous - self.logins.len()
    }

    pub fn len(&self) -> usize {
        self.logins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.logins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logins_are_keyed_case_insensitively() {
        let mut manager = NetworkLoginManager::new();
        manager.set_login(NetworkLogin::new("Example.com", "user", "pass"));

        assert!(manager.get_login("example.com").is_some());
        assert!(manager.get_login("EXAMPLE.COM").is_some());
    }

    #[test]
    fn subdomains_use_parent_login() {
        let mut manager = NetworkLoginManager::new();
        manager.set_login(NetworkLogin::new("example.com", "user", "pass"));

        let login = manager.get_login_for_domain("a.b.example.com").unwrap();

        assert_eq!(login.username, "user");
    }

    #[test]
    fn access_keys_render_connection_string() {
        let mut login = NetworkLogin::new("example.com", "user", "pass");
        login.set_access_key(vec![0xab, 0xcd]);

        assert_eq!(login.get_connection_string(), "abcd@example.com");
    }

    #[test]
    fn expired_logins_are_cleared() {
        let mut manager = NetworkLoginManager::new();
        let mut login = NetworkLogin::new("example.com", "user", "pass");
        login.expires = Some(100);
        manager.set_login(login);

        assert_eq!(manager.clear_expired(100), 1);
        assert!(manager.is_empty());
    }

    #[test]
    fn deleting_logins_works() {
        let mut manager = NetworkLoginManager::new();
        manager.set_login(NetworkLogin::new("example.com", "user", "pass"));

        assert!(manager.delete_login("example.com"));
        assert!(!manager.delete_login("example.com"));
    }
}
