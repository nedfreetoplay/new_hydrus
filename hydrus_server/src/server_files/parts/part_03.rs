// The free functions below preserve the Python module API. The HTTP resource
// layer will call them once it is wired up; keeping them now means callers do
// not have to change shape later.

/// `get_all_hashes( file_type )`
#[allow(dead_code)]
pub fn get_all_hashes(file_type: ServerFileType) -> HydrusResult<HashSet<Vec<u8>>> {
    ServerFiles::from_global()?.get_all_hashes(file_type)
}

/// `get_expected_file_path( hash )`
#[allow(dead_code)]
pub fn get_expected_file_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    Ok(ServerFiles::from_global()?.get_expected_file_path(hash))
}

/// `get_expected_thumbnail_path( hash )`
#[allow(dead_code)]
pub fn get_expected_thumbnail_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    Ok(ServerFiles::from_global()?.get_expected_thumbnail_path(hash))
}

/// `get_file_path( hash )`
#[allow(dead_code)]
pub fn get_file_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    ServerFiles::from_global()?.get_file_path(hash)
}

/// `get_thumbnail_path( hash )`
#[allow(dead_code)]
pub fn get_thumbnail_path(hash: &[u8]) -> HydrusResult<PathBuf> {
    ServerFiles::from_global()?.get_thumbnail_path(hash)
}

/// `iterate_all_paths( file_type )`
#[allow(dead_code)]
pub fn iterate_all_paths(file_type: ServerFileType) -> HydrusResult<Vec<PathBuf>> {
    ServerFiles::from_global()?.iterate_all_paths(file_type)
}

fn io_error(kind: ErrorKind, context: impl std::fmt::Display, error: io::Error) -> HydrusException {
    HydrusException::new(kind, format!("{context}: {error}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }

    text
}

fn hex_decode(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }

    let mut bytes = Vec::with_capacity(text.len() / 2);

    for pair in text.as_bytes().chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;

        bytes.push((high << 4) | low);
    }

    Ok(bytes)
}

fn hex_digit(byte: u8) -> Result<u8, ()> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    }
}
