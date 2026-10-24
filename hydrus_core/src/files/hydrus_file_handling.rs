//! Rust port of `hydrus/core/files/HydrusFileHandling.py`.
//!
//! This is the front door for imported files: it decides what a file is, hashes
//! it, reads the handful of properties hydrus stores, and renders a thumbnail.
//! The Python module is a collection of globals and dispatch chains; the Rust
//! port keeps the same order of preference but represents the eight-tuple as
//! [`FileInfo`] and keeps the default-thumbnail map behind a lock.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use image::DynamicImage;
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

use crate::files::hydrus_animation_handling;
use crate::files::hydrus_archive_handling;
use crate::files::hydrus_clip_handling;
use crate::files::hydrus_ffmpeg;
use crate::files::hydrus_ffmpeg_rendering;
use crate::files::hydrus_krita_handling;
use crate::files::hydrus_office_open_xml_handling;
use crate::files::hydrus_ole_handling;
use crate::files::hydrus_ora_handling;
use crate::files::hydrus_paint_net_handling;
use crate::files::hydrus_procreate_handling;
use crate::files::hydrus_psd_handling;
use crate::files::hydrus_svg_handling;
use crate::files::hydrus_ugoira_handling;
use crate::files::hydrus_video_handling;
use crate::files::images::hydrus_image_handling;
use crate::hydrus_constants::{
    ANIMATIONS, ARCHIVES, AUDIO, HEIF_TYPE_SEQUENCES, IMAGES, Mime, PIL_AVIF_MIMES, PIL_HEIF_MIMES,
    POTENTIAL_OFFICE_OLE_MIMES, VIDEO, allowed_mimes, mimes_that_definitely_have_audio,
};
use crate::hydrus_data;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_paths;
use crate::hydrus_serialisable;
use crate::hydrus_static_dir;
use crate::hydrus_temp;
use crate::hydrus_text;

/// Python loads this through `HydrusStaticDir.get_static_path`.
pub const DEFAULT_THUMBNAIL_FILENAME: &str = "hydrus.png";

/// The eight fields returned by Python's `get_file_info`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    pub size: u64,
    pub mime: Mime,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<i64>,
    pub num_frames: Option<u32>,
    pub has_audio: bool,
    pub num_words: Option<i64>,
}

/// The three extra hashes Python returns from `get_extra_hashes_from_path`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraHashes {
    pub md5: Vec<u8>,
    pub sha1: Vec<u8>,
    pub sha512: Vec<u8>,
}

/// One `(offsets, headers)` pair in Python's header table.
#[derive(Clone, Copy)]
pub struct OffsetAndHeaders {
    pub offsets: &'static [usize],
    pub headers: &'static [&'static [u8]],
}

/// One entry in Python's `headers_and_mime` table.
#[derive(Clone, Copy)]
pub struct HeaderMime {
    pub offsets_and_headers: &'static [OffsetAndHeaders],
    pub mime: Mime,
}

