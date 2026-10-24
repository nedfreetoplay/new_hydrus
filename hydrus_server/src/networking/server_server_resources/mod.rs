//! Rust port of the first resource layer from
//! `hydrus/server/networking/ServerServerResources.py`.
//!
//! Twisted supplies the request transport and deferreds. This module ports the
//! resource policy and the simple network endpoints (busy, access keys,
//! sessions, shutdown, account/options/account-types/service-info). Repository
//! content resources are intentionally reported as not implemented until their
//! DB layer is ported.

include!("parts/part_01.rs");

include!("parts/part_02.rs");

include!("parts/part_03.rs");

include!("parts/part_04.rs");

include!("parts/part_05.rs");

include!("parts/part_06.rs");

include!("parts/part_07.rs");

#[cfg(test)]
mod tests;
