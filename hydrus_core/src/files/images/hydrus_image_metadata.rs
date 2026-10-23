//! Rust port of `hydrus/core/files/images/HydrusImageMetadata.py`.
//!
//! Python reads `PILImage.info`, the merged EXIF dictionary, JPEG quantization
//! tables, and the JPEG sampling mode. The Rust `image` crate intentionally does
//! not expose those PIL-specific structures, so this port keeps the metadata
//! logic and represents the inputs with [`MetadataValue`] and [`ImageMetadata`].
//! File-specific extractors can populate those values without changing the
//! rendering, washing, and quality-estimation code.
//!
//! The pure rendering functions (`render_dict`, `render_key_value`,
//! `render_char_card`) are a complete port. [`read_png_metadata`] fills the
//! PIL-style info dictionary for PNGs, including gamma, chromaticity, sRGB,
//! ICC, text chunks, and EXIF orientation; callers can populate
//! [`ImageMetadata`] directly for other formats.

use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use base64::Engine;
use flate2::read::ZlibDecoder;
use serde_json::Value;

use crate::hydrus_data;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_lists;

pub type MetadataMap = BTreeMap<String, MetadataValue>;

/// A value from an image information / EXIF dictionary.
///
/// This is deliberately richer than JSON because PIL often stores raw bytes and
/// integer tuples in `info`.
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<MetadataValue>),
    Dict(MetadataMap),
}

impl MetadataValue {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    fn is_empty_collection(&self) -> bool {
        match self {
            Self::List(items) => items.is_empty(),
            Self::Dict(items) => items.is_empty(),
            _ => false,
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            Self::Int(value) => *value == 0,
            Self::Float(value) => *value == 0.0,
            _ => false,
        }
    }
}

impl std::fmt::Display for MetadataValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(formatter, "None"),
            Self::Bool(true) => write!(formatter, "True"),
            Self::Bool(false) => write!(formatter, "False"),
            Self::Int(value) => write!(formatter, "{value}"),
            // `Debug` keeps a trailing `.0`, matching Python's `str( float )`.
            Self::Float(value) => write!(formatter, "{value:?}"),
            Self::Text(value) => write!(formatter, "{value}"),
            Self::Bytes(value) => write!(
                formatter,
                "{} of data",
                hydrus_data::to_human_bytes(value.len() as u64)
            ),
            Self::List(items) => {
                write!(formatter, "[")?;

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }

                    write!(formatter, "{}", item.python_repr())?;
                }

                write!(formatter, "]")
            }
            Self::Dict(items) => {
                write!(formatter, "{{")?;

                for (index, (key, value)) in items.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }

                    write!(formatter, "'{key}': {}", value.python_repr())?;
                }

                write!(formatter, "}}")
            }
        }
    }
}

impl MetadataValue {
    fn python_repr(&self) -> String {
        match self {
            Self::Text(value) => format!("'{}'", value.replace('\'', "\\'")),
            _ => self.to_string(),
        }
    }
}

impl From<&Value> for MetadataValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Bool(*value),
            Value::Number(number) => {
                if let Some(value) = number.as_i64() {
                    Self::Int(value)
                } else {
                    Self::Float(number.as_f64().unwrap_or(0.0))
                }
            }
            Value::String(value) => Self::Text(value.clone()),
            Value::Array(items) => {
                Self::List(items.iter().map(MetadataValue::from).collect())
            }
            Value::Object(items) => Self::Dict(
                items
                    .iter()
                    .map(|(key, value)| (key.clone(), MetadataValue::from(value)))
                    .collect(),
            ),
        }
    }
}

/// `render_char_card( chara_text, indent_depth )`
pub fn render_char_card(chara_text: &str, indent_depth: usize) -> String {
    let card_json = base64::engine::general_purpose::STANDARD
        .decode(chara_text)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());

    let Some(card_json) = card_json else {
        return render_key_value(
            indent_depth,
            "chara",
            &MetadataValue::Text(chara_text.to_string()),
            &[],
        );
    };

    if let Value::Object(card) = &card_json {
        let has_v2_spec = card
            .get("spec")
            .and_then(Value::as_str)
            .is_some_and(|value| value == "chara_card_v2")
            || card
                .get("spec_version")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "2.0");

        if has_v2_spec {
            return render_key_value(
                indent_depth,
                "Character Card",
                &MetadataValue::from(&card_json),
                &["name", "description", "personality"],
            );
        }
    }

    render_key_value(
        indent_depth,
        "Character Card?",
        &MetadataValue::from(&card_json),
        &[],
    )
}

