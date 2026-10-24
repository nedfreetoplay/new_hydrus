//! Client-side image metadata flags from `ClientImageMetadata.py`.
//!
//! The heavy parsing already lives in `hydrus_core`; this layer exposes the
//! flag set used by `FileImportJob` and media results.

use std::path::Path;

use hydrus_core::files::images::hydrus_image_metadata::{self as core_metadata, ImageMetadata};
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageMetadataFlags {
    pub has_exif: bool,
    pub has_human_readable_embedded_metadata: bool,
    pub has_icc_profile: bool,
    pub has_software_source: bool,
    pub has_iptc: bool,
    pub has_xmp: bool,
}

impl ImageMetadataFlags {
    pub fn from_metadata(metadata: &ImageMetadata) -> Self {
        let has_iptc = metadata.info.keys().any(|key| {
            let key = key.to_lowercase();
            key.contains("photoshop") || key.contains("iptc")
        });
        let has_xmp = metadata.info.keys().any(|key| {
            let key = key.to_lowercase();
            key.contains("xmp") || key.contains("adobe.xmp")
        });

        Self {
            has_exif: metadata.has_exif(),
            has_human_readable_embedded_metadata: metadata.has_human_readable_embedded_metadata(),
            has_icc_profile: metadata.has_icc_profile(),
            has_software_source: metadata.has_software_source(),
            has_iptc,
            has_xmp,
        }
    }

    pub fn any(&self) -> bool {
        self.has_exif
            || self.has_human_readable_embedded_metadata
            || self.has_icc_profile
            || self.has_software_source
            || self.has_iptc
            || self.has_xmp
    }
}

/// Reads metadata for formats that the core parser supports.
pub fn read_image_metadata(path: impl AsRef<Path>, mime: Mime) -> HydrusResult<ImageMetadata> {
    match mime {
        Mime::Png => core_metadata::read_png_metadata(path),
        _ => Err(HydrusException::new(
            ErrorKind::UnsupportedFile,
            format!("metadata parsing is not implemented for {}", mime.as_str()),
        )),
    }
}

pub fn get_metadata_flags(path: impl AsRef<Path>, mime: Mime) -> HydrusResult<ImageMetadataFlags> {
    read_image_metadata(path, mime).map(|metadata| ImageMetadataFlags::from_metadata(&metadata))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydrus_client_metadata_{}_{}.png",
            std::process::id(),
            rand::random::<u64>()
        ));
        RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        path
    }

    #[test]
    fn plain_png_has_no_metadata_flags() {
        let path = png();
        let flags = get_metadata_flags(&path, Mime::Png).unwrap();

        assert!(!flags.has_exif);
        assert!(!flags.has_icc_profile);
        assert!(!flags.any());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unsupported_formats_are_rejected() {
        let path = png();
        assert!(get_metadata_flags(&path, Mime::Jpeg).is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn flags_are_computed_from_maps() {
        let mut metadata = core_metadata::read_png_metadata(png()).unwrap();
        metadata.info.insert(
            "XML:com.adobe.xmp".to_string(),
            core_metadata::MetadataValue::Text("<x:xmpmeta/>".to_string()),
        );

        let flags = ImageMetadataFlags::from_metadata(&metadata);
        assert!(flags.has_xmp);
        assert!(flags.any());
    }
}
