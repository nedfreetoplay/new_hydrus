//! Rust port of `hydrus/core/HydrusSessions.py`.
//!
//! The Python class stores bytes keys and calls back into `HG.controller` for
//! database reads/writes and pub/sub. The Rust port keeps the same shape but
//! makes those dependencies explicit:
//!
//! * [`Account`] is the duck-typed account object (`get_account_key`,
//!   `IsDirty`).
//! * [`SessionController`] is the small slice of the controller the manager
//!   needs: reading accounts/sessions, writing a new session, subscribing to
//!   the refresh topics, and the server-busy check.
//!
//! Note that Python's refresh code checks the *outer* service-key dictionary
//! when deciding whether to cache an account or hashed access key (rather than
//! the per-service inner dictionary). That quirk is preserved here, with a
//! comment, so a future behaviour-compatibility check has an obvious place to
//! start.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::hydrus_data::generate_key;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_pubsub::{PubSubMessage, Subscriber};
use crate::hydrus_time::{get_now, time_has_passed};

/// `HYDRUS_SESSION_LIFETIME`
pub const HYDRUS_SESSION_LIFETIME: i64 = 30 * 86400;

pub type AccountKey = Vec<u8>;
pub type ServiceKey = Vec<u8>;
pub type SessionKey = Vec<u8>;

/// The bits of Python's `Account` this module uses.
pub trait Account: Send + Sync {
    fn get_account_key(&self) -> AccountKey;

    /// Python spelling: `IsDirty()`.
    fn is_dirty(&self) -> bool;
}

/// One row from `read( 'sessions' )`, in Python's tuple order.
#[derive(Clone)]
pub struct ExistingSession {
    pub session_key: SessionKey,
    pub service_key: ServiceKey,
    pub account: Arc<dyn Account>,
    pub hashed_access_key: Vec<u8>,
    pub expires: i64,
}

/// The controller operations `HydrusSessionManagerServer` performs.
///
/// The Rust port passes keys explicitly instead of using Python's variadic
/// `controller.read( action, *args )`.
pub trait SessionController: Send + Sync {
    fn read_sessions(&self, service_key: Option<&[u8]>) -> HydrusResult<Vec<ExistingSession>>;

    fn read_account(
        &self,
        service_key: &[u8],
        account_key: &[u8],
    ) -> HydrusResult<Arc<dyn Account>>;

    fn read_account_key_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<AccountKey>;

    fn write_session(
        &self,
        session_key: &[u8],
        service_key: &[u8],
        account_key: &[u8],
        expires: i64,
    ) -> HydrusResult<()>;

    fn subscribe(&self, subscriber: Arc<dyn Subscriber>, method_name: &str, topic: &str);

    /// `HG.server_busy.locked()`. The server overrides this; the default is
    /// "not busy".
    fn server_busy(&self) -> bool {
        false
    }
}

#[derive(Default)]
struct SessionState {
    service_keys_to_session_keys_to_sessions:
        HashMap<ServiceKey, HashMap<SessionKey, (AccountKey, i64)>>,
    service_keys_to_account_keys_to_accounts:
        HashMap<ServiceKey, HashMap<AccountKey, Arc<dyn Account>>>,
    service_keys_to_hashed_access_keys_to_account_keys:
        HashMap<ServiceKey, HashMap<Vec<u8>, AccountKey>>,
}

/// `HydrusSessionManagerServer`
pub struct HydrusSessionManagerServer {
    controller: Arc<dyn SessionController>,
    state: Mutex<SessionState>,
}

fn session_error() -> HydrusException {
    HydrusException::new(
        ErrorKind::Session,
        "Did not find that session! Try again!",
    )
}

fn server_busy_error(message: &str) -> HydrusException {
    HydrusException::new(ErrorKind::ServerBusy, message)
}

impl HydrusSessionManagerServer {
    /// `__init__`: refreshes every account, then subscribes to the two refresh
    /// topics.
    pub fn new(controller: Arc<dyn SessionController>) -> HydrusResult<Arc<Self>> {
        let manager = Arc::new(Self {
            controller,
            state: Mutex::new(SessionState::default()),
        });

        manager.refresh_all_accounts(None)?;

        manager.controller.subscribe(
            Arc::clone(&manager) as Arc<dyn Subscriber>,
            "RefreshAccounts",
            "update_session_accounts",
        );

        manager.controller.subscribe(
            Arc::clone(&manager) as Arc<dyn Subscriber>,
            "RefreshAllAccounts",
            "update_all_session_accounts",
        );

        Ok(manager)
    }