/// `render_dict( d, indent_depth, keys_to_put_at_the_top = None )`
pub fn render_dict(
    dictionary: &MetadataMap,
    indent_depth: usize,
    keys_to_put_at_the_top: &[&str],
) -> Option<String> {
    let sort_key = |key: &&String| {
        if let Some(position) = keys_to_put_at_the_top
            .iter()
            .position(|top_key| *top_key == key.as_str())
        {
            (0_u8, position, (*key).clone())
        } else {
            (1_u8, 0, (*key).clone())
        }
    };

    let mut keys: Vec<&String> = dictionary.keys().collect();
    keys.sort_by_key(sort_key);

    let mut texts = Vec::new();

    if indent_depth == 0 && keys.iter().any(|key| key.as_str() == "chara") {
        keys.retain(|key| key.as_str() != "chara");

        if let Some(chara) = dictionary.get("chara") {
            let chara_text = match chara {
                MetadataValue::Text(text) => text.clone(),
                _ => chara.to_string(),
            };

            texts.push(render_char_card(&chara_text, indent_depth));
        }
    }

    for key in keys {
        let value = &dictionary[key];

        if matches!(value, MetadataValue::Bytes(_)) {
            continue;
        }

        texts.push(render_key_value(
            indent_depth,
            key,
            value,
            keys_to_put_at_the_top,
        ));
    }

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

/// `render_key_value( indent_depth, key, value, keys_to_put_at_the_top = None )`
pub fn render_key_value(
    indent_depth: usize,
    key: &str,
    value: &MetadataValue,
    keys_to_put_at_the_top: &[&str],
) -> String {
    let indent = "    ";

    let value_string = match value {
        MetadataValue::Dict(dictionary) => {
            render_dict(
                dictionary,
                indent_depth + 1,
                keys_to_put_at_the_top,
            )
            .unwrap_or_else(|| {
                format!(
                    "{}empty/unknown",
                    indent.repeat(indent_depth + 1)
                )
            })
        }
        MetadataValue::Bytes(_) => {
            format!(
                "{}{}",
                indent.repeat(indent_depth + 1),
                value
            )
        }
        _ => {
            let raw_value = value.to_string();

            raw_value
                .lines()
                .map(|line| format!("{}{}", indent.repeat(indent_depth + 1), line))
                .collect::<Vec<_>>()
                .join("\n")
        }
    };

    format!(
        "{}{}:\n{}",
        indent.repeat(indent_depth),
        key,
        value_string
    )
}

/// `get_embedded_file_text( pil_image )`, given the already-collected info map.
pub fn get_embedded_file_text(info: &MetadataMap) -> Option<String> {
    let info_dict = wash_pil_image_info_dict_for_human_readable_metadata(info);

    render_dict(&info_dict, 0, &[])
}

/// `has_exif( pil_image )`, given the already-collected EXIF map.
pub fn has_exif(format: &str, exif: Option<&MetadataMap>) -> bool {
    let exif_is_readable = matches!(
        format,
        "JPEG" | "JXL" | "TIFF" | "PNG" | "WEBP" | "HEIF" | "AVIF" | "MPO"
    );

    exif_is_readable && exif.is_some_and(|exif| !exif.is_empty())
}

/// `has_human_readable_embedded_metadata( pil_image )`, given the info map.
pub fn has_human_readable_embedded_metadata(info: &MetadataMap) -> bool {
    get_embedded_file_text(info).is_some()
}

const SUBSAMPLING_444: u8 = 0;
const SUBSAMPLING_422: u8 = 1;
const SUBSAMPLING_420: u8 = 2;
const SUBSAMPLING_UNKNOWN: u8 = 3;
const SUBSAMPLING_GREYSCALE: u8 = 4;

/// `subsampling_quality_lookup`
fn subsampling_quality(subsampling: u8) -> f64 {
    match subsampling {
        SUBSAMPLING_444 => 1.00,
        SUBSAMPLING_422 => 0.93,
        SUBSAMPLING_420 => 0.83,
        SUBSAMPLING_UNKNOWN => 0.75,
        SUBSAMPLING_GREYSCALE => 0.967,
        _ => 0.75,
    }
}

/// `subsampling_str_lookup[ subsampling ]`
pub fn subsampling_as_str(subsampling: u8) -> &'static str {
    match subsampling {
        SUBSAMPLING_444 => "4:4:4",
        SUBSAMPLING_422 => "4:2:2",
        SUBSAMPLING_420 => "4:2:0",
        SUBSAMPLING_GREYSCALE => "greyscale (no subsampling)",
        _ => "unknown",
    }
}

