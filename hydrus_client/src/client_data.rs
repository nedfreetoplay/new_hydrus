//! Shared client helpers from `hydrus/client/ClientData.py`.

use hydrus_core::hydrus_constants;
use hydrus_core::hydrus_data;

pub fn merge_counts(min_a: i64, max_a: i64, min_b: i64, max_b: i64) -> (i64, i64) {
    (min_a.max(min_b), max_a + max_b)
}

pub fn convert_zoom_to_percentage(zoom: f64) -> String {
    let zoom_percent = zoom * 100.0;
    let pretty = format!("{zoom_percent:.2}%");

    if pretty.ends_with("00%") {
        format!("{zoom_percent:.0}%")
    } else {
        pretty
    }
}

pub fn ord_is_sensible_ascii(o: u8) -> bool {
    (32..=127).contains(&o)
}

pub fn ord_is_alpha_lower(o: u8) -> bool {
    o.is_ascii_lowercase()
}

pub fn ord_is_alpha_upper(o: u8) -> bool {
    o.is_ascii_uppercase()
}

pub fn ord_is_alpha(o: u8) -> bool {
    o.is_ascii_alphabetic()
}

pub fn ord_is_number(o: u8) -> bool {
    o.is_ascii_digit()
}

/// `resolution_to_pretty_string`
pub fn resolution_to_pretty_string(resolution: Option<(u32, u32)>, use_nice: bool) -> String {
    let Some((width, height)) = resolution else {
        return "no resolution".to_string();
    };

    if use_nice {
        if let Some(nice) = hydrus_constants::nice_resolution((width, height)) {
            return nice.to_string();
        }
    }

    format!(
        "{}x{}",
        hydrus_core::hydrus_numbers::base_to_human_int(width),
        hydrus_core::hydrus_numbers::base_to_human_int(height)
    )
}

pub fn to_human_bytes(size: Option<u64>, sig_figs: usize) -> String {
    hydrus_data::base_to_human_bytes(size, sig_figs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Booru {
    pub name: String,
    pub search_url: String,
    pub search_separator: String,
    pub advance_by_page_num: bool,
    pub thumb_classname: String,
    pub image_id: String,
    pub image_data: String,
    pub tag_classnames_to_namespaces: Vec<(String, String)>,
}

impl Booru {
    pub fn get_data(&self) -> (&str, &str, bool, &str, &str, &str, &[(String, String)]) {
        (
            &self.search_url,
            &self.search_separator,
            self.advance_by_page_num,
            &self.thumb_classname,
            &self.image_id,
            &self.image_data,
            &self.tag_classnames_to_namespaces,
        )
    }

    pub fn get_gallery_parsing_info(&self) -> (&str, bool, &str, &str) {
        (
            &self.search_url,
            self.advance_by_page_num,
            &self.search_separator,
            &self.thumb_classname,
        )
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_namespaces(&self) -> Vec<&str> {
        self.tag_classnames_to_namespaces
            .iter()
            .map(|(_, namespace)| namespace.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Credentials {
    host: String,
    port: u16,
    access_key: Option<Vec<u8>>,
}

impl Credentials {
    pub fn new(host: impl Into<String>, port: u16, access_key: Option<Vec<u8>>) -> Self {
        let host = host.into();
        let host = if host == "localhost" {
            "127.0.0.1".to_string()
        } else {
            host
        };

        Self {
            host,
            port,
            access_key,
        }
    }

    pub fn get_access_key(&self) -> Option<&[u8]> {
        self.access_key.as_deref()
    }

    pub fn get_address(&self) -> (&str, u16) {
        (&self.host, self.port)
    }

    pub fn get_connection_string(&self) -> String {
        let mut connection_string = String::new();

        if let Some(access_key) = &self.access_key {
            connection_string.push_str(&hex_encode(access_key));
            connection_string.push('@');
        }

        connection_string.push_str(&self.host);
        connection_string.push(':');
        connection_string.push_str(&self.port.to_string());

        connection_string
    }

    pub fn get_ported_address(&self) -> String {
        let host = self.host.trim_end_matches('/');

        if let Some((actual_host, gubbins)) = host.split_once('/') {
            format!("{actual_host}:{}/{gubbins}", self.port)
        } else {
            format!("{host}:{}", self.port)
        }
    }

    pub fn has_access_key(&self) -> bool {
        self.access_key.as_ref().is_some_and(|key| !key.is_empty())
    }

    pub fn set_access_key(&mut self, access_key: Option<Vec<u8>>) {
        self.access_key = access_key;
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imageboard {
    pub name: String,
    pub post_url: String,
    pub flood_time: i64,
    pub form_fields: Vec<(String, String)>,
    pub restrictions: Vec<(String, String)>,
}

impl Imageboard {
    pub fn is_ok_to_post(&self) -> bool {
        true
    }

    pub fn get_board_info(&self) -> (&str, i64, &[(String, String)], &[(String, String)]) {
        (
            &self.post_url,
            self.flood_time,
            &self.form_fields,
            &self.restrictions,
        )
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests;
