//! Networking/login panel state from `ClientGUINetworking.py` and
//! `ClientGUILogin.py`.

use std::collections::BTreeMap;

use crate::networking::NetworkContext;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginState {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub remember: bool,
}

impl LoginState {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            username: String::new(),
            password: String::new(),
            remember: false,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.domain.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }

    pub fn clear_password(&mut self) {
        self.password.clear();
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkingPanelState {
    pub contexts: Vec<String>,
    pub selected_context: Option<usize>,
    pub headers: BTreeMap<String, String>,
    pub login: Option<LoginState>,
    pub bandwidth_rules_visible: bool,
}

impl NetworkingPanelState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_contexts(&mut self, contexts: impl IntoIterator<Item = String>) {
        self.contexts = contexts.into_iter().collect();
        self.selected_context = if self.contexts.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn add_context(&mut self, context: &NetworkContext) {
        let context = context.to_string();

        if !self.contexts.contains(&context) {
            self.contexts.push(context);
        }

        if self.selected_context.is_none() && !self.contexts.is_empty() {
            self.selected_context = Some(0);
        }
    }

    pub fn select_context(&mut self, index: usize) -> bool {
        if index < self.contexts.len() {
            self.selected_context = Some(index);
            true
        } else {
            false
        }
    }

    pub fn get_selected_context(&self) -> Option<&str> {
        self.selected_context
            .and_then(|index| self.contexts.get(index))
            .map(String::as_str)
    }

    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    pub fn remove_header(&mut self, name: &str) -> bool {
        self.headers.remove(name).is_some()
    }

    pub fn set_login(&mut self, login: LoginState) {
        self.login = Some(login);
    }

    pub fn clear_login(&mut self) {
        self.login = None;
    }

    pub fn can_submit_login(&self) -> bool {
        self.login.as_ref().is_some_and(LoginState::is_valid)
    }

    pub fn set_bandwidth_rules_visible(&mut self, visible: bool) {
        self.bandwidth_rules_visible = visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contexts_can_be_added_and_selected() {
        let mut panel = NetworkingPanelState::new();
        panel.add_context(&NetworkContext::for_domain("example.com"));
        panel.add_context(&NetworkContext::for_domain("example.com"));

        assert_eq!(panel.contexts.len(), 1);
        assert_eq!(panel.get_selected_context(), Some("domain example.com"));

        assert!(!panel.select_context(5));
    }

    #[test]
    fn headers_can_be_edited() {
        let mut panel = NetworkingPanelState::new();
        panel.set_header("User-Agent", "hydrus");

        assert_eq!(panel.headers["User-Agent"], "hydrus");
        assert!(panel.remove_header("User-Agent"));
        assert!(!panel.remove_header("User-Agent"));
    }

    #[test]
    fn login_requires_all_fields() {
        let mut panel = NetworkingPanelState::new();
        let mut login = LoginState::new("example.com");
        panel.set_login(login.clone());

        assert!(!panel.can_submit_login());

        login.username = "user".to_string();
        login.password = "pass".to_string();
        panel.set_login(login);

        assert!(panel.can_submit_login());
    }

    #[test]
    fn clearing_login_removes_state() {
        let mut panel = NetworkingPanelState::new();
        panel.set_login(LoginState::new("example.com"));
        panel.clear_login();

        assert!(!panel.can_submit_login());
    }
}
