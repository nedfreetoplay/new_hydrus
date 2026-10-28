//! Basic file metadata from `hydrus/client/db/ClientDBFilesMetadataBasic.py`.

use hydrus_core::files::hydrus_file_handling::FileInfo;
use hydrus_core::hydrus_constants::Mime;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_exceptions::HydrusResult;

pub fn set_file_info(db: &DBBase, hash_id: i64, file_info: &FileInfo) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO files_info
         ( hash_id, size, mime, width, height, duration, num_frames, has_audio, num_words )
         VALUES ( ?, ?, ?, ?, ?, ?, ?, ?, ? )",
        &[
            &hash_id,
            &(file_info.size as i64),
            &file_info.mime.as_int(),
            &file_info.width.map(i64::from),
            &file_info.height.map(i64::from),
            &file_info.duration_ms,
            &file_info.num_frames.map(i64::from),
            &file_info.has_audio,
            &file_info.num_words,
        ],
    )?;

    Ok(())
}

pub fn get_file_info(db: &DBBase, hash_id: i64) -> HydrusResult<Option<FileInfo>> {
    db.query_one(
        "SELECT size, mime, width, height, duration, num_frames, has_audio, num_words
         FROM files_info WHERE hash_id = ?",
        &[&hash_id],
        |row| {
            let mime_int: u16 = row.get(1)?;
            let mime = Mime::try_from_int(mime_int).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Integer,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "unknown mime",
                    )),
                )
            })?;

            Ok(FileInfo {
                size: row.get::<_, i64>(0)? as u64,
                mime,
                width: row.get::<_, Option<i64>>(2)?.map(|value| value as u32),
                height: row.get::<_, Option<i64>>(3)?.map(|value| value as u32),
                duration_ms: row.get(4)?,
                num_frames: row.get::<_, Option<i64>>(5)?.map(|value| value as u32),
                has_audio: row.get::<_, Option<bool>>(6)?.unwrap_or(false),
                num_words: row.get(7)?,
            })
        },
    )
}

pub fn has_file_info(db: &DBBase, hash_id: i64) -> HydrusResult<bool> {
    let count: i64 = db
        .query_one(
            "SELECT COUNT(*) FROM files_info WHERE hash_id = ?",
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataFlag {
    IccProfile,
    Exif,
    Xmp,
    Iptc,
    HumanReadableEmbeddedMetadata,
    SoftwareSource,
    Transparency,
}

impl MetadataFlag {
    pub fn table(self) -> &'static str {
        match self {
            Self::IccProfile => "has_icc_profile",
            Self::Exif => "has_exif",
            Self::Xmp => "has_xmp",
            Self::Iptc => "has_iptc",
            Self::HumanReadableEmbeddedMetadata => "has_human_readable_embedded_metadata",
            Self::SoftwareSource => "has_software_source",
            Self::Transparency => "has_transparency",
        }
    }
}

pub fn set_metadata_flag(
    db: &DBBase,
    hash_id: i64,
    flag: MetadataFlag,
    value: bool,
) -> HydrusResult<()> {
    let table = flag.table();

    if value {
        db.execute(
            &format!("INSERT OR IGNORE INTO {table} ( hash_id ) VALUES ( ? )"),
            &[&hash_id],
        )?;
    } else {
        db.execute(
            &format!("DELETE FROM {table} WHERE hash_id = ?"),
            &[&hash_id],
        )?;
    }

    Ok(())
}

pub fn has_metadata_flag(db: &DBBase, hash_id: i64, flag: MetadataFlag) -> HydrusResult<bool> {
    let table = flag.table();
    let count: i64 = db
        .query_one(
            &format!("SELECT COUNT(*) FROM {table} WHERE hash_id = ?"),
            &[&hash_id],
            |row| row.get(0),
        )?
        .unwrap_or(0);

    Ok(count > 0)
}

pub fn get_hash_ids_with_flag(db: &DBBase, flag: MetadataFlag) -> HydrusResult<Vec<i64>> {
    db.query_all(
        &format!("SELECT hash_id FROM {}", flag.table()),
        &[],
        |row| row.get(0),
    )
}

pub fn set_forced_mime(db: &DBBase, hash_id: i64, mime: Mime) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO files_info_forced_filetypes ( hash_id, forced_mime )
         VALUES ( ?, ? )",
        &[&hash_id, &mime.as_int()],
    )?;

    Ok(())
}

pub fn get_forced_mime(db: &DBBase, hash_id: i64) -> HydrusResult<Option<Mime>> {
    let mime: Option<u16> = db.query_one(
        "SELECT forced_mime FROM files_info_forced_filetypes WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )?;

    Ok(mime.and_then(Mime::try_from_int))
}

pub fn set_blurhash(db: &DBBase, hash_id: i64, blurhash: &str) -> HydrusResult<()> {
    db.execute(
        "INSERT OR REPLACE INTO blurhashes ( hash_id, blurhash ) VALUES ( ?, ? )",
        &[&hash_id, &blurhash],
    )?;

    Ok(())
}

pub fn get_blurhash(db: &DBBase, hash_id: i64) -> HydrusResult<Option<String>> {
    db.query_one(
        "SELECT blurhash FROM blurhashes WHERE hash_id = ?",
        &[&hash_id],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests;
