//! Rust port of `hydrus/core/files/HydrusOLEHandling.py`.
//!
//! Legacy `.doc`, `.xls` and `.ppt` files are OLE compound documents. Their
//! root CLSID identifies the application, and `SummaryInformation` carries a
//! human-readable creating application plus a word count.

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

use cfb::CompoundFile;

use crate::hydrus_constants::Mime;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

const OLE_HEADER: &[u8; 8] = b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1";
const SUMMARY_INFORMATION_STREAM: &str = "/\u{5}SummaryInformation";
const PIDSI_WORD_COUNT: u32 = 0x0F;
const PIDSI_APP_NAME: u32 = 0x12;

/// `OLEFILE_OK`
pub const OLEFILE_OK: bool = true;

/// `GUID_MIMES`, uppercased as in the Python module.
pub const GUID_MIMES: &[(&str, Mime)] = &[
    ("00020900-0000-0000-C000-000000000046", Mime::Doc),
    ("00020906-0000-0000-C000-000000000046", Mime::Doc),
    ("00030003-0000-0000-C000-000000000046", Mime::Doc),
    ("F4754C9B-64F5-4B40-8AF4-679732AC0607", Mime::Doc),
    ("00044851-0000-0000-C000-000000000046", Mime::Ppt),
    ("64818D10-4F9B-11CF-86EA-00AA00B929E8", Mime::Ppt),
    ("EA7BAE70-FB3B-11CD-A903-00AA00510EA3", Mime::Ppt),
    ("CF4F55F4-8F87-4D47-80BB-5808164BB3F8", Mime::Ppt),
    ("00020810-0000-0000-C000-000000000046", Mime::Xls),
    ("00020820-0000-0000-C000-000000000046", Mime::Xls),
    ("00020830-0000-0000-C000-000000000046", Mime::Xls),
];

/// Resolve a CLSID string, case-insensitively.
pub fn mime_from_clsid(clsid: &str) -> Option<Mime> {
    let clsid = clsid.to_uppercase();

    GUID_MIMES
        .iter()
        .find(|(guid, _)| *guid == clsid)
        .map(|(_, mime)| *mime)
}

/// `is_ole_file( path )`
pub fn is_ole_file(path: impl AsRef<Path>) -> bool {
    let Ok(mut file) = std::fs::File::open(path.as_ref()) else {
        return false;
    };

    let mut header = [0u8; 8];

    file.read_exact(&mut header).is_ok() && &header == OLE_HEADER
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PropertyValue {
    AnsiString(String),
    WideString(String),
    Integer(i64),
    Unsigned(u64),
    Boolean(bool),
}

impl PropertyValue {
    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Unsigned(value) => i64::try_from(*value).ok(),
            Self::Boolean(value) => Some(i64::from(*value)),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::AnsiString(value) | Self::WideString(value) => Some(value),
            _ => None,
        }
    }
}

fn parse_ansi_string(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());

    String::from_utf8_lossy(&data[..end]).into_owned()
}

fn parse_wide_string(data: &[u8]) -> String {
    let mut units = Vec::with_capacity(data.len() / 2);

    for chunk in data.chunks_exact(2) {
        let unit = u16::from_le_bytes([chunk[0], chunk[1]]);

        if unit == 0 {
            break;
        }

        units.push(unit);
    }

    String::from_utf16_lossy(&units)
}

fn parse_property_value(data: &[u8], offset: usize) -> Option<PropertyValue> {
    let value_type = read_u32(data, offset)?;
    let value_offset = offset + 4;

    match value_type {
        2 => Some(PropertyValue::Integer(i64::from(
            read_u16(data, value_offset)? as i16,
        ))),
        3 => Some(PropertyValue::Integer(i64::from(read_i32(
            data,
            value_offset,
        )?))),
        11 => Some(PropertyValue::Boolean(read_u16(data, value_offset)? != 0)),
        18 => Some(PropertyValue::Unsigned(u64::from(read_u16(
            data,
            value_offset,
        )?))),
        19 => Some(PropertyValue::Unsigned(u64::from(read_u32(
            data,
            value_offset,
        )?))),
        30 => {
            let length = read_u32(data, value_offset)? as usize;
            let bytes = data.get(value_offset + 4..value_offset + 4 + length)?;

            Some(PropertyValue::AnsiString(parse_ansi_string(bytes)))
        }
        31 => {
            let characters = read_u32(data, value_offset)? as usize;
            let bytes =
                data.get(value_offset + 4..value_offset + 4 + characters.saturating_mul(2))?;

            Some(PropertyValue::WideString(parse_wide_string(bytes)))
        }
        _ => None,
    }
}

