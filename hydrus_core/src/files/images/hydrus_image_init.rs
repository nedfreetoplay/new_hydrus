//! Rust port of `hydrus/core/files/images/HydrusImageInit.py`.
//!
//! Python used this module to add missing PIL warning classes and to silence a
//! few noisy decoder warnings. The Rust `image` crate has no matching global
//! warning registry, and the hydrus image code explicitly checks for
//! decompression bombs itself, so the port keeps an idempotent initialisation
//! hook for API compatibility.

use std::sync::Once;

static INIT: Once = Once::new();

/// `warnings.simplefilter` and `warnings.filterwarnings` from the Python module.
///
/// This is a no-op in Rust. It exists so callers that used to import the Python
/// module for its side effects have a single obvious entry point.
pub fn initialise_image_library() {
    INIT.call_once(|| {
        // The image crate does not expose PIL's process-global warning filters.
        // Decoder limits are disabled explicitly at the decode call sites, and
        // bomb detection is performed by
        // `hydrus_image_handling::is_decompression_bomb`.
    });
}

/// Alias using the Python spelling, for call sites that mirror the old import.
pub fn initialize_image_library() {
    initialise_image_library();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialisation_is_idempotent() {
        initialise_image_library();
        initialize_image_library();
        initialise_image_library();
    }
}
