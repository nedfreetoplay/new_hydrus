//! A port of `hydrus.core.networking.HydrusNetworkLegacy`.
//!
//! The Python module exists only to upgrade the old `( key, title,
//! dictionary_string )` account-type triple. That upgrade lives next to the
//! other network serialisable types, so this module re-exports it under the
//! historical Python name.
//!
//! ```ignore
//! let account_type = convert_to_new_account_type( key, title, dictionary_string )?;
//! ```

pub use crate::networking::hydrus_network::convert_legacy_account_type as convert_to_new_account_type;