fn parse_property_section(data: &[u8], section_offset: usize) -> HashMap<u32, PropertyValue> {
    let mut properties = HashMap::new();

    let Some(_section_size) = read_u32(data, section_offset) else {
        return properties;
    };

    let Some(num_properties) = read_u32(data, section_offset + 4) else {
        return properties;
    };

    let property_list_offset = section_offset + 8;

    for index in 0..num_properties as usize {
        let entry_offset = property_list_offset + index * 8;

        let Some(property_id) = read_u32(data, entry_offset) else {
            break;
        };

        let Some(relative_offset) = read_u32(data, entry_offset + 4) else {
            break;
        };

        let value_offset = section_offset.saturating_add(relative_offset as usize);

        if let Some(value) = parse_property_value(data, value_offset) {
            properties.insert(property_id, value);
        }
    }

    properties
}

/// Parse an OLE property-set stream (`SummaryInformation` and friends).
pub fn parse_property_set(data: &[u8]) -> HashMap<u32, PropertyValuePublic> {
    // Keep the public return type useful without exposing the internal enum.
    let mut result = HashMap::new();

    if data.len() < 28 {
        return result;
    }

    if read_u16(data, 0) != Some(0xFFFE) {
        return result;
    }

    let Some(num_sets) = read_u32(data, 24) else {
        return result;
    };

    for set_index in 0..num_sets as usize {
        let descriptor_offset = 28 + set_index * 20;

        let Some(section_offset) = read_u32(data, descriptor_offset + 16) else {
            break;
        };

        for (id, value) in parse_property_section(data, section_offset as usize) {
            result.insert(id, PropertyValuePublic::from(value));
        }
    }

    result
}

/// Public mirror of the private property value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyValuePublic {
    String(String),
    Integer(i64),
    Unknown,
}

impl From<PropertyValue> for PropertyValuePublic {
    fn from(value: PropertyValue) -> Self {
        match value {
            PropertyValue::AnsiString(value) | PropertyValue::WideString(value) => {
                Self::String(value)
            }
            PropertyValue::Integer(value) => Self::Integer(value),
            PropertyValue::Unsigned(value) => Self::Integer(value as i64),
            PropertyValue::Boolean(value) => Self::Integer(i64::from(value)),
        }
    }
}

fn read_summary_information<F: Read + Seek>(
    compound: &mut CompoundFile<F>,
) -> Option<HashMap<u32, PropertyValue>> {
    let mut stream = compound.open_stream(SUMMARY_INFORMATION_STREAM).ok()?;

    let mut data = Vec::new();

    stream.read_to_end(&mut data).ok()?;

    let mut properties = HashMap::new();

    if data.len() < 28 || read_u16(&data, 0) != Some(0xFFFE) {
        return None;
    }

    let num_sets = read_u32(&data, 24)?;

    for set_index in 0..num_sets as usize {
        let descriptor_offset = 28 + set_index * 20;

        let section_offset = read_u32(&data, descriptor_offset + 16)?;

        properties.extend(parse_property_section(&data, section_offset as usize));
    }

    Some(properties)
}

/// `mime_From_ole_file( path )`
pub fn mime_from_ole_file(path: impl AsRef<Path>) -> Mime {
    if !is_ole_file(path.as_ref()) {
        return Mime::Unknown;
    }

    let Ok(mut compound) = cfb::open(path.as_ref()) else {
        return Mime::UndeterminedOle;
    };

    let root_clsid = compound.root_entry().clsid().to_string().to_uppercase();

    if let Some(mime) = mime_from_clsid(&root_clsid) {
        return mime;
    }

    if let Some(properties) = read_summary_information(&mut compound) {
        if let Some(creating_application) = properties
            .get(&PIDSI_APP_NAME)
            .and_then(PropertyValue::as_string)
        {
            if creating_application.starts_with("Microsoft Word") {
                return Mime::Doc;
            }

            if creating_application.starts_with("Microsoft PowerPoint") {
                return Mime::Ppt;
            }

            if creating_application.starts_with("Microsoft Excel") {
                return Mime::Xls;
            }
        }
    }

    Mime::UndeterminedOle
}

