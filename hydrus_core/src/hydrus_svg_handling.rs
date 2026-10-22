//! Rust port of `hydrus/core/files/HydrusSVGHandling.py`.
//!
//! Python deliberately does not render or measure SVGs yet; both helpers raise
//! so the caller can fall back to a generic thumbnail. The port keeps that
//! behaviour.

use std::path::Path;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `generate_thumbnail_numpy_from_svg_path( path, target_resolution )`
pub fn generate_thumbnail_numpy_from_svg_path(
    _path: impl AsRef<Path>,
    _target_resolution: (u32, u32),
) -> HydrusResult<Vec<u8>> {
    Err(HydrusException::new(ErrorKind::NoThumbnailFile, ""))
}

/// `get_svg_resolution( path )`
pub fn get_svg_resolution(_path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    Err(HydrusException::new(ErrorKind::NoResolutionFile, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_helpers_preserve_the_python_fallbacks() {
        let thumbnail = generate_thumbnail_numpy_from_svg_path("x.svg", (100, 100)).unwrap_err();
        assert!(thumbnail.is_a(ErrorKind::NoThumbnailFile));

        let resolution = get_svg_resolution("x.svg").unwrap_err();
        assert!(resolution.is_a(ErrorKind::NoResolutionFile));
    }
}