    fn lock(&self) -> MutexGuard<'_, SessionState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn get_account_from_account_key(
        &self,
        state: &mut SessionState,
        service_key: &[u8],
        account_key: &[u8],
    ) -> HydrusResult<Arc<dyn Account>> {
        let account_keys_to_accounts = state
            .service_keys_to_account_keys_to_accounts
            .entry(service_key.to_vec())
            .or_default();

        if let Some(account) = account_keys_to_accounts.get(account_key) {
            return Ok(Arc::clone(account));
        }

        if self.controller.server_busy() {
            return Err(server_busy_error(
                "Sorry, server is busy and cannot fetch account data right now!",
            ));
        }

        let account = self.controller.read_account(service_key, account_key)?;

        account_keys_to_accounts.insert(account_key.to_vec(), Arc::clone(&account));

        Ok(account)
    }

    fn get_account_key_from_access_key(
        &self,
        state: &mut SessionState,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<AccountKey> {
        let hashed_access_key = Sha256::digest(access_key).to_vec();

        let hashed_access_keys_to_account_keys = state
            .service_keys_to_hashed_access_keys_to_account_keys
            .entry(service_key.to_vec())
            .or_default();

        if let Some(account_key) = hashed_access_keys_to_account_keys.get(&hashed_access_key) {
            return Ok(account_key.clone());
        }

        if self.controller.server_busy() {
            return Err(server_busy_error(
                "Sorry, server is busy and cannot fetch account id data right now!",
            ));
        }

        let account_key = self
            .controller
            .read_account_key_from_access_key(service_key, access_key)?;

        hashed_access_keys_to_account_keys.insert(hashed_access_key, account_key.clone());

        Ok(account_key)
    }

    /// `add_session( service_key, access_key )`
    pub fn add_session(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<(SessionKey, i64)> {
        let mut state = self.lock();

        let account_key = self.get_account_key_from_access_key(&mut state, service_key, access_key)?;

        // Python fetches the account here, which also validates it exists.
        self.get_account_from_account_key(&mut state, service_key, &account_key)?;

        let session_key = generate_key().to_vec();

        let expires = get_now() + HYDRUS_SESSION_LIFETIME;

        self.controller
            .write_session(&session_key, service_key, &account_key, expires)?;

        state
            .service_keys_to_session_keys_to_sessions
            .entry(service_key.to_vec())
            .or_default()
            .insert(session_key.clone(), (account_key, expires));

        Ok((session_key, expires))
    }

    /// `get_account( service_key, session_key )`
    pub fn get_account(
        &self,
        service_key: &[u8],
        session_key: &[u8],
    ) -> HydrusResult<Arc<dyn Account>> {
        let mut state = self.lock();

        let Some(session_keys_to_sessions) = state
            .service_keys_to_session_keys_to_sessions
            .get_mut(service_key)
        else {
            return Err(session_error());
        };

        let Some((account_key, expires)) = session_keys_to_sessions.get(session_key).cloned() else {
            return Err(session_error());
        };

        if time_has_passed(Some(expires)) {
            session_keys_to_sessions.remove(session_key);
        } else {
            let accounts = state
                .service_keys_to_account_keys_to_accounts
                .entry(service_key.to_vec())
                .or_default();

            if let Some(account) = accounts.get(&account_key) {
                return Ok(Arc::clone(account));
            }

            // Python would raise a bare KeyError here. A Session exception is
            // friendlier and lets callers treat it as "that session is gone".
            return Err(session_error());
        }

        Err(session_error())
    }

    /// `get_account_from_access_key( service_key, access_key )`
    pub fn get_account_from_access_key(
        &self,
        service_key: &[u8],
        access_key: &[u8],
    ) -> HydrusResult<Arc<dyn Account>> {
        let mut state = self.lock();

        let account_key = self.get_account_key_from_access_key(&mut state, service_key, access_key)?;

        self.get_account_from_account_key(&mut state, service_key, &account_key)
    }

    /// `get_dirty_accounts()`
    pub fn get_dirty_accounts(&self) -> HashMap<ServiceKey, Vec<Arc<dyn Account>>> {
        let state = self.lock();

        let mut service_keys_to_dirty_accounts = HashMap::new();

        for (service_key, account_keys_to_accounts) in
            &state.service_keys_to_account_keys_to_accounts
        {
            let dirty_accounts: Vec<Arc<dyn Account>> = account_keys_to_accounts
                .values()
                .filter(|account| account.is_dirty())
                .cloned()
                .collect();

            if !dirty_accounts.is_empty() {
                service_keys_to_dirty_accounts.insert(service_key.clone(), dirty_accounts);
            }
        }

        service_keys_to_dirty_accounts
    }

    /// `refresh_accounts( service_key, account_keys = None )`
    pub fn refresh_accounts(
        &self,
        service_key: &[u8],
        account_keys: Option<&[AccountKey]>,
    ) -> HydrusResult<()> {
        let mut state = self.lock();

        let keys_to_refresh: Vec<AccountKey> = match account_keys {
            Some(account_keys) => account_keys.to_vec(),
            None => state
                .service_keys_to_account_keys_to_accounts
                .get(service_key)
                .map(|accounts| accounts.keys().cloned().collect())
                .unwrap_or_default(),
        };

        for account_key in keys_to_refresh {
            let account = self.controller.read_account(service_key, &account_key)?;

            state
                .service_keys_to_account_keys_to_accounts
                .entry(service_key.to_vec())
                .or_default()
                .insert(account_key, account);
        }

        Ok(())
    }

    /// `refresh_all_accounts( service_key = None )`
    pub fn refresh_all_accounts(&self, service_key: Option<&[u8]>) -> HydrusResult<()> {
        let mut state = self.lock();

        let existing_sessions = match service_key {
            None => {
                state.service_keys_to_session_keys_to_sessions.clear();
                state.service_keys_to_account_keys_to_accounts.clear();
                state
                    .service_keys_to_hashed_access_keys_to_account_keys
                    .clear();

                self.controller.read_sessions(None)?
            }
            Some(service_key) => {
                state
                    .service_keys_to_session_keys_to_sessions
                    .remove(service_key);
                state
                    .service_keys_to_account_keys_to_accounts
                    .remove(service_key);
                state
                    .service_keys_to_hashed_access_keys_to_account_keys
                    .remove(service_key);

                self.controller.read_sessions(Some(service_key))?
            }
        };

        for session in existing_sessions {
            let account_key = session.account.get_account_key();

            state
                .service_keys_to_session_keys_to_sessions
                .entry(session.service_key.clone())
                .or_default()
                .insert(session.session_key.clone(), (account_key.clone(), session.expires));

            // Python checks the *outer* map here. Preserve that so the port
            // remains a faithful reference for behaviour comparisons.
            if !state
                .service_keys_to_account_keys_to_accounts
                .contains_key(&account_key)
            {
                state
                    .service_keys_to_account_keys_to_accounts
                    .entry(session.service_key.clone())
                    .or_default()
                    .insert(account_key.clone(), session.account);
            }

            // Same outer-map quirk as above.
            if !state
                .service_keys_to_hashed_access_keys_to_account_keys
                .contains_key(&session.hashed_access_key)
            {
                state
                    .service_keys_to_hashed_access_keys_to_account_keys
                    .entry(session.service_key)
                    .or_default()
                    .insert(session.hashed_access_key, account_key);
            }
        }

        Ok(())
    }

    /// `update_accounts( service_key, accounts )`
    pub fn update_accounts(
        &self,
        service_key: &[u8],
        accounts: &[Arc<dyn Account>],
    ) {
        let mut state = self.lock();

        let account_keys_to_accounts = state
            .service_keys_to_account_keys_to_accounts
            .entry(service_key.to_vec())
            .or_default();

        for account in accounts {
            account_keys_to_accounts.insert(account.get_account_key(), Arc::clone(account));
        }
    }
}

fn value_to_key(value: &Value) -> Option<Vec<u8>> {
    if let Some(array) = value.as_array() {
        let mut key = Vec::with_capacity(array.len());

        for item in array {
            key.push(u8::try_from(item.as_u64()?).ok()?);
        }

        return Some(key);
    }

    if let Some(text) = value.as_str() {
        // Be liberal: serialised keys may arrive as hex or as plain text.
        if let Some(key) = crate::hydrus_data::hex_to_none_or_bytes(Some(text)) {
            if !key.is_empty() {
                return Some(key);
            }
        }

        return Some(text.as_bytes().to_vec());
    }

    None
}

fn message_service_key(message: &PubSubMessage) -> Option<Vec<u8>> {
    message
        .arg(0)
        .and_then(value_to_key)
        .or_else(|| message.kwarg("service_key").and_then(value_to_key))
}

impl Subscriber for HydrusSessionManagerServer {
    fn has_method(&self, method_name: &str) -> bool {
        matches!(method_name, "RefreshAccounts" | "RefreshAllAccounts")
    }

    fn call(&self, method_name: &str, message: &PubSubMessage) -> HydrusResult<()> {
        match method_name {
            "RefreshAccounts" => {
                let Some(service_key) = message_service_key(message) else {
                    return Err(HydrusException::new(
                        ErrorKind::Validation,
                        "update_session_accounts did not include a service key!",
                    ));
                };

                self.refresh_accounts(&service_key, None)
            }
            "RefreshAllAccounts" => self.refresh_all_accounts(message_service_key(message).as_deref()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestAccount {
        key: AccountKey,
        dirty: bool,
    }

    impl TestAccount {
        fn new(key: u8, dirty: bool) -> Arc<Self> {
            Arc::new(Self {
                key: vec![key; 32],
                dirty,
            })
        }
    }

    impl Account for TestAccount {
        fn get_account_key(&self) -> AccountKey {
            self.key.clone()
        }

        fn is_dirty(&self) -> bool {
            self.dirty
        }
    }

    #[derive(Default)]
    struct TestController {
        sessions: Mutex<Vec<ExistingSession>>,
        accounts: Mutex<HashMap<(Vec<u8>, Vec<u8>), Arc<dyn Account>>>,
        access_keys: Mutex<HashMap<(Vec<u8>, Vec<u8>), AccountKey>>,
        writes: Mutex<Vec<(SessionKey, ServiceKey, AccountKey, i64)>>,
        reads: AtomicUsize,
        busy: bool,
        subscribers: Mutex<Vec<(Arc<dyn Subscriber>, String, String)>>,
    }

    impl TestController {
        fn with_session(service_key: &[u8], account: Arc<dyn Account>, access_key: &[u8]) -> Self {
            let hashed_access_key = Sha256::digest(access_key).to_vec();
            let account_key = account.get_account_key();

            let controller = Self::default();

            controller.sessions.lock().unwrap().push(ExistingSession {
                session_key: vec![7; 32],
                service_key: service_key.to_vec(),
                account: Arc::clone(&account),
                hashed_access_key,
                expires: get_now() + 3600,
            });

            controller
                .accounts
                .lock()
                .unwrap()
                .insert((service_key.to_vec(), account_key), account);

            controller
                .access_keys
                .lock()
                .unwrap()
                .insert((service_key.to_vec(), access_key.to_vec()), vec![1; 32]);

            controller
        }
    }

    impl SessionController for TestController {
        fn read_sessions(
            &self,
            service_key: Option<&[u8]>,
        ) -> HydrusResult<Vec<ExistingSession>> {
            self.reads.fetch_add(1, Ordering::SeqCst);

            Ok(self
                .sessions
                .lock()
                .unwrap()
                .iter()
                .filter(|session| {
                    service_key.is_none_or(|key| session.service_key == key)
                })
                .cloned()
                .collect())
        }

        fn read_account(
            &self,
            service_key: &[u8],
            account_key: &[u8],
        ) -> HydrusResult<Arc<dyn Account>> {
            self.reads.fetch_add(1, Ordering::SeqCst);

            self.accounts
                .lock()
                .unwrap()
                .get(&(service_key.to_vec(), account_key.to_vec()))
                .cloned()
                .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "no account"))
        }

        fn read_account_key_from_access_key(
            &self,
            service_key: &[u8],
            access_key: &[u8],
        ) -> HydrusResult<AccountKey> {
            self.reads.fetch_add(1, Ordering::SeqCst);

            self.access_keys
                .lock()
                .unwrap()
                .get(&(service_key.to_vec(), access_key.to_vec()))
                .cloned()
                .ok_or_else(|| HydrusException::new(ErrorKind::DataMissing, "no access key"))
        }

        fn write_session(
            &self,
            session_key: &[u8],
            service_key: &[u8],
            account_key: &[u8],
            expires: i64,
        ) -> HydrusResult<()> {
            self.writes.lock().unwrap().push((
                session_key.to_vec(),
                service_key.to_vec(),
                account_key.to_vec(),
                expires,
            ));

            Ok(())
        }

        fn subscribe(&self, subscriber: Arc<dyn Subscriber>, method_name: &str, topic: &str) {
            self.subscribers.lock().unwrap().push((
                subscriber,
                method_name.to_string(),
                topic.to_string(),
            ));
        }

        fn server_busy(&self) -> bool {
            self.busy
        }
    }

    fn service_key() -> Vec<u8> {
        vec![3; 32]
    }

    #[test]
    fn new_refreshes_and_subscribes() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(1, false),
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        assert_eq!(controller.reads.load(Ordering::SeqCst), 1);

        let subscribers = controller.subscribers.lock().unwrap();
        assert_eq!(subscribers.len(), 2);
        assert_eq!(subscribers[0].1, "RefreshAccounts");
        assert_eq!(subscribers[0].2, "update_session_accounts");
        assert_eq!(subscribers[1].1, "RefreshAllAccounts");
        assert_eq!(subscribers[1].2, "update_all_session_accounts");

        drop(subscribers);

        // The account is cached, so getting it should not hit the controller.
        let account = manager
            .get_account_from_access_key(&service_key(), b"access")
            .unwrap();
        assert_eq!(account.get_account_key(), vec![1; 32]);
        assert_eq!(controller.reads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn add_session_writes_and_caches() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(2, false),
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        let (session_key, expires) = manager.add_session(&service_key(), b"access").unwrap();

        assert_eq!(session_key.len(), 32);
        assert!(expires >= get_now() + HYDRUS_SESSION_LIFETIME);

        assert_eq!(controller.writes.lock().unwrap().len(), 1);

        let account = manager.get_account(&service_key(), &session_key).unwrap();
        assert_eq!(account.get_account_key(), vec![2; 32]);
    }

    #[test]
    fn expired_session_is_forgotten() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(3, false),
            b"access",
        ));

        controller.sessions.lock().unwrap()[0].expires = get_now() - 1;

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        let error = manager
            .get_account(&service_key(), &vec![7; 32])
            .err()
            .unwrap();

        assert!(error.is_a(ErrorKind::Session));
    }

    #[test]
    fn unknown_access_key_is_an_error() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(4, false),
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        assert!(
            manager
                .get_account_from_access_key(&service_key(), b"wrong")
                .is_err()
        );
    }

    #[test]
    fn dirty_accounts_are_reported_per_service() {
        let account = TestAccount::new(5, true);
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            account,
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        let dirty = manager.get_dirty_accounts();

        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[&service_key()].len(), 1);
    }

    #[test]
    fn update_accounts_replaces_cached_account() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(6, false),
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        manager.update_accounts(&service_key(), &[TestAccount::new(6, true)]);

        assert_eq!(manager.get_dirty_accounts()[&service_key()].len(), 1);
    }

    #[test]
    fn subscriber_handles_refresh_topics() {
        let controller = Arc::new(TestController::with_session(
            &service_key(),
            TestAccount::new(7, false),
            b"access",
        ));

        let manager = HydrusSessionManagerServer::new(
            Arc::clone(&controller) as Arc<dyn SessionController>,
        )
        .unwrap();

        manager.refresh_all_accounts(Some(&service_key())).unwrap();

        let message = PubSubMessage::with_args(
            "update_session_accounts",
            vec![Value::from(
                service_key()
                    .iter()
                    .map(|byte| Value::from(*byte))
                    .collect::<Vec<_>>(),
            )],
        );

        assert!(manager.has_method("RefreshAccounts"));
        assert!(manager.has_method("RefreshAllAccounts"));
        assert!(!manager.has_method("something_else"));

        manager.call("RefreshAccounts", &message).unwrap();
        manager.call("RefreshAllAccounts", &message).unwrap();
    }
}
