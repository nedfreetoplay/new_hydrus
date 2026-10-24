//! Shared library for Hydrus external tools.
//!
//! This crate is the Rust port of `hydrus_python/hydrus/external`. It contains
//! the standalone parsers and helpers that do not depend on a client or server
//! controller:
//!
//! * [`blurhash`] — blurhash encoding/decoding helpers;
//! * [`hexagonitswfheader`] — SWF header parsing;
//! * [`logic_expression_query_parser`] — boolean tag query parsing;
//! * [`system_predicate_parser`] — `system:` predicate parsing.

pub mod blurhash;
pub mod hexagonitswfheader;
pub mod logic_expression_query_parser;
pub mod system_predicate_parser;