/// `subsampling_misc_object_to_enum_lookup`
pub fn subsampling_from_misc_object(value: &MetadataValue) -> Option<u8> {
    let text = match value {
        MetadataValue::Int(value) => value.to_string(),
        MetadataValue::Text(value) => value.clone(),
        _ => return None,
    };

    match text.as_str() {
        "420" | "2" => Some(SUBSAMPLING_420),
        "422" | "1" => Some(SUBSAMPLING_422),
        "444" | "0" => Some(SUBSAMPLING_444),
        _ => None,
    }
}

/// `get_chroma_subsampling_from_pil_info( pil_image )`
pub fn get_chroma_subsampling_from_pil_info(info: &MetadataMap) -> u8 {
    info.get("chroma")
        .and_then(subsampling_from_misc_object)
        .unwrap_or(SUBSAMPLING_UNKNOWN)
}

/// `get_jpeg_subsampling_raw( pil_image )`
pub fn get_jpeg_subsampling_raw(mode: &str, sampling: Option<u8>) -> u8 {
    if mode == "L" {
        return SUBSAMPLING_GREYSCALE;
    }

    match sampling {
        Some(value @ 0..=2) => value,
        _ => SUBSAMPLING_UNKNOWN,
    }
}

static SOFTWARE_PATTERNS: LazyLock<[regex::Regex; 2]> = LazyLock::new(|| {
    [
        regex::Regex::new(
            r"^(Created|Converted|Cropped|Compressed|Edited) with (?P<software>.+)",
        )
        .expect("the software comment regex is valid"),
        regex::Regex::new(
            r"^(created|converted|cropped|compressed|edited) with (?P<software>.+)",
        )
        .expect("the software comment regex is valid"),
    ]
});

/// `get_software_source_from_comment_info_field( value )`
pub fn get_software_source_from_comment_info_field(
    value: &MetadataValue,
) -> Option<String> {
    let MetadataValue::Text(value) = value else {
        return None;
    };

    for pattern in SOFTWARE_PATTERNS.iter() {
        if let Some(captures) = pattern.captures(value) {
            return captures.name("software").map(|software| software.as_str().to_string());
        }
    }

    None
}

/// `get_software_source_from_pil_info( pil_image )`
pub fn get_software_source_from_pil_info(info: &MetadataMap) -> Option<String> {
    let mut components = Vec::new();

    for key in ["Software", "software"] {
        if let Some(value) = info.get(key) {
            components.push(value.to_string());
        }
    }

    if let Some(value) = info.get("Comment").or_else(|| info.get("comment")) {
        if let Some(software) = get_software_source_from_comment_info_field(value) {
            components.push(software);
        }
    }

    for key in ["Creator", "creator", "Source", "source"] {
        if let Some(value) = info.get(key) {
            components.push(value.to_string());
        }
    }

    // These two appends are in the Python source; dedupe below removes the
    // resulting duplicates.
    if let Some(value) = info.get("Creator") {
        components.push(value.to_string());
    }

    if let Some(value) = info.get("Source") {
        components.push(value.to_string());
    }

    let components: Vec<String> = components
        .into_iter()
        .map(|component| component.trim().to_string())
        .filter(|component| !component.is_empty())
        .collect();

    if components.is_empty() {
        return None;
    }

    Some(hydrus_lists::dedupe_list(components).join(" / "))
}

/// `has_software_source( pil_image )`, given the info map.
pub fn has_software_source(info: &MetadataMap) -> bool {
    get_software_source_from_pil_info(info).is_some()
}

/// `has_icc_profile( pil_image )`
pub fn has_icc_profile(info: &MetadataMap) -> bool {
    info.get("icc_profile")
        .is_some_and(|value| matches!(value, MetadataValue::Bytes(bytes) if !bytes.is_empty()))
}

/// `get_icc_profile_bytes( pil_image )`
pub fn get_icc_profile_bytes(info: &MetadataMap) -> HydrusResult<Vec<u8>> {
    match info.get("icc_profile") {
        Some(MetadataValue::Bytes(bytes)) if !bytes.is_empty() => Ok(bytes.clone()),
        _ => Err(HydrusException::new(
            ErrorKind::DataMissing,
            "This image has no ICC profile!",
        )),
    }
}

