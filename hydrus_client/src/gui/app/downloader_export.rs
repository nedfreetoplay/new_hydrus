//! Export and import of downloader definitions as standalone JSON files.

use serde_json::{Value, json};

use super::model::{DownloaderKind, DownloaderPage};

pub const FORMAT: &str = "hydrus-rust-downloader";
pub const VERSION: u32 = 1;

pub fn to_json(page: &DownloaderPage) -> Value {
    json!({
        "format": FORMAT,
        "version": VERSION,
        "name": page.title,
        "kind": kind_name(page.kind),
        "url": page.url,
        "query": page.query,
        "site": page.site,
        "filter": page.filter,
        "subject": page.subject,
        "check_period": page.check_period,
        "auto_watch": page.auto_watch,
    })
}

pub fn from_json(value: &Value, id: impl Into<String>) -> Result<DownloaderPage, String> {
    if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err("this file is not a Hydrus downloader definition".to_string());
    }

    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .and_then(DownloaderKind::parse)
        .ok_or_else(|| "downloader definition has an unknown kind".to_string())?;

    let mut page = DownloaderPage::new(id, kind);

    if let Some(name) = value.get("name").and_then(Value::as_str) {
        if !name.trim().is_empty() {
            page.title = name.trim().to_string();
        }
    }

    page.url = string_field(value, "url");
    page.query = string_field(value, "query");
    page.site = string_field(value, "site");
    page.filter = string_field(value, "filter");
    page.subject = string_field(value, "subject");
    page.check_period = value
        .get("check_period")
        .and_then(Value::as_i64)
        .unwrap_or(3600)
        .max(60);
    page.auto_watch = value
        .get("auto_watch")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(page)
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn kind_name(kind: DownloaderKind) -> &'static str {
    match kind {
        DownloaderKind::Urls => "urls",
        DownloaderKind::Gallery => "gallery",
        DownloaderKind::Watcher => "watcher",
        DownloaderKind::Simple => "simple",
    }
}

#[cfg(test)]
mod tests;