/// `office_ole_document_word_count( path )`
pub fn office_ole_document_word_count(path: impl AsRef<Path>) -> HydrusResult<Option<i64>> {
    if !is_ole_file(path.as_ref()) {
        return Err(HydrusException::new(
            ErrorKind::LimitedSupportFile,
            "File is not OLE!",
        ));
    }

    let Ok(mut compound) = cfb::open(path.as_ref()) else {
        return Ok(None);
    };

    let Some(properties) = read_summary_information(&mut compound) else {
        return Ok(None);
    };

    Ok(properties
        .get(&PIDSI_WORD_COUNT)
        .and_then(PropertyValue::as_i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn unique_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hydrus_ole_{}_{}_{}.ole",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn property_value_string(value: &str) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(&30u32.to_le_bytes());
        data.extend_from_slice(&((value.len() + 1) as u32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        data.push(0);

        while data.len() % 4 != 0 {
            data.push(0);
        }

        data
    }

    fn property_value_i32(value: i32) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&value.to_le_bytes());

        data
    }

    fn summary_information(app: Option<&str>, words: Option<i32>) -> Vec<u8> {
        let mut properties: Vec<(u32, Vec<u8>)> = Vec::new();

        if let Some(app) = app {
            properties.push((PIDSI_APP_NAME, property_value_string(app)));
        }

        if let Some(words) = words {
            properties.push((PIDSI_WORD_COUNT, property_value_i32(words)));
        }

        let mut section = Vec::new();

        section.extend_from_slice(&0u32.to_le_bytes()); // section size placeholder
        section.extend_from_slice(&(properties.len() as u32).to_le_bytes());

        let mut value_offset = 8 + properties.len() * 8;

        for (id, value) in &properties {
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&(value_offset as u32).to_le_bytes());
            value_offset += value.len();
        }

        for (_, value) in &properties {
            section.extend_from_slice(value);
        }

        let section_size = section.len() as u32;
        section[0..4].copy_from_slice(&section_size.to_le_bytes());

        let mut stream = Vec::new();

        stream.extend_from_slice(&0xFFFEu16.to_le_bytes()); // byte order
        stream.extend_from_slice(&0u16.to_le_bytes()); // version
        stream.extend_from_slice(&0u32.to_le_bytes()); // system identifier
        stream.extend_from_slice(&[0u8; 16]); // CLSID
        stream.extend_from_slice(&1u32.to_le_bytes()); // number of property sets
        stream.extend_from_slice(&[0u8; 16]); // FMTID
        stream.extend_from_slice(&48u32.to_le_bytes()); // section offset
        stream.extend_from_slice(&section);

        stream
    }

    fn build_ole(path: &Path, summary: Option<&[u8]>) {
        let mut compound = cfb::create(path).unwrap();

        if let Some(summary) = summary {
            let mut stream = compound.create_stream(SUMMARY_INFORMATION_STREAM).unwrap();
            stream.write_all(summary).unwrap();
        }

        compound.flush().unwrap();
    }

    #[test]
    fn clsid_lookup_is_case_insensitive() {
        assert_eq!(
            mime_from_clsid("00020906-0000-0000-c000-000000000046"),
            Some(Mime::Doc)
        );
        assert_eq!(
            mime_from_clsid("CF4F55F4-8F87-4D47-80BB-5808164BB3F8"),
            Some(Mime::Ppt)
        );
        assert_eq!(mime_from_clsid("not-a-guid"), None);
    }

    #[test]
    fn ole_header_is_recognised() {
        let path = unique_path("header");
        std::fs::write(&path, [OLE_HEADER.as_slice(), &[0u8; 32]].concat()).unwrap();

        assert!(is_ole_file(&path));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn non_ole_files_are_unknown() {
        let path = unique_path("not_ole");
        std::fs::write(&path, b"not an ole file").unwrap();

        assert!(!is_ole_file(&path));
        assert_eq!(mime_from_ole_file(&path), Mime::Unknown);

        let error = office_ole_document_word_count(&path).unwrap_err();
        assert!(error.is_a(ErrorKind::LimitedSupportFile));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn summary_information_gives_app_name_and_word_count() {
        let path = unique_path("summary");
        let data = summary_information(Some("Microsoft Word 97"), Some(1234));

        build_ole(&path, Some(&data));

        assert_eq!(mime_from_ole_file(&path), Mime::Doc);
        assert_eq!(office_ole_document_word_count(&path).unwrap(), Some(1234));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn an_ole_file_without_summary_is_undetermined() {
        let path = unique_path("empty");

        build_ole(&path, None);

        assert_eq!(mime_from_ole_file(&path), Mime::UndeterminedOle);
        assert_eq!(office_ole_document_word_count(&path).unwrap(), None);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn property_set_parser_reads_multiple_sections() {
        let data = summary_information(Some("Microsoft Excel"), Some(42));
        let parsed = parse_property_set(&data);

        assert_eq!(
            parsed.get(&PIDSI_APP_NAME),
            Some(&PropertyValuePublic::String("Microsoft Excel".to_string()))
        );
        assert_eq!(
            parsed.get(&PIDSI_WORD_COUNT),
            Some(&PropertyValuePublic::Integer(42))
        );
    }
}
