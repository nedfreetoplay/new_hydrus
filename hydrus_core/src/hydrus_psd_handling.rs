//! Rust port of `hydrus/core/files/HydrusPSDHandling.py`.
//!
//! Photoshop documents have a simple fixed header followed by sections of
//! length-prefixed image resources. Hydrus reads the canvas size directly and
//! asks FFMPEG to render the embedded preview for thumbnails.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use image::DynamicImage;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_ffmpeg_rendering;
use crate::hydrus_image_handling;

/// `( resource_id, size, data_offset )`
pub type PsdImageResource = (u16, u32, u64);

fn read_uint16(reader: &mut impl Read) -> HydrusResult<u16> {
    let mut bytes = [0_u8; 2];

    reader.read_exact(&mut bytes).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })?;

    Ok(u16::from_be_bytes(bytes))
}

fn read_uint32(reader: &mut impl Read) -> HydrusResult<u32> {
    let mut bytes = [0_u8; 4];

    reader.read_exact(&mut bytes).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })?;

    Ok(u32::from_be_bytes(bytes))
}

/// `get_psd_image_resources( path )`
pub fn get_psd_image_resources(path: impl AsRef<Path>) -> HydrusResult<Vec<PsdImageResource>> {
    let path = path.as_ref();

    let mut file = std::fs::File::open(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not open the PSD file at \"{}\": {error}", path.display()),
        )
    })?;

    // PSD Header: 26 bytes.
    file.seek(SeekFrom::Start(26)).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })?;

    // Color Mode Data section.
    let color_mode_data_len = read_uint32(&mut file)?;

    file.seek(SeekFrom::Current(i64::from(color_mode_data_len)))
        .map_err(|_| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

    let image_resources_len = read_uint32(&mut file)?;

    let image_resources_start = file.stream_position().map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })?;

    let image_resources_end = image_resources_start
        .checked_add(u64::from(image_resources_len))
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

    let mut resources = Vec::new();

    while file.stream_position().map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })? < image_resources_end
    {
        let mut signature = [0_u8; 4];

        file.read_exact(&mut signature).map_err(|_| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

        if &signature != b"8BIM" {
            return Err(HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Invalid PSD image resource signature!",
            ));
        }

        let resource_id = read_uint16(&mut file)?;

        let mut name_len_byte = [0_u8; 1];
        file.read_exact(&mut name_len_byte).map_err(|_| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

        let name_len = name_len_byte[0] as usize;

        let mut name_bytes = vec![0_u8; name_len];
        file.read_exact(&mut name_bytes).map_err(|_| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

        if (name_len + 1) % 2 == 1 {
            let mut padding = [0_u8; 1];
            file.read_exact(&mut padding).map_err(|_| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    "Could not read this PSD file!",
                )
            })?;
        }

        let size = read_uint32(&mut file)?;

        let data_offset = file.stream_position().map_err(|_| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not read this PSD file!",
            )
        })?;

        resources.push((resource_id, size, data_offset));

        let padded_size = u64::from(size) + u64::from(size % 2);

        file.seek(SeekFrom::Current(padded_size as i64))
            .map_err(|_| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    "Could not read this PSD file!",
                )
            })?;
    }

    Ok(resources)
}

/// `get_psd_image_resource_ids( image_resources )`
pub fn get_psd_image_resource_ids(
    image_resources: &[PsdImageResource],
) -> std::collections::HashSet<u16> {
    image_resources
        .iter()
        .map(|(resource_id, _, _)| *resource_id)
        .collect()
}

/// `parse_ffmpeg_psd_line( lines )`
pub fn parse_ffmpeg_psd_line(lines: &[String]) -> HydrusResult<String> {
    lines
        .iter()
        .find(|line| {
            line.trim_start().starts_with("Stream") && line.contains("Video: ") && line.contains("psd")
        })
        .cloned()
        .ok_or_else(|| {
            HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not find PSD information!",
            )
        })
}

