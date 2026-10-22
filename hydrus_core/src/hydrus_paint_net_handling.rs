//! Rust port of `hydrus/core/files/HydrusPaintNETHandling.py`.
//!
//! Paint.NET `.pdn` files begin with a length-prefixed UTF-8 XML header. The
//! header contains the canvas size and a base64-encoded PNG preview.

use std::io::Read;
use std::path::Path;

use base64::Engine as _;
use image::DynamicImage;
use roxmltree::Document;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_image_handling;

fn no_thumbnail(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::NoThumbnailFile, message)
}

/// `get_paint_net_xml_header( path )`
pub fn get_paint_net_xml_header(path: impl AsRef<Path>) -> HydrusResult<String> {
    let path = path.as_ref();

    let mut file = std::fs::File::open(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not open the Paint.NET file at \"{}\": {error}", path.display()),
        )
    })?;

    let mut signature = [0_u8; 4];
    file.read_exact(&mut signature).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot read the XML from this Paint.NET file!",
        )
    })?;

    let mut length_bytes = [0_u8; 4];
    file.read_exact(&mut length_bytes[..3]).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot read the XML from this Paint.NET file!",
        )
    })?;

    let header_length = i32::from_le_bytes(length_bytes);

    if header_length < 0 {
        return Err(HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot read the XML from this Paint.NET file!",
        ));
    }

    let mut header_bytes = vec![0_u8; header_length as usize];

    file.read_exact(&mut header_bytes).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot read the XML from this Paint.NET file!",
        )
    })?;

    String::from_utf8(header_bytes).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot read the XML from this Paint.NET file!",
        )
    })
}

/// `get_paint_net_resolution_from_xml_header( xml_header )`
pub fn get_paint_net_resolution_from_xml_header(xml_header: &str) -> HydrusResult<(u32, u32)> {
    let document = Document::parse(xml_header).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Cannot parse the XML from this Paint.NET file!",
        )
    })?;

    let root = document.root_element();

    let width = root
        .attribute("width")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Cannot parse the XML from this Paint.NET file!",
            )
        })?;

    let height = root
        .attribute("height")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Cannot parse the XML from this Paint.NET file!",
            )
        })?;

    Ok((width, height))
}

/// `get_paint_net_resolution( path )`
pub fn get_paint_net_resolution(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let xml_header = get_paint_net_xml_header(path)
        .map_err(|_| no_thumbnail("Could not read resolution bytes from this Paint.NET!"))?;

    get_paint_net_resolution_from_xml_header(&xml_header)
        .map_err(|_| no_thumbnail("Could not read resolution bytes from this Paint.NET!"))
}

/// `thumbnail_pil_image_from_paint_net( path )`
pub fn thumbnail_pil_image_from_paint_net(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let xml_header = get_paint_net_xml_header(path)
        .map_err(|_| no_thumbnail("Could not read thumb bytes from this Paint.NET xml!"))?;

    let document = Document::parse(&xml_header)
        .map_err(|_| no_thumbnail("Could not read thumb bytes from this Paint.NET xml!"))?;

    let thumb = document
        .descendants()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "thumb"
                && node
                    .parent()
                    .is_some_and(|parent| parent.tag_name().name() == "custom")
        })
        .ok_or_else(|| no_thumbnail("Could not read thumb bytes from this Paint.NET xml!"))?;

    let png_b64 = thumb
        .attribute("png")
        .ok_or_else(|| no_thumbnail("Could not read thumb bytes from this Paint.NET xml!"))?;

    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(png_b64)
        .map_err(|_| no_thumbnail("Could not decode thumb bytes from this Paint.NET xml!"))?;

    hydrus_image_handling::decode_image_bytes(&png_bytes)
}

/// `generate_thumbnail_numpy_from_paint_net( path, target_resolution )`
pub fn generate_thumbnail_numpy_from_paint_net(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let image = thumbnail_pil_image_from_paint_net(path)?;

    Ok(hydrus_image_handling::resize_image(
        &image,
        target_resolution,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use image::GenericImageView;

    use super::*;

    fn unique_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_paintnet_{}_{}_{}.pdn",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn pdn_with_header(header: &str) -> std::path::PathBuf {
        let path = unique_path("pdn");

        let mut file = std::fs::File::create(&path).unwrap();

        file.write_all(b"PDN3").unwrap();

        let length = header.len() as i32;
        let bytes = length.to_le_bytes();

        file.write_all(&bytes[..3]).unwrap();
        file.write_all(header.as_bytes()).unwrap();

        path
    }

    #[test]
    fn resolution_is_read_from_the_length_prefixed_header() {
        let path = pdn_with_header(r#"<custom width="320" height="200"><custom/></custom>"#);

        assert_eq!(get_paint_net_resolution(&path).unwrap(), (320, 200));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn thumbnail_is_decoded_from_base64_png() {
        let image = DynamicImage::new_rgba8(2, 1);
        let mut png_bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )
            .unwrap();

        let png_b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let header = format!(
            r#"<custom width="2" height="1"><custom><thumb png="{png_b64}"/></custom></custom>"#
        );

        let path = pdn_with_header(&header);

        let thumb = thumbnail_pil_image_from_paint_net(&path).unwrap();

        assert_eq!(thumb.dimensions(), (2, 1));

        std::fs::remove_file(path).ok();
    }
}
