//! Base64 encoding helpers.

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(super) fn base64_encode(bytes: &[u8], url_safe: bool, pad: bool) -> String {
    let mut encoded = String::new();

    for chunk in bytes.chunks(3) {
        let value = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;

        let length = encoded.len();

        for index in 0..4 {
            let char_index = ((value >> (18 - index * 6)) & 0x3F) as usize;
            encoded.push(BASE64_ALPHABET[char_index] as char);
        }

        match chunk.len() {
            1 => {
                encoded.truncate(length + 2);
                if pad {
                    encoded.push_str("==");
                }
            }
            2 => {
                encoded.truncate(length + 3);
                if pad {
                    encoded.push('=');
                }
            }
            _ => {}
        }
    }

    if url_safe {
        encoded = encoded.replace('+', "-").replace('/', "_");
    }

    encoded
}

pub(super) fn base64_decode(text: &str, url_safe: bool) -> String {
    let cleaned: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=')
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .collect();

    let mut bytes = Vec::new();

    for chunk in cleaned.chunks(4) {
        if chunk.len() == 1 {
            break;
        }

        let mut value = 0u32;

        for (index, byte) in chunk.iter().enumerate() {
            let position = BASE64_ALPHABET
                .iter()
                .position(|candidate| candidate == byte)
                .unwrap_or(0) as u32;

            value |= position << (18 - index * 6);
        }

        bytes.push((value >> 16) as u8);

        if chunk.len() > 2 {
            bytes.push((value >> 8) as u8);
        }
        if chunk.len() > 3 {
            bytes.push(value as u8);
        }
    }

    let _ = url_safe;

    String::from_utf8_lossy(&bytes).into_owned()
}
