//! Session containers from `hydrus/client/networking/ClientNetworkingSessions.py`.
//!
//! The HTTP transport is added later; this keeps per-context cookie/header state
//! and the dirty tracking the persistence layer needs.

use std::collections::BTreeMap;

use crate::networking::NetworkContext;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkSession {
    pub headers: BTreeMap<String, String>,
    pub cookies: BTreeMap<String, String>,
    pub proxies: Vec<String>,
}

impl NetworkSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    pub fn set_cookie(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.cookies.insert(name.into(), value.into());
    }

    pub fn clear(&mut self) {
        self.headers.clear();
        self.cookies.clear();
        self.proxies.clear();
    }
}

#[derive(Debug, Default)]
pub struct NetworkSessionManager {
    sessions: BTreeMap<String, NetworkSession>,
    dirty: bool,
}

impl NetworkSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_session(&mut self, context: &NetworkContext) -> &mut NetworkSession {
        self.dirty = true;
        self.sessions
            .entry(context.to_string())
            .or_insert_with(NetworkSession::new)
    }

    pub fn get_session_for_domain(&mut self, domain: &str) -> &mut NetworkSession {
        let context = NetworkContext::for_domain(domain);
        self.get_session(&context)
    }

    pub fn clear_session(&mut self, context: &NetworkContext) {
        if let Some(session) = self.sessions.get_mut(&context.to_string()) {
            session.clear();
            self.dirty = true;
        }
    }

    pub fn get_network_contexts(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    pub fn has_session(&self, context: &NetworkContext) -> bool {
        self.sessions.contains_key(&context.to_string())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn set_clean(&mut self) {
        self.dirty = false;
    }

    /// `ReinitialiseSessions` — drop the transports but keep the context keys.
    pub fn reinitialise_sessions(&mut self) {
        for session in self.sessions.values_mut() {
            session.clear();
        }

        self.dirty = true;
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_are_keyed_by_context() {
        let mut manager = NetworkSessionManager::new();
        let context = NetworkContext::for_domain("example.com");

        manager.get_session(&context).set_cookie("a", "b");

        assert!(manager.has_session(&context));
        assert_eq!(manager.len(), 1);
        assert!(manager.is_dirty());
    }

    #[test]
    fn domain_helper_creates_domain_context() {
        let mut manager = NetworkSessionManager::new();
        manager
            .get_session_for_domain("example.com")
            .set_header("X", "1");

        assert!(manager.has_session(&NetworkContext::for_domain("example.com")));
    }

    #[test]
    fn clearing_a_session_removes_its_state() {
        let mut manager = NetworkSessionManager::new();
        let context = NetworkContext::for_domain("example.com");
        manager.get_session(&context).set_cookie("a", "b");

        manager.clear_session(&context);

        let session = manager.get_session(&context);
        assert!(session.cookies.is_empty());
    }

    #[test]
    fn dirty_state_can_be_cleared() {
        let mut manager = NetworkSessionManager::new();
        manager.get_session(&NetworkContext::default_context());

        assert!(manager.is_dirty());

        manager.set_clean();

        assert!(!manager.is_dirty());
    }

    #[test]
    fn reinitialising_keeps_contexts_but_clears_headers() {
        let mut manager = NetworkSessionManager::new();
        let context = NetworkContext::for_domain("example.com");
        manager.get_session(&context).set_header("X", "1");

        manager.reinitialise_sessions();

        assert_eq!(manager.len(), 1);
        assert!(manager.get_session(&context).headers.is_empty());
    }
}