/// `get_jpeg_quantization_quality_estimate( pil_image )`
///
/// The tables are supplied directly so this can be used by any JPEG metadata
/// reader. An empty slice behaves like PIL's empty `quantization` dictionary.
pub fn get_jpeg_quantization_quality_estimate(
    tables: Option<&[Vec<u16>]>,
    subsampling: u8,
) -> (&'static str, Option<f64>) {
    let Some(tables) = tables else {
        return ("unknown", None);
    };

    if tables.is_empty() {
        return ("unknown", None);
    }

    let average_table_sum =
        tables.iter().map(|table| table.iter().map(|v| *v as f64).sum::<f64>()).sum::<f64>()
            / tables.len() as f64;

    let quality = average_table_sum.powf(1.0 / subsampling_quality(subsampling));

    let label = if quality >= 2800.0 {
        "very low"
    } else if quality >= 2000.0 {
        "low"
    } else if quality >= 1400.0 {
        "medium low"
    } else if quality >= 1000.0 {
        "medium"
    } else if quality >= 700.0 {
        "medium high"
    } else if quality >= 480.0 {
        "high"
    } else if quality >= 330.0 {
        "very high"
    } else {
        "extremely high"
    };

    (label, Some(quality))
}

/// Keys that are parsed and displayed elsewhere rather than in the generic
/// human-readable metadata view.
static PIL_INFO_KEYS_THAT_ARE_NOT_CONSIDERED_HUMAN_READABLE_STUFF: LazyLock<
    HashSet<&'static str>,
> = LazyLock::new(|| {
    [
        "exif",
        "Raw profile type exif",
        "icc_profile",
        "progression",
        "progressive",
        "srgb",
        "gamma",
        "chromaticity",
        "dpi",
        "jfif",
        "jfif_unit",
        "jfif_density",
        "jfif_version",
        "compression",
        "resolution",
        "Software",
        "software",
        "adobe",
        "adobe_transform",
        "transparency",
        "background",
        "duration",
        "bit_depth",
        "primary",
        "chroma",
        "loop",
        "photoshop",
        "extension",
        "bbox",
        "blend",
        "disposal",
        "sizes",
        "interlace",
        "aspect",
        "xmp",
        "XML:com.adobe.xmp",
        "iptc",
        "Raw profile type iptc",
        "default_image",
        "distortion",
        "Creator",
        "creator",
        "Source",
        "source",
        "mpoffset",
        "Creation Time",
        "create-date",
        "modify-date",
        "date:create",
        "date:modify",
        "date:timestamp",
        "Thumb::MTime",
    ]
    .into_iter()
    .collect()
});

/// `wash_pil_image_info_dict_for_human_readable_metadata( info_dict )`
pub fn wash_pil_image_info_dict_for_human_readable_metadata(
    info_dict: &MetadataMap,
) -> MetadataMap {
    let mut new_info_dict = MetadataMap::new();

    for (key, value) in info_dict {
        if PIL_INFO_KEYS_THAT_ARE_NOT_CONSIDERED_HUMAN_READABLE_STUFF.contains(key.as_str()) {
            continue;
        }

        if matches!(value, MetadataValue::Null) || value.is_empty_collection() {
            continue;
        }

        if (key == "comment" || key == "Comment")
            && get_software_source_from_comment_info_field(value).is_some()
        {
            continue;
        }

        if key == "timestamp" && value.is_zero() {
            continue;
        }

        if key.starts_with("Thumb::") {
            continue;
        }

        new_info_dict.insert(key.clone(), value.clone());
    }

    new_info_dict
}

/// The PIL image properties the Python metadata module needs.
///
/// Populate this from a file-specific metadata reader. `format` and `mode` are
/// the PIL-style names ("PNG", "RGB", and so on); `quantization` holds the JPEG
/// tables; `sampling` is PIL's `JpegImagePlugin.get_sampling` result.
#[derive(Clone, Debug, Default)]
pub struct ImageMetadata {
    pub info: MetadataMap,
    pub exif: Option<MetadataMap>,
    pub format: String,
    pub mode: String,
    pub quantization: Option<Vec<Vec<u16>>>,
    pub sampling: Option<u8>,
}