const fn oh(offsets: &'static [usize], headers: &'static [&'static [u8]]) -> OffsetAndHeaders {
    OffsetAndHeaders { offsets, headers }
}

const fn hm(offsets_and_headers: &'static [OffsetAndHeaders], mime: Mime) -> HeaderMime {
    HeaderMime {
        offsets_and_headers,
        mime,
    }
}

/// `headers_and_mime`
///
/// The ordering is significant: open-document ZIPs and EPUBs must be tested
/// before the generic ZIP signature, and animated formats are refined after
/// their generic header matches.
pub const HEADERS_AND_MIME: &[HeaderMime] = &[
    hm(&[oh(&[0], &[b"\xff\xd8"])], Mime::Jpeg),
    hm(&[oh(&[0], &[b"\x89PNG"])], Mime::UndeterminedPng),
    hm(&[oh(&[0], &[b"GIF87a", b"GIF89a"])], Mime::UndeterminedGif),
    hm(&[oh(&[8], &[b"WEBP"])], Mime::UndeterminedWebp),
    hm(&[oh(&[0], &[b"II*\x00", b"MM\x00*"])], Mime::Tiff),
    hm(&[oh(&[0], &[b"BM"])], Mime::Bmp),
    hm(
        &[oh(&[0], &[b"\x00\x00\x01\x00", b"\x00\x00\x02\x00"])],
        Mime::Icon,
    ),
    hm(&[oh(&[0], &[b"qoif"])], Mime::Qoi),
    hm(
        &[oh(
            &[0],
            &[b"\xff\x0a", b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a"],
        )],
        Mime::UndeterminedJxl,
    ),
    hm(&[oh(&[0], &[b"CWS", b"FWS", b"ZWS"])], Mime::Flash),
    hm(&[oh(&[0], &[b"FLV"])], Mime::Flv),
    hm(&[oh(&[0], &[b"%PDF"])], Mime::Pdf),
    hm(&[oh(&[0], &[b"8BPS\x00\x01", b"8BPS\x00\x02"])], Mime::Psd),
    hm(&[oh(&[0], &[b"CSFCHUNK"])], Mime::Clip),
    hm(&[oh(&[0], &[b"SAI-CANVAS"])], Mime::Sai2),
    hm(&[oh(&[0], &[b"gimp xcf "])], Mime::Xcf),
    // Important: this comes before ZIP because Krita files are ZIPs too.
    hm(
        &[oh(&[38, 42, 58, 63], &[b"application/x-krita"])],
        Mime::Krita,
    ),
    hm(
        &[oh(&[38, 42, 58, 63], &[b"image/openraster"])],
        Mime::OpenRaster,
    ),
    hm(&[oh(&[0], &[b"PDN3"])], Mime::PaintDotNet),
    hm(&[oh(&[38, 43], &[b"application/epub+zip"])], Mime::Epub),
    hm(
        &[
            oh(&[4], &[b"FORM"]),
            oh(&[12], &[b"DJVU", b"DJVM", b"PM44", b"BM44", b"SDJV"]),
        ],
        Mime::Djvu,
    ),
    hm(&[oh(&[0], &[b"{\\rtf"])], Mime::Rtf),
    hm(
        &[oh(&[0], &[b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08"])],
        Mime::Zip,
    ),
    hm(&[oh(&[0], &[b"7z\xbc\xaf\x27\x1c"])], Mime::SevenZ),
    hm(
        &[oh(
            &[0],
            &[
                b"\x52\x61\x72\x21\x1a\x07\x00",
                b"\x52\x61\x72\x21\x1a\x07\x01\x00",
            ],
        )],
        Mime::Rar,
    ),
    hm(&[oh(&[0], &[b"\x1f\x8b"])], Mime::Gzip),
    hm(&[oh(&[0], &[b"hydrus encrypted zip"])], Mime::EncryptedZip),
    hm(&[oh(&[4], &[b"ftypavif"])], Mime::Avif),
    hm(&[oh(&[4], &[b"ftypavis"])], Mime::AvifSequence),
    hm(
        &[oh(&[4], &[b"ftypmif1"]), oh(&[16, 20, 24], &[b"avif"])],
        Mime::Avif,
    ),
    hm(
        &[oh(
            &[4],
            &[b"ftypheic", b"ftypheix", b"ftypheim", b"ftypheis"],
        )],
        Mime::Heic,
    ),
    hm(
        &[oh(
            &[4],
            &[b"ftyphevc", b"ftyphevx", b"ftyphevm", b"ftyphevs"],
        )],
        Mime::HeicSequence,
    ),
    hm(&[oh(&[4], &[b"ftypmif1"])], Mime::Heif),
    hm(&[oh(&[4], &[b"ftypmsf1"])], Mime::HeifSequence),
    hm(
        &[oh(
            &[4],
            &[
                b"ftypmp4",
                b"ftypisom",
                b"ftypM4V",
                b"ftypMSNV",
                b"ftypavc1",
                b"ftypFACE",
                b"ftypdash",
            ],
        )],
        Mime::UndeterminedMp4,
    ),
    hm(&[oh(&[4], &[b"ftypqt"])], Mime::Mov),
    hm(&[oh(&[0], &[b"fLaC"])], Mime::Flac),
    hm(&[oh(&[0], &[b"RIFF"]), oh(&[8], &[b"WAVE"])], Mime::Wave),
    hm(&[oh(&[0], &[b"wvpk"])], Mime::WavPack),
    hm(&[oh(&[8], &[b"AVI "])], Mime::Avi),
    hm(
        &[oh(
            &[0],
            &[b"\x30\x26\xb2\x75\x8e\x66\xcf\x11\xa6\xd9\x00\xaa\x00\x62\xce\x6c"],
        )],
        Mime::UndeterminedWm,
    ),
    hm(&[oh(&[0], &[b"\x4d\x5a\x90\x00\x03"])], Mime::WindowsExe),
    hm(
        &[oh(
            &[0],
            &[
                b"\x31\xbe\x00\x00",
                b"PO^Q",
                b"\xfe\x37\x00\x23",
                b"\xdb\xa5\x2d\x00\x00\x00",
                b"\xdb\xa5\x2d\x00",
            ],
        )],
        Mime::Doc,
    ),
    hm(
        &[oh(&[0], &[b"\xed\xde\xad\x0b", b"\x0b\xad\xde\xad"])],
        Mime::Ppt,
    ),
    hm(
        &[oh(&[0], &[b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"])],
        Mime::UndeterminedOle,
    ),
];

fn default_thumbnail_map() -> &'static Mutex<HashMap<Mime, PathBuf>> {
    static MAP: OnceLock<Mutex<HashMap<Mime, PathBuf>>> = OnceLock::new();

    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_default_thumbnail_map() -> MutexGuard<'static, HashMap<Mime, PathBuf>> {
    default_thumbnail_map()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `initialise_mimes_to_default_thumbnail_paths()`
pub fn initialise_mimes_to_default_thumbnail_paths() {
    let hydro = hydrus_static_dir::get_static_path(DEFAULT_THUMBNAIL_FILENAME, false);
    let audio = hydrus_static_dir::get_static_path("audio.png", false);
    let video = hydrus_static_dir::get_static_path("video.png", false);
    let zip = hydrus_static_dir::get_static_path("zip.png", false);
    let image = hydrus_static_dir::get_static_path("image.png", false);

    let mut map = lock_default_thumbnail_map();

    map.clear();

    for mime in AUDIO {
        map.insert(*mime, audio.clone());
    }

    for mime in VIDEO {
        map.insert(*mime, video.clone());
    }

    for mime in ANIMATIONS {
        map.insert(*mime, video.clone());
    }

    for mime in ARCHIVES {
        map.insert(*mime, zip.clone());
    }

    for mime in IMAGES {
        map.insert(*mime, image.clone());
    }

    map.insert(Mime::Unknown, hydro.clone());
    map.insert(Mime::OctetStream, hydro);
}

/// `mimes_to_default_thumbnail_paths[ mime ]`
pub fn default_thumbnail_path(mime: Mime) -> PathBuf {
    {
        let map = lock_default_thumbnail_map();

        if !map.is_empty() {
            if let Some(path) = map.get(&mime) {
                return path.clone();
            }

            return map.get(&Mime::Unknown).cloned().unwrap_or_else(|| {
                hydrus_static_dir::get_static_path(DEFAULT_THUMBNAIL_FILENAME, false)
            });
        }
    }

    initialise_mimes_to_default_thumbnail_paths();

    default_thumbnail_path(mime)
}

fn default_thumbnail_numpy(
    mime: Mime,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    let path = default_thumbnail_path(mime);

    hydrus_image_handling::generate_default_thumbnail_numpy_from_path(path, target_resolution)
}

/// `generate_default_thumbnail( mime, target_resolution )`
pub fn generate_default_thumbnail(
    mime: Mime,
    target_resolution: (u32, u32),
) -> HydrusResult<DynamicImage> {
    default_thumbnail_numpy(mime, target_resolution)
}

/// `generate_thumbnail_bytes( path, target_resolution, mime, duration_ms, num_frames, percentage_in = 35 )`
pub fn generate_thumbnail_bytes(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
    duration_ms: Option<i64>,
    num_frames: Option<u32>,
    percentage_in: f64,
) -> HydrusResult<Vec<u8>> {
    let image = generate_thumbnail_numpy(
        path,
        target_resolution,
        mime,
        duration_ms,
        num_frames,
        percentage_in,
    )?;

    hydrus_image_handling::generate_thumbnail_bytes_from_numpy(&image)
}

/// `print_more_thumb_error_info( e, message, extra_description = None )`
pub fn print_more_thumb_error_info(
    error: &HydrusException,
    message: &str,
    extra_description: Option<&str>,
) {
    if error.is_a(ErrorKind::NoThumbnailFile) {
        return;
    }

    hydrus_data::safe_print(message);

    if let Some(extra) = extra_description {
        hydrus_data::safe_print(extra);
    }

    hydrus_data::safe_print(hydrus_data::print_exception(error, false));
}

/// `generate_thumbnail_numpy( path, target_resolution, mime, duration_ms, num_frames, percentage_in = 35, extra_description = None )`
pub fn generate_thumbnail_numpy(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
    duration_ms: Option<i64>,
    num_frames: Option<u32>,
    percentage_in: f64,
) -> HydrusResult<DynamicImage> {
    generate_thumbnail_numpy_with_description(
        path.as_ref(),
        target_resolution,
        mime,
        duration_ms,
        num_frames,
        percentage_in,
        None,
    )
}

fn generate_thumbnail_numpy_with_description(
    path: &Path,
    target_resolution: (u32, u32),
    mime: Mime,
    duration_ms: Option<i64>,
    num_frames: Option<u32>,
    percentage_in: f64,
    extra_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let throttled_error = |error: HydrusException| {
        print_more_thumb_error_info(
            &error,
            &format!("Problem generating thumbnail for \"{}\":", path.display()),
            extra_description,
        );

        default_thumbnail_numpy(mime, target_resolution)
    };

    if AUDIO.contains(&mime) {
        return match generate_thumbnail_numpy_audio(
            path,
            target_resolution,
            mime,
            extra_description,
        ) {
            Ok(image) => Ok(image),
            Err(error) => throttled_error(error),
        };
    }

    if mime == Mime::Ugoira {
        return match (|| {
            let frame_count = num_frames.unwrap_or(1).saturating_sub(1);

            let desired_thumb_frame_index =
                ((percentage_in / 100.0) * f64::from(frame_count)) as usize;

            hydrus_ugoira_handling::generate_thumbnail_numpy_from_ugoira_path(
                path,
                target_resolution,
                desired_thumb_frame_index,
            )
        })() {
            Ok(image) => Ok(image),
            Err(error) => throttled_error(error),
        };
    }

    if VIDEO.contains(&mime) || ANIMATIONS.contains(&mime) {
        if let Ok(Some(image)) = generate_thumbnail_numpy_using_ffmpeg_video_renderer(
            path,
            target_resolution,
            mime,
            duration_ms,
            num_frames,
            percentage_in,
            extra_description,
        ) {
            return Ok(image);
        }

        return default_thumbnail_numpy(mime, target_resolution);
    }

    if mime == Mime::Cbz || mime == Mime::Epub {
        return match generate_thumbnail_numpy_weird_application_gubbins(
            path,
            target_resolution,
            mime,
            extra_description,
        ) {
            Ok(image) => Ok(image),
            Err(error) => throttled_error(error),
        };
    }

    if mime == Mime::Clip
        || mime == Mime::Procreate
        || mime == Mime::Docx
        || mime == Mime::Xlsx
        || mime == Mime::Pptx
        || mime == Mime::Krita
        || mime == Mime::OpenRaster
        || mime == Mime::Psd
        || mime == Mime::PaintDotNet
        || mime == Mime::Sai2
        || mime == Mime::Xcf
        || mime == Mime::Svg
        || mime == Mime::Pdf
    {
        return match generate_thumbnail_numpy_weird_application_gubbins(
            path,
            target_resolution,
            mime,
            extra_description,
        ) {
            Ok(image) => Ok(image),
            Err(error) => throttled_error(error),
        };
    }

    if IMAGES.contains(&mime) {
        return match hydrus_image_handling::generate_thumbnail_numpy_from_static_image_path(
            path,
            target_resolution,
            mime,
        ) {
            Ok(image) => Ok(image),
            Err(error) => throttled_error(error),
        };
    }

    default_thumbnail_numpy(mime, target_resolution)
}

/// `generate_thumbnail_numpy_audio( path, target_resolution, mime, extra_description = None )`
pub fn generate_thumbnail_numpy_audio(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
    _extra_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let (file_handle, temp_path) = hydrus_temp::get_temp_path("audio_image", ".png", None)?;

    let result = (|| -> HydrusResult<DynamicImage> {
        let rendered =
            hydrus_ffmpeg_rendering::render_any_attached_still_image_to_path(path, &temp_path)?;

        if !rendered {
            return Err(HydrusException::new(
                ErrorKind::NoThumbnailFile,
                "No attached still image.",
            ));
        }

        let cover_mime = get_mime(&temp_path, false)?;

        hydrus_image_handling::generate_thumbnail_numpy_from_static_image_path(
            &temp_path,
            target_resolution,
            cover_mime,
        )
    })();

    hydrus_temp::clean_up_temp_path(file_handle, &temp_path);

    result.or_else(|_| default_thumbnail_numpy(mime, target_resolution))
}

/// `generate_thumbnail_numpy_using_ffmpeg_video_renderer( path, target_resolution, mime, duration_ms, num_frames, percentage_in = 35, extra_description = None )`
pub fn generate_thumbnail_numpy_using_ffmpeg_video_renderer(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
    _duration_ms: Option<i64>,
    _num_frames: Option<u32>,
    _percentage_in: f64,
    _extra_description: Option<&str>,
) -> HydrusResult<Option<DynamicImage>> {
    let path = path.as_ref();

    let png_bytes = match hydrus_ffmpeg_rendering::render_image_to_png_bytes(path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };

    let image = match hydrus_image_handling::decode_image_bytes(&png_bytes) {
        Ok(image) => image,
        Err(_) => return Ok(None),
    };

    let thumbnail = hydrus_image_handling::resize_image(&image, target_resolution, None);

    let _ = mime;

    Ok(Some(thumbnail))
}

/// `generate_thumbnail_numpy_weird_application_gubbins( path, target_resolution, mime, extra_description = None )`
pub fn generate_thumbnail_numpy_weird_application_gubbins(
    path: impl AsRef<Path>,
    target_resolution: (u32, u32),
    mime: Mime,
    _extra_description: Option<&str>,
) -> HydrusResult<DynamicImage> {
    let path = path.as_ref();

    if mime == Mime::Cbz || mime == Mime::Epub {
        let (file_handle, temp_path) = hydrus_temp::get_temp_path("cover_page_thumb", "", None)?;

        let result = (|| -> HydrusResult<DynamicImage> {
            hydrus_archive_handling::extract_cover_page(path, &temp_path, mime)?;

            let cover_mime = get_mime(&temp_path, false)?;

            if cover_mime == Mime::Html {
                return Err(HydrusException::new(
                    ErrorKind::NoThumbnailFile,
                    "Do not support an HTML thumbnail for epubs yet!",
                ));
            }

            if cover_mime == Mime::Svg {
                let thumbnail = hydrus_svg_handling::generate_thumbnail_numpy_from_svg_path(
                    &temp_path,
                    target_resolution,
                )?;

                return hydrus_image_handling::decode_image_bytes(&thumbnail).map(|image| {
                    hydrus_image_handling::resize_image(&image, target_resolution, None)
                });
            }

            hydrus_image_handling::generate_thumbnail_numpy_from_static_image_path(
                &temp_path,
                target_resolution,
                cover_mime,
            )
        })();

        hydrus_temp::clean_up_temp_path(file_handle, &temp_path);

        return result;
    }

    if mime == Mime::Clip {
        let (file_handle, temp_path) = hydrus_temp::get_temp_path("clip_thumb", "", None)?;

        let result = (|| -> HydrusResult<DynamicImage> {
            hydrus_clip_handling::extract_db_png_to_path(path, &temp_path)?;

            hydrus_image_handling::generate_thumbnail_numpy_from_static_image_path(
                &temp_path,
                target_resolution,
                Mime::Png,
            )
        })();

        hydrus_temp::clean_up_temp_path(file_handle, &temp_path);

        return result;
    }

    if mime == Mime::Procreate {
        let (file_handle, temp_path) = hydrus_temp::get_temp_path("procreate_thumb", ".png", None)?;

        let result = (|| -> HydrusResult<DynamicImage> {
            hydrus_procreate_handling::extract_zipped_thumbnail_to_path(path, &temp_path)?;

            hydrus_image_handling::generate_thumbnail_numpy_from_static_image_path(
                &temp_path,
                target_resolution,
                Mime::Png,
            )
        })();

        hydrus_temp::clean_up_temp_path(file_handle, &temp_path);

        return result;
    }

    if mime == Mime::Docx || mime == Mime::Xlsx || mime == Mime::Pptx {
        return hydrus_office_open_xml_handling::generate_thumbnail_from_office_path(
            path,
            target_resolution,
        );
    }

    if mime == Mime::Svg {
        let thumbnail =
            hydrus_svg_handling::generate_thumbnail_numpy_from_svg_path(path, target_resolution)?;

        let image = hydrus_image_handling::decode_image_bytes(&thumbnail)?;

        return Ok(hydrus_image_handling::resize_image(
            &image,
            target_resolution,
            None,
        ));
    }

    if mime == Mime::Pdf {
        let thumbnail =
            crate::files::hydrus_pdf_handling::base_generate_thumbnail_numpy_from_pdf_path(
                path,
                target_resolution,
            )?;

        let image = hydrus_image_handling::decode_image_bytes(&thumbnail)?;

        return Ok(hydrus_image_handling::resize_image(
            &image,
            target_resolution,
            None,
        ));
    }

    if mime == Mime::Krita {
        return hydrus_krita_handling::generate_thumbnail_numpy_from_kra_path(
            path,
            target_resolution,
        );
    }

    if mime == Mime::OpenRaster {
        return hydrus_ora_handling::generate_thumbnail_numpy_from_ora_path(
            path,
            target_resolution,
        );
    }

    if mime == Mime::Psd {
        return hydrus_psd_handling::generate_thumbnail_numpy_from_psd_path(
            path,
            target_resolution,
        );
    }

    if mime == Mime::PaintDotNet {
        return hydrus_paint_net_handling::generate_thumbnail_numpy_from_paint_net(
            path,
            target_resolution,
        );
    }

    // SAI2 and XCF need their own specialised renderers. The modules that can
    // read their metadata are ported separately; until then they deliberately
    // use the generic image-project thumbnail.
    Err(HydrusException::new(
        ErrorKind::NoThumbnailFile,
        "No specialised renderer for this image project file yet.",
    ))
}

fn io_error(path: &Path, error: std::io::Error) -> HydrusException {
    HydrusException::new(
        ErrorKind::FileMissing,
        format!("Could not access \"{}\": {error}", path.display()),
    )
}

fn zero_size_error() -> HydrusException {
    HydrusException::new(ErrorKind::ZeroSizeFile, "File is of zero length!")
}

fn read_first_bytes(path: &Path, count: usize) -> HydrusResult<Vec<u8>> {
    let mut file = File::open(path).map_err(|error| io_error(path, error))?;

    let mut bytes = vec![0_u8; count];

    let mut read_total = 0;

    while read_total < count {
        match file.read(&mut bytes[read_total..]) {
            Ok(0) => break,
            Ok(read) => read_total += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(path, error)),
        }
    }

    bytes.truncate(read_total);

    Ok(bytes)
}

/// `passes_offsets_and_headers_pair( offsets, headers, first_bytes_of_file )`
pub fn passes_offsets_and_headers_pair(
    offsets: &[usize],
    headers: &[&[u8]],
    first_bytes_of_file: &[u8],
) -> bool {
    for offset in offsets {
        for header in headers {
            let Some(end) = offset.checked_add(header.len()) else {
                continue;
            };

            if first_bytes_of_file.get(*offset..end) == Some(*header) {
                return true;
            }
        }
    }

    false
}

/// `passes_offsets_and_headers( offsets_and_headers, first_bytes_of_file )`
pub fn passes_offsets_and_headers(
    offsets_and_headers: &[OffsetAndHeaders],
    first_bytes_of_file: &[u8],
) -> bool {
    offsets_and_headers.iter().all(|pair| {
        passes_offsets_and_headers_pair(pair.offsets, pair.headers, first_bytes_of_file)
    })
}

fn find_header_mime(first_bytes_of_file: &[u8]) -> Option<Mime> {
    HEADERS_AND_MIME
        .iter()
        .find(|entry| passes_offsets_and_headers(entry.offsets_and_headers, first_bytes_of_file))
        .map(|entry| entry.mime)
}

fn update_mime_from_bytes(bytes: &[u8]) -> Option<Mime> {
    hydrus_serialisable::initialise_serialisable_registry();
    crate::networking::hydrus_network::register_serialisable_types();

    let update = hydrus_serialisable::create_from_network_bytes(bytes, false).ok()?;

    match update.serialisable_type() {
        crate::hydrus_serialisable::SERIALISABLE_TYPE_CONTENT_UPDATE => Some(Mime::UpdateContent),
        crate::hydrus_serialisable::SERIALISABLE_TYPE_DEFINITIONS_UPDATE => {
            Some(Mime::UpdateDefinitions)
        }
        _ => None,
    }
}

/// `get_mime( path, ok_to_look_for_hydrus_updates = False )`
pub fn get_mime(path: impl AsRef<Path>, ok_to_look_for_hydrus_updates: bool) -> HydrusResult<Mime> {
    let path = path.as_ref();

    let size = std::fs::metadata(path)
        .map_err(|error| io_error(path, error))?
        .len();

    if size == 0 {
        return Err(zero_size_error());
    }

    if ok_to_look_for_hydrus_updates && size < 64 * 1024 * 1024 {
        let bytes = std::fs::read(path).map_err(|error| io_error(path, error))?;

        if let Some(mime) = update_mime_from_bytes(&bytes) {
            return Ok(mime);
        }
    }

    let first_bytes_of_file = read_first_bytes(path, 256)?;

    let Some(header_mime) = find_header_mime(&first_bytes_of_file) else {
        return get_mime_from_non_header_data(path, &first_bytes_of_file);
    };

    if header_mime == Mime::Zip {
        match hydrus_archive_handling::is_encrypted_zip(path) {
            Ok(true) => return Ok(Mime::Zip),
            Err(error) if error.is_a(ErrorKind::DamagedOrUnusualFile) => return Ok(Mime::Zip),
            _ => {}
        }

        if let Some(mime) = hydrus_archive_handling::mime_from_open_document(path) {
            return Ok(mime);
        }

        if let Some(mime) =
            hydrus_office_open_xml_handling::mime_from_microsoft_open_xml_document(path)
        {
            return Ok(mime);
        }

        if hydrus_procreate_handling::zip_looks_like_procreate(path) {
            return Ok(Mime::Procreate);
        }

        // Ugoira detection is deliberately after open documents and Office,
        // matching Python.
        if hydrus_ugoira_handling::zip_looks_like_ugoira(path) {
            return Ok(Mime::Ugoira);
        }

        if hydrus_archive_handling::zip_looks_like_cbz(path) {
            return Ok(Mime::Cbz);
        }

        return Ok(Mime::Zip);
    }

    if header_mime == Mime::UndeterminedWm || header_mime == Mime::UndeterminedMp4 {
        return get_mime_from_ffmpeg(path).or(Ok(Mime::Unknown));
    }

    if header_mime == Mime::UndeterminedPng {
        return Ok(
            if hydrus_animation_handling::is_png_animated(&first_bytes_of_file) {
                Mime::Apng
            } else {
                Mime::Png
            },
        );
    }

    if header_mime == Mime::UndeterminedGif {
        return Ok(
            if hydrus_animation_handling::pil_animation_has_duration(path) {
                Mime::AnimationGif
            } else {
                Mime::StaticGif
            },
        );
    }

    if header_mime == Mime::UndeterminedJxl {
        return Ok(if hydrus_video_handling::file_is_animated(path) {
            Mime::AnimationJxl
        } else {
            Mime::Jxl
        });
    }

    if header_mime == Mime::UndeterminedWebp {
        return Ok(
            if hydrus_animation_handling::pil_animation_has_duration(path) {
                Mime::AnimationWebp
            } else {
                Mime::Webp
            },
        );
    }

    if header_mime == Mime::UndeterminedOle {
        return Ok(hydrus_ole_handling::mime_from_ole_file(path));
    }

    Ok(header_mime)
}

fn get_mime_from_non_header_data(path: &Path, first_bytes_of_file: &[u8]) -> HydrusResult<Mime> {
    if first_bytes_of_file.starts_with(b"{") || first_bytes_of_file.starts_with(b"[") {
        let potential_json_document_bytes =
            std::fs::read(path).map_err(|error| io_error(path, error))?;

        if hydrus_text::looks_like_json(&potential_json_document_bytes) {
            return Ok(Mime::Json);
        }
    }

    if hydrus_text::looks_like_html(first_bytes_of_file) {
        return Ok(Mime::Html);
    }

    if hydrus_text::looks_like_svg(first_bytes_of_file) {
        return Ok(Mime::Svg);
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    let likely_to_false_positive =
        file_name.ends_with(".txt") || file_name.ends_with(".log") || file_name.ends_with(".json");

    if !likely_to_false_positive {
        match get_mime_from_ffmpeg(path) {
            Ok(mime) if mime != Mime::Unknown => return Ok(mime),
            Ok(_) => {}
            Err(error) if error.is_a(ErrorKind::UnsupportedFile) => {}
            Err(error) if error.is_a(ErrorKind::Shutdown) => return Err(error),
            Err(_) => {
                hydrus_data::safe_print(format!("FFMPEG had trouble with: {}", path.display()));
            }
        }
    }

    Ok(Mime::Unknown)
}

/// `get_mime_from_ffmpeg( path )`
pub fn get_mime_from_ffmpeg(path: impl AsRef<Path>) -> HydrusResult<Mime> {
    let path = path.as_ref();

    let lines = hydrus_ffmpeg::get_ffmpeg_info_lines(path, None)?;

    let (has_video, video_format, _) = hydrus_video_handling::parse_ffmpeg_video_format(&lines);
    let (has_audio, audio_format, _, _) =
        crate::files::hydrus_audio_handling::parse_ffmpeg_audio(&lines);

    if !has_video && !has_audio {
        return Ok(Mime::Unknown);
    }

    let mime_text = match hydrus_video_handling::parse_ffmpeg_mime_text(&lines) {
        Ok(text) => text,
        Err(_) => return Ok(Mime::Unknown),
    };

    if mime_text.contains("matroska") || mime_text.contains("webm") {
        let has_webm_video = if has_video {
            ["vp8", "vp9", "av1"]
                .iter()
                .any(|format| video_format.contains(format))
        } else {
            false
        };

        let has_webm_audio = if has_audio {
            ["vorbis", "opus"]
                .iter()
                .any(|format| audio_format.contains(format))
        } else {
            true
        };

        if has_webm_video && has_webm_audio {
            return Ok(Mime::Webm);
        }

        if has_video {
            return Ok(Mime::Mkv);
        }

        if has_audio {
            return Ok(Mime::AudioMkv);
        }
    } else if mime_text == "mpeg" || mime_text == "mpegvideo" || mime_text == "mpegts" {
        return Ok(Mime::Mpeg);
    } else if mime_text == "flac" {
        return Ok(Mime::Flac);
    } else if mime_text == "wav" {
        return Ok(Mime::Wave);
    } else if mime_text == "mp3" {
        return Ok(Mime::Mp3);
    } else if mime_text == "tta" {
        return Ok(Mime::TrueAudio);
    } else if mime_text.contains("mp4") {
        let container = hydrus_video_handling::parse_ffmpeg_metadata_container(&lines);

        if container == "M4A" {
            return Ok(Mime::M4a);
        }

        if container == "qt" {
            return Ok(Mime::Mov);
        }

        if container == "isom" || container == "mp42" {
            if has_video {
                return Ok(Mime::Mp4Video);
            }

            if has_audio {
                return Ok(Mime::AudioMp4);
            }
        }

        if has_audio && video_format.contains("mjpeg") {
            return Ok(Mime::M4a);
        }

        if has_video {
            return Ok(Mime::Mp4Video);
        }

        if has_audio {
            return Ok(Mime::AudioMp4);
        }
    } else if mime_text == "ogg" {
        return Ok(if has_video { Mime::Ogv } else { Mime::Ogg });
    } else if mime_text.contains("rm") {
        return Ok(if has_video {
            Mime::RealVideo
        } else {
            Mime::RealAudio
        });
    } else if mime_text == "asf" {
        return Ok(if has_video { Mime::Wmv } else { Mime::Wma });
    } else if mime_text == "wv" {
        return Ok(Mime::WavPack);
    }

    Ok(Mime::Unknown)
}

/// `get_is_png_real_quick( path )`
pub fn get_is_png_real_quick(path: impl AsRef<Path>) -> HydrusResult<bool> {
    let first_bytes = read_first_bytes(path.as_ref(), 256)?;

    Ok(HEADERS_AND_MIME.iter().any(|entry| {
        entry.mime == Mime::UndeterminedPng
            && passes_offsets_and_headers(entry.offsets_and_headers, &first_bytes)
    }))
}

/// `get_thumbnail_mime( path )`
pub fn get_thumbnail_mime(path: impl AsRef<Path>) -> HydrusResult<Mime> {
    let path = path.as_ref();

    let first_bytes = read_first_bytes(path, 256)?;

    for entry in HEADERS_AND_MIME {
        if entry.mime != Mime::Jpeg && entry.mime != Mime::UndeterminedPng {
            continue;
        }

        if passes_offsets_and_headers(entry.offsets_and_headers, &first_bytes) {
            return Ok(if entry.mime == Mime::UndeterminedPng {
                Mime::Png
            } else {
                entry.mime
            });
        }
    }

    get_mime(path, false)
}

/// `get_file_info( path, mime = None, ok_to_look_for_hydrus_updates = False )`
pub fn get_file_info(
    path: impl AsRef<Path>,
    mime: Option<Mime>,
    ok_to_look_for_hydrus_updates: bool,
) -> HydrusResult<FileInfo> {
    let path = path.as_ref();

    let size = std::fs::metadata(path)
        .map_err(|error| io_error(path, error))?
        .len();

    if size == 0 {
        return Err(zero_size_error());
    }

    let mime = match mime {
        Some(mime) => mime,
        None => get_mime(path, ok_to_look_for_hydrus_updates)?,
    };

    if !allowed_mimes().contains(&mime) {
        let message = if mime == Mime::Html {
            "Looks like HTML -- maybe the client needs to be taught how to recognise this URL and then parse it?"
        } else if mime == Mime::Json {
            "Looks like JSON -- maybe the client needs to be taught how to recognise this URL and then parse it?"
        } else if mime == Mime::Unknown {
            "Unknown filetype!"
        } else {
            "Filetype is not permitted!"
        };

        return Err(HydrusException::new(ErrorKind::UnsupportedFile, message));
    }

    if PIL_HEIF_MIMES.contains(&mime) && !hydrus_image_handling::HEIF_OK {
        return Err(HydrusException::new(
            ErrorKind::UnsupportedFile,
            format!(
                "Sorry, you do not have HEIF support, which is needed to support this filetype ({})! Please rebuild your venv.",
                mime.mimetype_str()
            ),
        ));
    }

    if PIL_AVIF_MIMES.contains(&mime) && !hydrus_image_handling::AVIF_OK {
        return Err(HydrusException::new(
            ErrorKind::UnsupportedFile,
            format!(
                "Sorry, you do not have AVIF support! Please rebuild your venv. ({})",
                mime.mimetype_str()
            ),
        ));
    }

    if mime == Mime::Jxl && !hydrus_image_handling::JXL_OK {
        return Err(HydrusException::new(
            ErrorKind::UnsupportedFile,
            "Sorry, you do not have JpegXL support! Please rebuild your venv. (jxl)",
        ));
    }

    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut duration_ms: Option<i64> = None;
    let mut num_frames: Option<u32> = None;
    let mut num_words: Option<i64> = None;

    let mut has_audio = mimes_that_definitely_have_audio().contains(&mime);

    if mime == Mime::Cbz || mime == Mime::Epub {
        let (file_handle, temp_path) = hydrus_temp::get_temp_path("coverpage_file", "", None)?;

        let cover_result = (|| -> HydrusResult<(u32, u32)> {
            hydrus_archive_handling::extract_cover_page(path, &temp_path, mime)?;

            let cover_mime = get_mime(&temp_path, false)?;

            hydrus_image_handling::get_image_resolution(&temp_path, cover_mime)
        })();

        hydrus_temp::clean_up_temp_path(file_handle, &temp_path);

        if let Ok((cover_width, cover_height)) = cover_result {
            width = Some(cover_width);
            height = Some(cover_height);
        }
    } else if mime == Mime::Clip {
        if let Ok(((clip_width, clip_height), clip_duration, clip_frames)) =
            hydrus_clip_handling::get_clip_properties(path)
        {
            width = Some(clip_width.unsigned_abs() as u32);
            height = Some(clip_height.unsigned_abs() as u32);
            duration_ms = clip_duration.map(|duration| duration.round() as i64);
            num_frames = clip_frames.map(|frames| frames.unsigned_abs() as u32);
        }
    } else if mime == Mime::Krita {
        if let Ok((krita_width, krita_height)) = hydrus_krita_handling::get_kra_properties(path) {
            width = Some(krita_width);
            height = Some(krita_height);
        }
    } else if mime == Mime::OpenRaster {
        if let Ok((ora_width, ora_height)) = hydrus_ora_handling::get_ora_properties(path) {
            width = Some(ora_width);
            height = Some(ora_height);
        }
    } else if mime == Mime::PaintDotNet {
        if let Ok((paint_net_width, paint_net_height)) =
            hydrus_paint_net_handling::get_paint_net_resolution(path)
        {
            width = Some(paint_net_width);
            height = Some(paint_net_height);
        }
    } else if mime == Mime::Procreate {
        if let Ok((procreate_width, procreate_height)) =
            hydrus_procreate_handling::get_procreate_resolution(path)
        {
            width = Some(procreate_width);
            height = Some(procreate_height);
        }
    } else if mime == Mime::Svg {
        if let Ok((svg_width, svg_height)) = hydrus_svg_handling::get_svg_resolution(path) {
            width = Some(svg_width);
            height = Some(svg_height);
        }
    } else if mime == Mime::Pptx {
        let (pptx_num_words, (pptx_width, pptx_height)) =
            hydrus_office_open_xml_handling::get_pptx_info(path);

        num_words = pptx_num_words;
        width = pptx_width;
        height = pptx_height;
    } else if mime == Mime::Docx {
        num_words = hydrus_office_open_xml_handling::get_docx_info(path);
    } else if POTENTIAL_OFFICE_OLE_MIMES.contains(&mime) {
        num_words = hydrus_ole_handling::office_ole_document_word_count(path).unwrap_or(None);
    } else if mime == Mime::Flash {
        if let Ok(((flash_width, flash_height), flash_duration, flash_frames)) =
            crate::files::hydrus_flash_handling::get_flash_properties(path)
        {
            width = Some(flash_width as u32);
            height = Some(flash_height as u32);
            duration_ms = Some(flash_duration as i64);
            num_frames = Some(flash_frames);
        }
    } else if mime == Mime::Psd {
        match hydrus_psd_handling::get_psd_resolution(path) {
            Ok((psd_width, psd_height)) => {
                width = Some(psd_width);
                height = Some(psd_height);
            }
            Err(error) => {
                hydrus_data::safe_print(format!(
                    "Problem calculating resolution for \"{}\":",
                    path.display()
                ));
                hydrus_data::safe_print(hydrus_data::print_exception(&error, false));
            }
        }
    } else if mime == Mime::Ugoira {
        if let Ok(((ugoira_width, ugoira_height), ugoira_duration, ugoira_frames)) =
            hydrus_ugoira_handling::get_ugoira_properties(path)
        {
            width = Some(ugoira_width);
            height = Some(ugoira_height);
            duration_ms = ugoira_duration;
            num_frames = ugoira_frames;
        }
    } else if VIDEO.contains(&mime)
        || HEIF_TYPE_SEQUENCES.contains(&mime)
        || mime == Mime::AvifSequence
        || mime == Mime::AnimationJxl
    {
        if let Ok(((video_width, video_height), video_duration, video_frames, video_audio)) =
            hydrus_video_handling::get_ffmpeg_video_properties(path, false)
        {
            width = Some(video_width);
            height = Some(video_height);
            duration_ms = Some(video_duration);
            num_frames = Some(video_frames);
            has_audio = video_audio;
        }
    } else if ANIMATIONS.contains(&mime) {
        if let Ok(((animation_width, animation_height), animation_duration, animation_frames)) =
            hydrus_animation_handling::get_animation_properties(path, mime)
        {
            width = Some(animation_width);
            height = Some(animation_height);
            duration_ms = Some(animation_duration);
            num_frames = Some(animation_frames);
        }
    } else if IMAGES.contains(&mime) {
        if let Ok((image_width, image_height)) =
            hydrus_image_handling::get_image_resolution(path, mime)
        {
            width = Some(image_width);
            height = Some(image_height);
        }
    } else if AUDIO.contains(&mime) {
        let ffmpeg_lines = hydrus_ffmpeg::get_ffmpeg_info_lines(path, None)?;

        let (file_duration_in_s, _) = hydrus_video_handling::parse_ffmpeg_duration(&ffmpeg_lines)?;

        let Some(file_duration_in_s) = file_duration_in_s else {
            return Err(HydrusException::new(
                ErrorKind::DamagedOrUnusualFile,
                "Could not determine the duration of this file!",
            ));
        };

        duration_ms = Some((file_duration_in_s * 1000.0) as i64);
    }

    Ok(FileInfo {
        size,
        mime,
        width,
        height,
        duration_ms,
        num_frames,
        has_audio,
        num_words,
    })
}

/// `get_extra_hashes_from_path( path )`
pub fn get_extra_hashes_from_path(path: impl AsRef<Path>) -> HydrusResult<ExtraHashes> {
    let path = path.as_ref();

    let file = File::open(path).map_err(|error| io_error(path, error))?;

    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();

    for block in hydrus_paths::read_file_like_as_blocks(file) {
        let block = block.map_err(|error| io_error(path, error))?;

        md5.update(&block);
        sha1.update(&block);
        sha512.update(&block);
    }

    Ok(ExtraHashes {
        md5: md5.finalize().to_vec(),
        sha1: sha1.finalize().to_vec(),
        sha512: sha512.finalize().to_vec(),
    })
}

/// `get_hash_from_path( path )`
pub fn get_hash_from_path(path: impl AsRef<Path>) -> HydrusResult<Vec<u8>> {
    let path = path.as_ref();

    let file = File::open(path).map_err(|error| io_error(path, error))?;

    let mut sha256 = Sha256::new();

    for block in hydrus_paths::read_file_like_as_blocks(file) {
        let block = block.map_err(|error| io_error(path, error))?;

        sha256.update(&block);
    }

    Ok(sha256.finalize().to_vec())
}

/// `get_file_modified_timestamp_ms( path )`
pub fn get_file_modified_timestamp_ms(path: impl AsRef<Path>) -> HydrusResult<i64> {
    let path = path.as_ref();

    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| io_error(path, error))?;

    let duration = modified
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            HydrusException::new(
                ErrorKind::DataMissing,
                "File modification time was before the Unix epoch!",
            )
        })?;

    Ok(duration.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use super::*;
    use image::ImageFormat;

    fn unique_path(label: &str, extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_file_handling_{}_{}_{}.{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            extension
        ))
    }

    fn write_bytes(label: &str, extension: &str, bytes: &[u8]) -> PathBuf {
        let path = unique_path(label, extension);

        let mut file = File::create(&path).unwrap();
        file.write_all(bytes).unwrap();

        path
    }

    fn png_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(name);
        chunk.extend_from_slice(data);
        chunk.extend_from_slice(&[0; 4]);

        chunk
    }

    #[test]
    fn header_matching_uses_offsets_and_all_pairs() {
        let bytes = b"RIFFxxxxWAVEmore";

        let pair = oh(&[0], &[b"RIFF"]);
        let wave = oh(&[8], &[b"WAVE"]);

        assert!(passes_offsets_and_headers_pair(
            pair.offsets,
            pair.headers,
            bytes
        ));
        assert!(passes_offsets_and_headers(&[pair, wave], bytes));
        assert!(!passes_offsets_and_headers(
            &[pair, oh(&[8], &[b"AVI "])],
            bytes
        ));
    }

    #[test]
    fn jpeg_and_png_are_detected_from_headers() {
        let jpeg = write_bytes("jpeg", "jpg", b"\xff\xd8\xff\xe0not a real image");
        let png = write_bytes("png", "png", b"\x89PNG\r\n\x1a\nrest");

        assert_eq!(get_mime(&jpeg, false).unwrap(), Mime::Jpeg);
        assert_eq!(get_mime(&png, false).unwrap(), Mime::Png);

        std::fs::remove_file(jpeg).ok();
        std::fs::remove_file(png).ok();
    }

    #[test]
    fn apng_header_becomes_apng() {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut actl = Vec::new();
        actl.extend_from_slice(&2_u32.to_be_bytes());
        actl.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&png_chunk(b"acTL", &actl));

        let path = write_bytes("apng", "png", &bytes);

        assert_eq!(get_mime(&path, false).unwrap(), Mime::Apng);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn static_and_animated_gifs_are_distinguished() {
        let static_gif = write_bytes(
            "static_gif",
            "gif",
            b"GIF89a\x01\x00\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
        );

        assert_eq!(get_mime(&static_gif, false).unwrap(), Mime::StaticGif);

        let animated_gif = write_bytes(
            "animated_gif",
            "gif",
            b"GIF89a\x01\x00\x01\x00\x00\x00\x00\x21\xf9\x04\x00\x0a\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00\x21\xf9\x04\x00\x0a\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
        );

        assert_eq!(get_mime(&animated_gif, false).unwrap(), Mime::AnimationGif);

        std::fs::remove_file(static_gif).ok();
        std::fs::remove_file(animated_gif).ok();
    }

    #[test]
    fn json_and_html_fallbacks_are_detected() {
        let json = write_bytes("json", "json", br#"{"hello": "world"}"#);

        assert_eq!(get_mime(&json, false).unwrap(), Mime::Json);

        let html = write_bytes(
            "html",
            "html",
            b"<!DOCTYPE html><html><body>hi</body></html>",
        );

        assert_eq!(get_mime(&html, false).unwrap(), Mime::Html);

        std::fs::remove_file(json).ok();
        std::fs::remove_file(html).ok();
    }

    #[test]
    fn zero_size_files_are_rejected() {
        let path = write_bytes("empty", "bin", b"");

        let error = get_mime(&path, false).unwrap_err();

        assert!(error.is_a(ErrorKind::ZeroSizeFile));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn extra_hashes_match_known_vectors() {
        let path = write_bytes("hashes", "txt", b"abc");

        let hashes = get_extra_hashes_from_path(&path).unwrap();

        assert_eq!(
            crate::hydrus_data::hex_encode(&hashes.md5),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            crate::hydrus_data::hex_encode(&hashes.sha1),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            crate::hydrus_data::hex_encode(&get_hash_from_path(&path).unwrap()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn file_info_reads_image_dimensions() {
        let path = unique_path("info_image", "png");

        let image = DynamicImage::new_rgb8(12, 7);
        image.save_with_format(&path, ImageFormat::Png).unwrap();

        let info = get_file_info(&path, None, false).unwrap();

        assert_eq!(info.mime, Mime::Png);
        assert_eq!(info.width, Some(12));
        assert_eq!(info.height, Some(7));
        assert!(!info.has_audio);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn default_thumbnail_paths_are_populated() {
        initialise_mimes_to_default_thumbnail_paths();

        assert!(default_thumbnail_path(Mime::Jpeg).ends_with("image.png"));
        assert!(default_thumbnail_path(Mime::Mp3).ends_with("audio.png"));
        assert!(default_thumbnail_path(Mime::Mp4Video).ends_with("video.png"));
        assert!(default_thumbnail_path(Mime::Zip).ends_with("zip.png"));
        assert!(default_thumbnail_path(Mime::Unknown).ends_with("hydrus.png"));
    }

    #[test]
    fn thumbnail_mime_only_reports_jpeg_or_png() {
        let path = write_bytes("thumb_mime", "png", b"\x89PNG\r\n\x1a\nrest");

        assert_eq!(get_thumbnail_mime(&path).unwrap(), Mime::Png);
        assert!(get_is_png_real_quick(&path).unwrap());

        std::fs::remove_file(path).ok();
    }
}
