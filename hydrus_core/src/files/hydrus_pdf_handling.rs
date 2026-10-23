//! Rust port of `hydrus/core/files/HydrusPDFHandling.py`.
//!
//! The core Python module is a capability stub: it signals that PDF rendering
//! and metadata extraction are not available, and the client supplies a richer
//! implementation. Keep the same error kinds.

use std::path::Path;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `base_generate_thumbnail_numpy_from_pdf_path( path, target_resolution )`
pub fn base_generate_thumbnail_numpy_from_pdf_path(
    _path: impl AsRef<Path>,
    _target_resolution: (u32, u32),
) -> HydrusResult<Vec<u8>> {
    Err(HydrusException::new(ErrorKind::NoThumbnailFile, ""))
}

/// `base_get_pdf_info( path )`
pub fn base_get_pdf_info(_path: impl AsRef<Path>) -> HydrusResult<()> {
    Err(HydrusException::new(ErrorKind::LimitedSupportFile, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_helpers_report_the_expected_capability_errors() {
        let thumbnail =
            base_generate_thumbnail_numpy_from_pdf_path("x.pdf", (100, 100)).unwrap_err();
        assert!(thumbnail.is_a(ErrorKind::NoThumbnailFile));

        let info = base_get_pdf_info("x.pdf").unwrap_err();
        assert!(info.is_a(ErrorKind::LimitedSupportFile));
    }
}