impl ImageMetadata {
    pub fn has_exif(&self) -> bool {
        has_exif(&self.format, self.exif.as_ref())
    }

    pub fn has_human_readable_embedded_metadata(&self) -> bool {
        has_human_readable_embedded_metadata(&self.info)
    }

    pub fn has_icc_profile(&self) -> bool {
        has_icc_profile(&self.info)
    }

    pub fn get_icc_profile_bytes(&self) -> HydrusResult<Vec<u8>> {
        get_icc_profile_bytes(&self.info)
    }

    pub fn has_software_source(&self) -> bool {
        has_software_source(&self.info)
    }

    pub fn get_embedded_file_text(&self) -> Option<String> {
        get_embedded_file_text(&self.info)
    }

    pub fn get_software_source(&self) -> Option<String> {
        get_software_source_from_pil_info(&self.info)
    }

    pub fn get_chroma_subsampling(&self) -> u8 {
        get_chroma_subsampling_from_pil_info(&self.info)
    }

    pub fn get_jpeg_subsampling(&self) -> u8 {
        get_jpeg_subsampling_raw(&self.mode, self.sampling)
    }

    pub fn get_jpeg_quantization_quality_estimate(
        &self,
    ) -> (&'static str, Option<f64>) {
        let subsampling = self.get_jpeg_subsampling();

        get_jpeg_quantization_quality_estimate(
            self.quantization.as_deref(),
            subsampling,
        )
    }
}

fn metadata_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn png_u32(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_be_bytes)
}

fn decode_png_zlib(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut output = Vec::new();

    decoder.read_to_end(&mut output).ok()?;

    Some(output)
}

fn decode_png_latin1(data: &[u8]) -> String {
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(data);

    text.into_owned()
}

fn split_png_keyword(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = data.iter().position(|byte| *byte == 0)?;

    Some((&data[..separator], &data[separator + 1..]))
}

/// Read the EXIF orientation out of the TIFF payload used by PNG's `eXIf`
/// chunk. This mirrors the one EXIF field the Python module rotates for.
fn exif_orientation_from_tiff(data: &[u8]) -> Option<u16> {
    let tiff = data.strip_prefix(b"Exif\0\0").unwrap_or(data);

    let (little_endian, offset) = match tiff.get(..4)? {
        b"II*\0" => (true, 4_usize),
        b"MM\0*" => (false, 4_usize),
        _ => return None,
    };

    let read_u16 = |bytes: &[u8]| {
        let bytes: [u8; 2] = bytes.try_into().ok()?;

        Some(if little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    };

    let read_u32 = |bytes: &[u8]| {
        let bytes: [u8; 4] = bytes.try_into().ok()?;

        Some(if little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    };

    let ifd_offset = read_u32(tiff.get(offset..offset + 4)?)? as usize;
    let count = read_u16(tiff.get(ifd_offset..ifd_offset + 2)?)? as usize;

    for index in 0..count {
        let entry_offset = ifd_offset + 2 + (index * 12);
        let entry = tiff.get(entry_offset..entry_offset + 12)?;

        let tag = read_u16(&entry[0..2])?;

        if tag != 274 {
            continue;
        }

        let field_type = read_u16(&entry[2..4])?;
        let value_count = read_u32(&entry[4..8])?;

        if field_type != 3 || value_count == 0 {
            return None;
        }

        return read_u16(&entry[8..10]);
    }

    None
}

/// Read the PNG information dictionary, JPEG-style quantization tables, and
/// EXIF orientation without requiring PIL.
///
/// `image` crate decoders expose pixels but not the original metadata
/// dictionaries. This parser fills the subset of `PILImage.info` that hydrus
/// uses: `gamma`, `chromaticity`, `srgb`, `icc_profile`, text chunks, `exif`,
/// and the image's format and mode.
pub fn read_png_metadata(path: impl AsRef<Path>) -> HydrusResult<ImageMetadata> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not read the PNG at \"{}\": {error}", path.display()),
        )
    })?;

    read_png_metadata_bytes(&bytes)
}