/// `parse_ffmpeg_psd_resolution( lines )`
pub fn parse_ffmpeg_psd_resolution(lines: &[String]) -> HydrusResult<(u32, u32)> {
    let line = parse_ffmpeg_psd_line(lines)?;

    let regex = regex::Regex::new(r" ([0-9]+)x([0-9]+)[, ]").expect("the PSD resolution regex is valid");

    let Some(captures) = regex.captures(&line) else {
        return Err(HydrusException::new(
            ErrorKind::NoResolutionFile,
            "Error parsing resolution!",
        ));
    };

    let width = captures[1]
        .parse::<u32>()
        .map_err(|_| HydrusException::new(ErrorKind::NoResolutionFile, "Error parsing resolution!"))?;
    let height = captures[2]
        .parse::<u32>()
        .map_err(|_| HydrusException::new(ErrorKind::NoResolutionFile, "Error parsing resolution!"))?;

    Ok((width, height))
}

/// `psd_has_icc_profile( path )`
pub fn psd_has_icc_profile(path: impl AsRef<Path>) -> HydrusResult<bool> {
    let image_resources = get_psd_image_resources(path)?;

    Ok(get_psd_image_resource_ids(&image_resources).contains(&1039))
}

/// `generate_pil_image_from_psd( path )`
pub fn generate_pil_image_from_psd(path: impl AsRef<Path>) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    let png_bytes = hydrus_ffmpeg_rendering::render_image_to_png_bytes(path)?;

    if png_bytes.is_empty() {
        return Err(HydrusException::new(
            ErrorKind::NoRenderFile,
            "This PSD has no embedded Preview file that FFMPEG can read!",
        ));
    }

    hydrus_image_handling::decode_image_bytes(&png_bytes)
}

/// `generate_thumbnail_numpy_from_psd_path( path, target_resolution )`
pub fn generate_thumbnail_numpy_from_psd_path(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let image = match generate_pil_image_from_psd(path) {
        Ok(image) => image,
        Err(error) if error.is_a(ErrorKind::LimitedSupportFile) => {
            return Err(HydrusException::new(
                ErrorKind::NoThumbnailFile,
                error.to_string(),
            ));
        }
        Err(error) => return Err(error),
    };

    Ok(hydrus_image_handling::resize_image(
        &image,
        target_resolution,
        None,
    ))
}

/// `get_psd_resolution( path )`
pub fn get_psd_resolution(path: impl AsRef<Path>) -> HydrusResult<(u32, u32)> {
    let path = path.as_ref();

    let mut file = std::fs::File::open(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not open the PSD file at \"{}\": {error}", path.display()),
        )
    })?;

    file.seek(SeekFrom::Start(14)).map_err(|_| {
        HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not read this PSD file!",
        )
    })?;

    let height = read_uint32(&mut file)?;
    let width = read_uint32(&mut file)?;

    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn unique_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_psd_{}_{}_{}.psd",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn resolution_is_read_from_the_psd_header() {
        let path = unique_path("resolution");

        let mut bytes = vec![0_u8; 26];
        bytes[14..18].copy_from_slice(&480_u32.to_be_bytes());
        bytes[18..22].copy_from_slice(&640_u32.to_be_bytes());

        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&bytes).unwrap();

        assert_eq!(get_psd_resolution(&path).unwrap(), (640, 480));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn image_resources_and_icc_profile_are_detected() {
        let path = unique_path("resources");

        let mut bytes = vec![0_u8; 26];

        // No colour mode data.
        bytes.extend_from_slice(&0_u32.to_be_bytes());

        let mut resources = Vec::new();

        resources.extend_from_slice(b"8BIM");
        resources.extend_from_slice(&1039_u16.to_be_bytes());
        resources.push(0); // Empty Pascal name.
        resources.extend_from_slice(&4_u32.to_be_bytes());
        resources.extend_from_slice(b"test");

        bytes.extend_from_slice(&(resources.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&resources);

        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&bytes).unwrap();

        assert!(psd_has_icc_profile(&path).unwrap());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn ffmpeg_psd_resolution_is_parsed() {
        let lines = vec!["Stream #0:0: Video: psd, rgb24, 1920x1080,".to_string()];

        assert_eq!(parse_ffmpeg_psd_resolution(&lines).unwrap(), (1920, 1080));
    }
}