/// Byte-buffer version of [`read_png_metadata`].
pub fn read_png_metadata_bytes(bytes: &[u8]) -> HydrusResult<ImageMetadata> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err(HydrusException::new(
            ErrorKind::UnsupportedFile,
            "This file is not a PNG.",
        ));
    }

    let mut metadata = ImageMetadata {
        format: "PNG".to_string(),
        ..ImageMetadata::default()
    };

    let mut offset = PNG_SIGNATURE.len();
    let mut saw_ihdr = false;

    while offset + 12 <= bytes.len() {
        let length = png_u32(bytes, offset)
            .ok_or_else(|| metadata_error("The PNG has a truncated chunk length."))?
            as usize;

        let chunk_end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| metadata_error("The PNG chunk length overflowed."))?;

        if chunk_end > bytes.len() {
            return Err(metadata_error("The PNG has a truncated chunk."));
        }

        let chunk_type: [u8; 4] = bytes[offset + 4..offset + 8]
            .try_into()
            .expect("four bytes were sliced");
        let data = &bytes[offset + 8..offset + 8 + length];

        match &chunk_type {
            b"IHDR" => {
                if data.len() < 10 {
                    return Err(metadata_error("The PNG IHDR chunk is truncated."));
                }

                let bit_depth = data[8];
                let color_type = data[9];

                metadata.mode = match color_type {
                    0 if bit_depth == 16 => "I;16".to_string(),
                    0 => "L".to_string(),
                    2 if bit_depth == 16 => "RGB".to_string(),
                    2 => "RGB".to_string(),
                    3 => "P".to_string(),
                    4 => "LA".to_string(),
                    6 => "RGBA".to_string(),
                    _ => "RGB".to_string(),
                };

                saw_ihdr = true;
            }
            b"gAMA" => {
                if let Some(value) = png_u32(data, 0) {
                    metadata.info.insert(
                        "gamma".to_string(),
                        MetadataValue::Float(f64::from(value) / 100_000.0),
                    );
                }
            }
            b"cHRM" => {
                if data.len() >= 32 {
                    let values: Vec<MetadataValue> = (0..8)
                        .filter_map(|index| png_u32(data, index * 4))
                        .map(|value| MetadataValue::Float(f64::from(value) / 100_000.0))
                        .collect();

                    if values.len() == 8 {
                        metadata
                            .info
                            .insert("chromaticity".to_string(), MetadataValue::List(values));
                    }
                }
            }
            b"sRGB" => {
                if let Some(value) = data.first() {
                    metadata
                        .info
                        .insert("srgb".to_string(), MetadataValue::Int(i64::from(*value)));
                }
            }
            b"iCCP" => {
                if let Some((name, rest)) = split_png_keyword(data) {
                    if let Some((&compression_method, compressed)) = rest.split_first() {
                        if compression_method == 0 {
                            if let Some(profile) = decode_png_zlib(compressed) {
                                metadata.info.insert(
                                    "profile".to_string(),
                                    MetadataValue::Text(decode_png_latin1(name)),
                                );
                                metadata.info.insert(
                                    "icc_profile".to_string(),
                                    MetadataValue::Bytes(profile),
                                );
                            }
                        }
                    }
                }
            }
            b"tEXt" => {
                if let Some((keyword, text)) = split_png_keyword(data) {
                    metadata.info.insert(
                        decode_png_latin1(keyword),
                        MetadataValue::Text(decode_png_latin1(text)),
                    );
                }
            }
            b"zTXt" => {
                if let Some((keyword, rest)) = split_png_keyword(data) {
                    if let Some((&compression_method, compressed)) = rest.split_first() {
                        if compression_method == 0 {
                            if let Some(text) = decode_png_zlib(compressed) {
                                metadata.info.insert(
                                    decode_png_latin1(keyword),
                                    MetadataValue::Text(decode_png_latin1(&text)),
                                );
                            }
                        }
                    }
                }
            }
            b"iTXt" => {
                let fields: Vec<&[u8]> = data.splitn(5, |byte| *byte == 0).collect();

                if fields.len() == 5 {
                    let keyword = decode_png_latin1(fields[0]);
                    let compressed = fields[1].first().copied().unwrap_or(0) != 0;
                    let text_bytes = if compressed {
                        decode_png_zlib(fields[4])
                    } else {
                        Some(fields[4].to_vec())
                    };

                    if let Some(text_bytes) = text_bytes {
                        metadata.info.insert(
                            keyword,
                            MetadataValue::Text(String::from_utf8_lossy(&text_bytes).into_owned()),
                        );
                    }
                }
            }
            b"eXIf" => {
                metadata.info.insert(
                    "exif".to_string(),
                    MetadataValue::Bytes(data.to_vec()),
                );

                if let Some(orientation) = exif_orientation_from_tiff(data) {
                    let mut exif = MetadataMap::new();
                    exif.insert(
                        "274".to_string(),
                        MetadataValue::Int(i64::from(orientation)),
                    );
                    metadata.exif = Some(exif);
                }
            }
            b"pHYs" => {
                if data.len() >= 9 {
                    let x = png_u32(data, 0).unwrap_or(0);
                    let y = png_u32(data, 4).unwrap_or(0);
                    let unit = data[8];

                    if unit == 1 {
                        metadata.info.insert(
                            "dpi".to_string(),
                            MetadataValue::List(vec![
                                MetadataValue::Float(f64::from(x) * 0.0254),
                                MetadataValue::Float(f64::from(y) * 0.0254),
                            ]),
                        );
                    }
                }
            }
            b"IEND" => break,
            _ => {}
        }

        offset = chunk_end;
    }

    if !saw_ihdr {
        return Err(metadata_error("The PNG has no IHDR chunk."));
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, MetadataValue)]) -> MetadataMap {
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn dict_rendering_sorts_and_indents() {
        let dictionary = map(&[
            ("zeta", MetadataValue::text("last")),
            ("alpha", MetadataValue::text("first")),
        ]);

        assert_eq!(
            render_dict(&dictionary, 0, &[]).unwrap(),
            "alpha:\n    first\nzeta:\n    last"
        );
    }

    #[test]
    fn nested_empty_dicts_are_marked_unknown() {
        let dictionary = map(&[("empty", MetadataValue::Dict(MetadataMap::new()))]);

        assert_eq!(
            render_dict(&dictionary, 0, &[]).unwrap(),
            "empty:\n    empty/unknown"
        );
    }

    #[test]
    fn character_cards_are_decoded_and_put_at_the_top() {
        let card = serde_json::json!({
            "spec": "chara_card_v2",
            "spec_version": "2.0",
            "name": "A",
            "description": "B",
            "personality": "C",
            "other": "D"
        });
        let encoded = base64::engine::general_purpose::STANDARD.encode(card.to_string());
        let dictionary = map(&[("chara", MetadataValue::text(encoded))]);

        let rendered = render_dict(&dictionary, 0, &[]).unwrap();

        assert!(rendered.starts_with("Character Card:"));
        assert!(rendered.find("name:").unwrap() < rendered.find("other:").unwrap());
    }

    #[test]
    fn non_json_chara_falls_back_to_a_plain_key_value() {
        let dictionary = map(&[("chara", MetadataValue::text("not base64"))]);

        assert_eq!(
            render_dict(&dictionary, 0, &[]).unwrap(),
            "chara:\n    not base64"
        );
    }

    #[test]
    fn washing_removes_decoder_internals_but_keeps_comments() {
        let info = map(&[
            ("icc_profile", MetadataValue::Bytes(vec![1, 2, 3])),
            ("gamma", MetadataValue::Float(0.45455)),
            ("Comment", MetadataValue::text("hello")),
            ("timestamp", MetadataValue::Int(0)),
            ("Thumb::URI", MetadataValue::text("file:///x")),
            ("Custom", MetadataValue::text("keep me")),
        ]);

        let washed = wash_pil_image_info_dict_for_human_readable_metadata(&info);

        assert_eq!(washed.len(), 2);
        assert_eq!(washed["Comment"], MetadataValue::text("hello"));
        assert_eq!(washed["Custom"], MetadataValue::text("keep me"));
    }

    #[test]
    fn software_comments_are_extracted_and_deduplicated() {
        let info = map(&[
            ("Software", MetadataValue::text("GIMP")),
            ("Comment", MetadataValue::text("Edited with GIMP")),
            ("Creator", MetadataValue::text("GIMP")),
        ]);

        assert_eq!(
            get_software_source_from_pil_info(&info).as_deref(),
            Some("GIMP")
        );
    }

    #[test]
    fn chroma_subsampling_accepts_strings_and_numbers() {
        assert_eq!(
            get_chroma_subsampling_from_pil_info(&map(&[(
                "chroma",
                MetadataValue::text("420")
            )])),
            SUBSAMPLING_420
        );

        assert_eq!(
            get_chroma_subsampling_from_pil_info(&map(&[(
                "chroma",
                MetadataValue::Int(422)
            )])),
            SUBSAMPLING_422
        );
    }

    #[test]
    fn jpeg_quality_estimate_uses_the_subsampling_exponent() {
        let tables = vec![vec![1_u16; 64]];

        let (label, quality) =
            get_jpeg_quantization_quality_estimate(Some(&tables), SUBSAMPLING_444);

        assert_eq!(label, "extremely high");
        assert_eq!(quality, Some(64.0));
    }

    #[test]
    fn icc_profile_helpers_require_non_empty_bytes() {
        let info = map(&[("icc_profile", MetadataValue::Bytes(vec![1]))]);

        assert!(has_icc_profile(&info));
        assert_eq!(get_icc_profile_bytes(&info).unwrap(), vec![1]);

        let empty = map(&[("icc_profile", MetadataValue::Bytes(Vec::new()))]);

        assert!(!has_icc_profile(&empty));
        assert!(get_icc_profile_bytes(&empty).is_err());
    }

    fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();

        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(chunk_type);
        chunk.extend_from_slice(data);
        chunk.extend_from_slice(&[0, 0, 0, 0]); // CRC is not read by hydrus.

        chunk
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        use std::io::Write as _;

        let mut encoder = flate2::write::ZlibEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        );

        encoder.write_all(data).unwrap();

        encoder.finish().unwrap()
    }

    fn png_with_metadata() -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();

        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2_u32.to_be_bytes());
        ihdr.extend_from_slice(&2_u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);

        bytes.extend_from_slice(&png_chunk(b"IHDR", &ihdr));

        let mut chrm = Vec::new();

        for value in [31270_u32, 32900, 64000, 33000, 30000, 60000, 15000, 6000] {
            chrm.extend_from_slice(&value.to_be_bytes());
        }

        bytes.extend_from_slice(&png_chunk(b"cHRM", &chrm));
        bytes.extend_from_slice(&png_chunk(b"gAMA", &45455_u32.to_be_bytes()));
        bytes.extend_from_slice(&png_chunk(b"sRGB", &[0]));
        bytes.extend_from_slice(&png_chunk(b"tEXt", b"Comment\0hello"));
        bytes.extend_from_slice(&png_chunk(
            b"iCCP",
            &[b"profile\0".as_slice(), &[0], &zlib(b"icc bytes")].concat(),
        ));

        // Little-endian TIFF with one orientation entry (274 = 6).
        let mut exif = b"II*\0".to_vec();
        exif.extend_from_slice(&8_u32.to_le_bytes());
        exif.extend_from_slice(&1_u16.to_le_bytes());
        exif.extend_from_slice(&274_u16.to_le_bytes());
        exif.extend_from_slice(&3_u16.to_le_bytes());
        exif.extend_from_slice(&1_u32.to_le_bytes());
        exif.extend_from_slice(&6_u16.to_le_bytes());
        exif.extend_from_slice(&0_u16.to_le_bytes());
        exif.extend_from_slice(&0_u32.to_le_bytes());

        bytes.extend_from_slice(&png_chunk(b"eXIf", &exif));
        bytes.extend_from_slice(&png_chunk(b"IEND", &[]));

        bytes
    }

    #[test]
    fn png_metadata_reads_gamma_chromaticity_icc_and_exif() {
        let metadata = read_png_metadata_bytes(&png_with_metadata()).unwrap();

        assert_eq!(metadata.format, "PNG");
        assert_eq!(metadata.mode, "RGB");
        assert_eq!(
            metadata.info["gamma"],
            MetadataValue::Float(0.45455)
        );
        assert_eq!(
            metadata.info["srgb"],
            MetadataValue::Int(0)
        );
        assert_eq!(metadata.info["Comment"], MetadataValue::text("hello"));
        assert_eq!(
            metadata.get_icc_profile_bytes().unwrap(),
            b"icc bytes".to_vec()
        );
        assert_eq!(metadata.exif.as_ref().unwrap()["274"], MetadataValue::Int(6));

        let chromaticity = match &metadata.info["chromaticity"] {
            MetadataValue::List(values) => values,
            other => panic!("unexpected chromaticity: {other:?}"),
        };

        assert_eq!(chromaticity.len(), 8);
        assert_eq!(chromaticity[0], MetadataValue::Float(0.3127));
    }

    #[test]
    fn non_png_metadata_is_rejected() {
        let error = read_png_metadata_bytes(b"not a png").unwrap_err();

        assert!(error.is_a(ErrorKind::UnsupportedFile));
    }
}
