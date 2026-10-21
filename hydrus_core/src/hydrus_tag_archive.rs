//! Rust port of `hydrus/core/HydrusTagArchive.py`.
//!
//! These are standalone SQLite archives that hydrus can import. As in Python,
//! they are *not* thread-safe: the caller owns the archive and its connection.
//!
//! Python returns sets and generators; the port returns `BTreeSet`s and eager
//! `Vec`s so callers do not have to hold a borrow of the archive while
//! iterating.

use std::collections::BTreeSet;
use std::fmt::Display;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// Python's `HASH_TYPE_MD5` .. `HASH_TYPE_SHA512`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum HashType {
    Md5 = 0,
    Sha1 = 1,
    Sha256 = 2,
    Sha512 = 3,
}

pub const HASH_TYPE_MD5: HashType = HashType::Md5;
pub const HASH_TYPE_SHA1: HashType = HashType::Sha1;
pub const HASH_TYPE_SHA256: HashType = HashType::Sha256;
pub const HASH_TYPE_SHA512: HashType = HashType::Sha512;

impl HashType {
    /// The byte length of a hash of this type.
    pub fn hash_size(self) -> usize {
        match self {
            Self::Md5 => 16,
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }

    /// `hash_type_to_str_lookup[ hash_type ]`
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Md5 => "md5",
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    /// `hash_str_to_type_lookup[ hash_str ]`
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "md5" => Some(Self::Md5),
            "sha1" => Some(Self::Sha1),
            "sha256" => Some(Self::Sha256),
            "sha512" => Some(Self::Sha512),
            _ => None,
        }
    }

    /// The Python `len_to_hash_type` lookup.
    pub fn from_hash_size(size: usize) -> Option<Self> {
        match size {
            16 => Some(Self::Md5),
            20 => Some(Self::Sha1),
            32 => Some(Self::Sha256),
            64 => Some(Self::Sha512),
            _ => None,
        }
    }

    pub fn try_from_int(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Md5),
            1 => Some(Self::Sha1),
            2 => Some(Self::Sha256),
            3 => Some(Self::Sha512),
            _ => None,
        }
    }
}

fn db_error(context: impl Display, error: rusqlite::Error) -> HydrusException {
    HydrusException::new(ErrorKind::Db, format!("{context}: {error}"))
}

fn archive_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Db, message)
}

fn unique_suffix() -> String {
    use rand::RngCore;

    let mut buffer = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buffer);

    buffer.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `read_large_id_query_in_separate_chunks( cursor, select_statement, chunk_size )`
///
/// Runs a `SELECT` that must produce one integer id per row and returns those
/// ids in chunks, using a temporary table exactly like Python.
pub fn read_large_id_query_in_separate_chunks(
    connection: &Connection,
    select_statement: &str,
    chunk_size: usize,
) -> HydrusResult<Vec<Vec<i64>>> {
    if chunk_size == 0 {
        return Err(archive_error("Chunk size must be greater than zero!"));
    }

    let table_name = format!("tempbigread{}", unique_suffix());

    connection
        .execute_batch(&format!(
            "CREATE TEMPORARY TABLE {table_name} ( job_id INTEGER PRIMARY KEY AUTOINCREMENT, temp_id INTEGER );"
        ))
        .map_err(|error| db_error("Could not create the temporary id table", error))?;

    let insert_result = connection.execute_batch(&format!(
        "INSERT INTO {table_name} ( temp_id ) {select_statement}"
    ));

    if let Err(error) = insert_result {
        let _ = connection.execute_batch(&format!("DROP TABLE {table_name};"));

        return Err(db_error("Could not populate the temporary id table", error));
    }

    let num_to_do = connection.changes() as i64;
    let chunk_size = chunk_size as i64;

    let mut chunks = Vec::new();
    let mut i = 0i64;

    while i < num_to_do {
        let mut statement = connection
            .prepare(&format!(
                "SELECT temp_id FROM {table_name} WHERE job_id BETWEEN ? AND ?;"
            ))
            .map_err(|error| db_error("Could not read the temporary id table", error))?;

        let rows = statement
            .query_map(params![i, i + chunk_size - 1], |row| row.get::<_, i64>(0))
            .map_err(|error| db_error("Could not read the temporary id table", error))?;

        let mut chunk = Vec::new();

        for row in rows {
            chunk.push(row.map_err(|error| db_error("Could not read an id", error))?);
        }

        chunks.push(chunk);

        i += chunk_size;
    }

    connection
        .execute_batch(&format!("DROP TABLE {table_name};"))
        .map_err(|error| db_error("Could not drop the temporary id table", error))?;

    Ok(chunks)
}

/// Python's `HydrusTagArchive`.
pub struct HydrusTagArchive {
    path: PathBuf,
    connection: Connection,
    namespaces: BTreeSet<String>,
}

impl HydrusTagArchive {
    /// `HydrusTagArchive( path )`
    pub fn new(path: impl AsRef<Path>) -> HydrusResult<Self> {
        let path = path.as_ref().to_path_buf();
        let is_new = !path.exists();

        let connection = Connection::open(&path)
            .map_err(|error| db_error(format!("Could not open \"{}\"", path.display()), error))?;

        let mut archive = Self {
            path,
            connection,
            namespaces: BTreeSet::new(),
        };

        if is_new {
            archive.init_db()?;
        }

        archive.reload_namespaces()?;

        Ok(archive)
    }

    fn init_db(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch(
                "
                CREATE TABLE hash_type ( hash_type INTEGER );

                CREATE TABLE hashes ( hash_id INTEGER PRIMARY KEY, hash BLOB );
                CREATE UNIQUE INDEX hashes_hash_index ON hashes ( hash );

                CREATE TABLE mappings ( hash_id INTEGER, tag_id INTEGER, PRIMARY KEY ( hash_id, tag_id ) );
                CREATE INDEX mappings_hash_id_index ON mappings ( hash_id );

                CREATE TABLE namespaces ( namespace TEXT );

                CREATE TABLE tags ( tag_id INTEGER PRIMARY KEY, tag TEXT );
                CREATE UNIQUE INDEX tags_tag_index ON tags ( tag );
                ",
            )
            .map_err(|error| db_error("Could not initialise the tag archive", error))
    }

    fn reload_namespaces(&mut self) -> HydrusResult<()> {
        let mut statement = self
            .connection
            .prepare("SELECT namespace FROM namespaces;")
            .map_err(|error| db_error("Could not read namespaces", error))?;

        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| db_error("Could not read namespaces", error))?;

        self.namespaces.clear();

        for row in rows {
            self.namespaces
                .insert(row.map_err(|error| db_error("Could not read a namespace", error))?);
        }

        self.namespaces.insert(String::new());

        Ok(())
    }

    fn add_mappings_for_hash(&self, hash_id: i64, tag_ids: &[i64]) -> HydrusResult<()> {
        let mut statement = self
            .connection
            .prepare("INSERT OR IGNORE INTO mappings ( hash_id, tag_id ) VALUES ( ?, ? );")
            .map_err(|error| db_error("Could not prepare the mapping insert", error))?;

        for tag_id in tag_ids {
            statement
                .execute(params![hash_id, tag_id])
                .map_err(|error| db_error("Could not add a mapping", error))?;
        }

        Ok(())
    }

    fn get_hashes_for_tag(&self, tag_id: i64) -> HydrusResult<BTreeSet<Vec<u8>>> {
        let mut statement = self
            .connection
            .prepare("SELECT hash FROM mappings NATURAL JOIN hashes WHERE tag_id = ?;")
            .map_err(|error| db_error("Could not prepare the hash query", error))?;

        let rows = statement
            .query_map(params![tag_id], |row| row.get::<_, Vec<u8>>(0))
            .map_err(|error| db_error("Could not read hashes", error))?;

        let mut hashes = BTreeSet::new();

        for row in rows {
            hashes.insert(row.map_err(|error| db_error("Could not read a hash", error))?);
        }

        Ok(hashes)
    }

    fn get_hash_id(&self, hash: &[u8], read_only: bool) -> HydrusResult<i64> {
        let existing = self
            .connection
            .query_row(
                "SELECT hash_id FROM hashes WHERE hash = ?;",
                params![hash],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| db_error("Could not look up a hash", error))?;

        if let Some(hash_id) = existing {
            return Ok(hash_id);
        }

        if read_only {
            return Err(archive_error("The hash is not in this archive."));
        }

        self.connection
            .execute("INSERT INTO hashes ( hash ) VALUES ( ? );", params![hash])
            .map_err(|error| db_error("Could not add a hash", error))?;

        Ok(self.connection.last_insert_rowid())
    }

    fn get_tags_for_hash(&self, hash_id: i64) -> HydrusResult<BTreeSet<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT tag FROM mappings NATURAL JOIN tags WHERE hash_id = ?;")
            .map_err(|error| db_error("Could not prepare the tag query", error))?;

        let rows = statement
            .query_map(params![hash_id], |row| row.get::<_, String>(0))
            .map_err(|error| db_error("Could not read tags", error))?;

        let mut tags = BTreeSet::new();

        for row in rows {
            tags.insert(row.map_err(|error| db_error("Could not read a tag", error))?);
        }

        Ok(tags)
    }

    fn get_tag_id(&mut self, tag: &str, read_only: bool) -> HydrusResult<i64> {
        let existing = self
            .connection
            .query_row("SELECT tag_id FROM tags WHERE tag = ?;", params![tag], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not look up a tag", error))?;

        let tag_id = if let Some(tag_id) = existing {
            tag_id
        } else {
            if read_only {
                return Err(archive_error("The tag is not in this archive."));
            }

            self.connection
                .execute("INSERT INTO tags ( tag ) VALUES ( ? );", params![tag])
                .map_err(|error| db_error("Could not add a tag", error))?;

            self.connection.last_insert_rowid()
        };

        if let Some((namespace, _subtag)) = tag.split_once(':') {
            if !namespace.is_empty() && !self.namespaces.contains(namespace) {
                self.connection
                    .execute(
                        "INSERT INTO namespaces ( namespace ) VALUES ( ? );",
                        params![namespace],
                    )
                    .map_err(|error| db_error("Could not add a namespace", error))?;

                self.namespaces.insert(namespace.to_string());
            }
        }

        Ok(tag_id)
    }

    /// `begin_big_job`
    pub fn begin_big_job(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("BEGIN IMMEDIATE;")
            .map_err(|error| db_error("Could not begin a big job", error))
    }

    /// `commit_big_job`
    pub fn commit_big_job(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("COMMIT;")
            .map_err(|error| db_error("Could not commit a big job", error))
    }

    /// `add_mapping( hash, tag )`
    pub fn add_mapping(&mut self, hash: &[u8], tag: &str) -> HydrusResult<()> {
        let hash_id = self.get_hash_id(hash, false)?;
        let tag_id = self.get_tag_id(tag, false)?;

        self.connection
            .execute(
                "INSERT OR IGNORE INTO mappings ( hash_id, tag_id ) VALUES ( ?, ? );",
                params![hash_id, tag_id],
            )
            .map_err(|error| db_error("Could not add a mapping", error))?;

        Ok(())
    }

    /// `add_mappings( hash, tags )`
    pub fn add_mappings<'a, I>(&mut self, hash: &[u8], tags: I) -> HydrusResult<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let hash_id = self.get_hash_id(hash, false)?;
        let mut tag_ids = Vec::new();

        for tag in tags {
            tag_ids.push(self.get_tag_id(tag, false)?);
        }

        self.add_mappings_for_hash(hash_id, &tag_ids)
    }

    /// `close`. Rust closes the connection on drop; this consumes the archive
    /// so the Python `del hta` shape still reads naturally.
    pub fn close(self) {}

    /// `delete_mapping( hash, tag )`
    pub fn delete_mapping(&mut self, hash: &[u8], tag: &str) -> HydrusResult<()> {
        let hash_id = self.get_hash_id(hash, false)?;
        let tag_id = self.get_tag_id(tag, false)?;

        self.connection
            .execute(
                "DELETE FROM mappings WHERE hash_id = ? AND tag_id = ?;",
                params![hash_id, tag_id],
            )
            .map_err(|error| db_error("Could not delete a mapping", error))?;

        Ok(())
    }

    /// `delete_mappings( hash )`
    pub fn delete_mappings(&mut self, hash: &[u8]) -> HydrusResult<()> {
        self.delete_tags(hash)
    }

    /// `delete_tags( hash )`
    pub fn delete_tags(&mut self, hash: &[u8]) -> HydrusResult<()> {
        let Ok(hash_id) = self.get_hash_id(hash, true) else {
            return Ok(());
        };

        self.connection
            .execute("DELETE FROM mappings WHERE hash_id = ?;", params![hash_id])
            .map_err(|error| db_error("Could not delete tags", error))?;

        Ok(())
    }

    /// `delete_namespaces`
    pub fn delete_namespaces(&mut self) -> HydrusResult<()> {
        self.namespaces.clear();
        self.namespaces.insert(String::new());

        self.connection
            .execute("DELETE FROM namespaces;", [])
            .map_err(|error| db_error("Could not delete namespaces", error))?;

        Ok(())
    }

    /// `get_hashes( tag )`
    pub fn get_hashes(&mut self, tag: &str) -> HydrusResult<BTreeSet<Vec<u8>>> {
        let Ok(tag_id) = self.get_tag_id(tag, true) else {
            return Ok(BTreeSet::new());
        };

        self.get_hashes_for_tag(tag_id)
    }

    /// `get_hash_type`
    pub fn get_hash_type(&self) -> HydrusResult<HashType> {
        let existing = self
            .connection
            .query_row("SELECT hash_type FROM hash_type;", [], |row| {
                row.get::<_, u8>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not read the hash type", error))?;

        if let Some(hash_type) = existing {
            return HashType::try_from_int(hash_type)
                .ok_or_else(|| archive_error("This archive has an unknown hash type!"));
        }

        let hash = self
            .connection
            .query_row("SELECT hash FROM hashes;", [], |row| row.get::<_, Vec<u8>>(0))
            .optional()
            .map_err(|error| db_error("Could not read a hash", error))?;

        let Some(hash) = hash else {
            return Err(archive_error(
                "This archive has no hash type set, and as it has no files, no hash type guess can be made.",
            ));
        };

        let hash_type = HashType::from_hash_size(hash.len())
            .ok_or_else(|| archive_error("This archive has non-standard hashes. Something is wrong."))?;

        self.set_hash_type(hash_type)?;

        Ok(hash_type)
    }

    /// `get_mappings( hash )`
    pub fn get_mappings(&self, hash: &[u8]) -> HydrusResult<BTreeSet<String>> {
        self.get_tags(hash)
    }

    /// `get_name`
    pub fn get_name(&self) -> String {
        let filename = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        filename
            .split_once('.')
            .map(|(name, _)| name.to_string())
            .unwrap_or(filename)
    }

    /// `get_namespaces`
    pub fn get_namespaces(&self) -> &BTreeSet<String> {
        &self.namespaces
    }

    /// `get_tags( hash )`
    pub fn get_tags(&self, hash: &[u8]) -> HydrusResult<BTreeSet<String>> {
        let Ok(hash_id) = self.get_hash_id(hash, true) else {
            return Ok(BTreeSet::new());
        };

        self.get_tags_for_hash(hash_id)
    }

    /// `has_hash( hash )`
    pub fn has_hash(&self, hash: &[u8]) -> bool {
        self.get_hash_id(hash, true).is_ok()
    }

    /// `has_hash_type_set`
    pub fn has_hash_type_set(&self) -> HydrusResult<bool> {
        self.connection
            .query_row("SELECT 1 FROM hash_type;", [], |row| row.get::<_, i64>(0))
            .optional()
            .map(|result| result.is_some())
            .map_err(|error| db_error("Could not read the hash type", error))
    }

    /// `iterate_hashes`. Python yields lazily; the port collects eagerly.
    pub fn iterate_hashes(&self) -> HydrusResult<Vec<Vec<u8>>> {
        let mut statement = self
            .connection
            .prepare("SELECT hash FROM hashes;")
            .map_err(|error| db_error("Could not prepare the hash query", error))?;

        let rows = statement
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .map_err(|error| db_error("Could not read hashes", error))?;

        let mut hashes = Vec::new();

        for row in rows {
            hashes.push(row.map_err(|error| db_error("Could not read a hash", error))?);
        }

        Ok(hashes)
    }

    /// `iterate_mappings`
    pub fn iterate_mappings(&self) -> HydrusResult<Vec<(Vec<u8>, BTreeSet<String>)>> {
        let mut mappings = Vec::new();

        for group_of_hash_ids in read_large_id_query_in_separate_chunks(
            &self.connection,
            "SELECT hash_id FROM hashes;",
            256,
        )? {
            for hash_id in group_of_hash_ids {
                let tags = self.get_tags_for_hash(hash_id)?;

                if tags.is_empty() {
                    continue;
                }

                let hash = self
                    .connection
                    .query_row(
                        "SELECT hash FROM hashes WHERE hash_id = ?;",
                        params![hash_id],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .map_err(|error| db_error("Could not read a hash", error))?;

                mappings.push((hash, tags));
            }
        }

        Ok(mappings)
    }

    /// `iterate_mappings_tag_first`
    pub fn iterate_mappings_tag_first(&self) -> HydrusResult<Vec<(String, BTreeSet<Vec<u8>>)>> {
        let mut mappings = Vec::new();

        for group_of_tag_ids in read_large_id_query_in_separate_chunks(
            &self.connection,
            "SELECT tag_id FROM tags;",
            256,
        )? {
            for tag_id in group_of_tag_ids {
                let hashes = self.get_hashes_for_tag(tag_id)?;

                if hashes.is_empty() {
                    continue;
                }

                let tag = self
                    .connection
                    .query_row("SELECT tag FROM tags WHERE tag_id = ?;", params![tag_id], |row| {
                        row.get::<_, String>(0)
                    })
                    .map_err(|error| db_error("Could not read a tag", error))?;

                mappings.push((tag, hashes));
            }
        }

        Ok(mappings)
    }

    /// `optimise`
    pub fn optimise(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("VACUUM; ANALYZE;")
            .map_err(|error| db_error("Could not optimise the tag archive", error))
    }

    /// `rebuild_namespaces( namespaces_to_exclude = None )`
    pub fn rebuild_namespaces(
        &mut self,
        namespaces_to_exclude: &BTreeSet<String>,
    ) -> HydrusResult<()> {
        self.namespaces.clear();
        self.namespaces.insert(String::new());

        self.connection
            .execute("DELETE FROM namespaces;", [])
            .map_err(|error| db_error("Could not delete namespaces", error))?;

        let mut statement = self
            .connection
            .prepare("SELECT tag FROM tags;")
            .map_err(|error| db_error("Could not read tags", error))?;

        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| db_error("Could not read tags", error))?;

        for row in rows {
            let tag = row.map_err(|error| db_error("Could not read a tag", error))?;

            if let Some((namespace, _subtag)) = tag.split_once(':') {
                if !namespace.is_empty()
                    && !self.namespaces.contains(namespace)
                    && !namespaces_to_exclude.contains(namespace)
                {
                    self.namespaces.insert(namespace.to_string());
                }
            }
        }

        for namespace in &self.namespaces {
            self.connection
                .execute(
                    "INSERT INTO namespaces ( namespace ) VALUES ( ? );",
                    params![namespace],
                )
                .map_err(|error| db_error("Could not add a namespace", error))?;
        }

        Ok(())
    }

    /// `set_hash_type`
    pub fn set_hash_type(&self, hash_type: HashType) -> HydrusResult<()> {
        self.connection
            .execute("DELETE FROM hash_type;", [])
            .map_err(|error| db_error("Could not clear the hash type", error))?;

        self.connection
            .execute(
                "INSERT INTO hash_type ( hash_type ) VALUES ( ? );",
                params![hash_type as u8],
            )
            .map_err(|error| db_error("Could not set the hash type", error))?;

        Ok(())
    }

    /// `set_mappings( hash, tags )`
    pub fn set_mappings<'a, I>(&mut self, hash: &[u8], tags: I) -> HydrusResult<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let hash_id = self.get_hash_id(hash, false)?;

        self.connection
            .execute("DELETE FROM mappings WHERE hash_id = ?;", params![hash_id])
            .map_err(|error| db_error("Could not clear mappings", error))?;

        let mut tag_ids = Vec::new();

        for tag in tags {
            tag_ids.push(self.get_tag_id(tag, false)?);
        }

        self.add_mappings_for_hash(hash_id, &tag_ids)
    }
}

/// Python's `TAG_PAIR_TYPE_SIBLINGS` / `TAG_PAIR_TYPE_PARENTS`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TagPairType {
    Siblings = 0,
    Parents = 1,
}

impl TagPairType {
    pub fn try_from_int(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Siblings),
            1 => Some(Self::Parents),
            _ => None,
        }
    }
}

pub const TAG_PAIR_TYPE_SIBLINGS: TagPairType = TagPairType::Siblings;
pub const TAG_PAIR_TYPE_PARENTS: TagPairType = TagPairType::Parents;

/// Python's `HydrusTagPairArchive`.
pub struct HydrusTagPairArchive {
    path: PathBuf,
    connection: Connection,
    pair_type: Option<TagPairType>,
}

impl HydrusTagPairArchive {
    /// `HydrusTagPairArchive( path )`
    pub fn new(path: impl AsRef<Path>) -> HydrusResult<Self> {
        let path = path.as_ref().to_path_buf();
        let is_new = !path.exists();

        let connection = Connection::open(&path).map_err(|error| {
            db_error(
                format!("Could not open \"{}\"", path.display()),
                error,
            )
        })?;

        if is_new {
            connection
                .execute_batch(
                    "
                    CREATE TABLE pair_type ( pair_type INTEGER );

                    CREATE TABLE pairs ( tag_id_1 INTEGER, tag_id_2 INTEGER, PRIMARY KEY ( tag_id_1, tag_id_2 ) );

                    CREATE TABLE tags ( tag_id INTEGER PRIMARY KEY, tag TEXT );
                    CREATE UNIQUE INDEX tags_tag_index ON tags ( tag );
                    ",
                )
                .map_err(|error| db_error("Could not initialise the tag pair archive", error))?;
        }

        let pair_type = connection
            .query_row("SELECT pair_type FROM pair_type;", [], |row| {
                row.get::<_, u8>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not read the pair type", error))?
            .and_then(TagPairType::try_from_int);

        Ok(Self {
            path,
            connection,
            pair_type,
        })
    }

    fn get_tag_id(&self, tag: &str) -> HydrusResult<i64> {
        let existing = self
            .connection
            .query_row("SELECT tag_id FROM tags WHERE tag = ?;", params![tag], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not look up a tag", error))?;

        if let Some(tag_id) = existing {
            return Ok(tag_id);
        }

        self.connection
            .execute("INSERT INTO tags ( tag ) VALUES ( ? );", params![tag])
            .map_err(|error| db_error("Could not add a tag", error))?;

        Ok(self.connection.last_insert_rowid())
    }

    /// `begin_big_job`
    pub fn begin_big_job(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("BEGIN IMMEDIATE;")
            .map_err(|error| db_error("Could not begin a big job", error))
    }

    /// `commit_big_job`
    pub fn commit_big_job(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("COMMIT;")
            .map_err(|error| db_error("Could not commit a big job", error))
    }

    /// `close`
    pub fn close(self) {}

    /// `add_pair( tag_1, tag_2 )`
    pub fn add_pair(&mut self, tag_1: &str, tag_2: &str) -> HydrusResult<()> {
        self.add_pairs(std::iter::once((tag_1, tag_2)))
    }

    /// `add_pairs( pairs )`
    pub fn add_pairs<'a, I>(&mut self, pairs: I) -> HydrusResult<()>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        if self.pair_type.is_none() {
            return Err(archive_error(
                "Please set the pair type first before you start populating the database!",
            ));
        }

        let mut statement = self
            .connection
            .prepare("INSERT OR IGNORE INTO pairs ( tag_id_1, tag_id_2 ) VALUES ( ?, ? );")
            .map_err(|error| db_error("Could not prepare the pair insert", error))?;

        for (tag_1, tag_2) in pairs {
            let tag_id_1 = self.get_tag_id(tag_1)?;
            let tag_id_2 = self.get_tag_id(tag_2)?;

            statement
                .execute(params![tag_id_1, tag_id_2])
                .map_err(|error| db_error("Could not add a pair", error))?;
        }

        Ok(())
    }

    /// `delete_pair( tag_1, tag_2 )`
    pub fn delete_pair(&mut self, tag_1: &str, tag_2: &str) -> HydrusResult<()> {
        self.delete_pairs(std::iter::once((tag_1, tag_2)))
    }

    /// `delete_pairs( pairs )`
    pub fn delete_pairs<'a, I>(&mut self, pairs: I) -> HydrusResult<()>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        for (tag_1, tag_2) in pairs {
            if !self.has_pair(tag_1, tag_2)? {
                continue;
            }

            let tag_id_1 = self.get_tag_id(tag_1)?;
            let tag_id_2 = self.get_tag_id(tag_2)?;

            self.connection
                .execute(
                    "DELETE FROM pairs WHERE tag_id_1 = ? AND tag_id_2 = ?;",
                    params![tag_id_1, tag_id_2],
                )
                .map_err(|error| db_error("Could not delete a pair", error))?;
        }

        Ok(())
    }

    /// `get_pair_type`
    pub fn get_pair_type(&self) -> Option<TagPairType> {
        self.pair_type
    }

    /// `get_name`
    pub fn get_name(&self) -> String {
        let filename = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        filename
            .split_once('.')
            .map(|(name, _)| name.to_string())
            .unwrap_or(filename)
    }

    /// `has_pair( tag_1, tag_2 )`
    pub fn has_pair(&mut self, tag_1: &str, tag_2: &str) -> HydrusResult<bool> {
        let tag_id_1 = self.get_tag_id(tag_1)?;
        let tag_id_2 = self.get_tag_id(tag_2)?;

        self.connection
            .query_row(
                "SELECT 1 FROM pairs WHERE tag_id_1 = ? AND tag_id_2 = ?;",
                params![tag_id_1, tag_id_2],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|result| result.is_some())
            .map_err(|error| db_error("Could not look up a pair", error))
    }

    /// `has_pair_type_set`
    pub fn has_pair_type_set(&self) -> bool {
        self.pair_type.is_some()
    }

    /// `iterate_pairs`
    pub fn iterate_pairs(&self) -> HydrusResult<Vec<(String, String)>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT t1.tag, t2.tag FROM pairs, tags AS t1, tags AS t2 ON ( pairs.tag_id_1 = t1.tag_id AND pairs.tag_id_2 = t2.tag_id );",
            )
            .map_err(|error| db_error("Could not prepare the pair query", error))?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| db_error("Could not read pairs", error))?;

        let mut pairs = Vec::new();

        for row in rows {
            pairs.push(row.map_err(|error| db_error("Could not read a pair", error))?);
        }

        Ok(pairs)
    }

    /// `optimise`
    pub fn optimise(&self) -> HydrusResult<()> {
        self.connection
            .execute_batch("VACUUM; ANALYZE;")
            .map_err(|error| db_error("Could not optimise the tag pair archive", error))
    }

    /// `set_pair_type`
    pub fn set_pair_type(&mut self, pair_type: TagPairType) -> HydrusResult<()> {
        self.connection
            .execute("DELETE FROM pair_type;", [])
            .map_err(|error| db_error("Could not clear the pair type", error))?;

        self.connection
            .execute(
                "INSERT INTO pair_type ( pair_type ) VALUES ( ? );",
                params![pair_type as u8],
            )
            .map_err(|error| db_error("Could not set the pair type", error))?;

        self.pair_type = Some(pair_type);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        let dir = crate::hydrus_temp::get_sub_temp_dir(&format!("tag_archive_{name}"))
            .expect("temp dir");

        dir.join("archive.db")
    }

    #[test]
    fn hash_types_round_trip() {
        for hash_type in [
            HashType::Md5,
            HashType::Sha1,
            HashType::Sha256,
            HashType::Sha512,
        ] {
            assert_eq!(HashType::from_str(hash_type.as_str()), Some(hash_type));
            assert_eq!(
                HashType::from_hash_size(hash_type.hash_size()),
                Some(hash_type)
            );
            assert_eq!(HashType::try_from_int(hash_type as u8), Some(hash_type));
        }

        assert_eq!(HashType::from_str("nope"), None);
        assert_eq!(HashType::from_hash_size(17), None);
        assert_eq!(HashType::try_from_int(4), None);
    }

    #[test]
    fn tag_archive_round_trip() {
        let path = temp_db("round_trip");
        let hash = vec![1u8; 32];

        {
            let mut archive = HydrusTagArchive::new(&path).expect("archive");

            archive.set_hash_type(HashType::Sha256).expect("hash type");

            archive.add_mapping(&hash, "artist:alice").expect("mapping");
            archive
                .add_mappings(&hash, ["series:one", "blue eyes"])
                .expect("mappings");

            assert!(archive.has_hash(&hash));
            assert!(!archive.has_hash(&[2u8; 32]));

            assert_eq!(
                archive.get_tags(&hash).expect("tags"),
                BTreeSet::from([
                    "artist:alice".to_string(),
                    "series:one".to_string(),
                    "blue eyes".to_string()
                ])
            );

            assert_eq!(
                archive.get_hashes("artist:alice").expect("hashes"),
                BTreeSet::from([hash.clone()])
            );

            assert_eq!(
                archive.get_namespaces(),
                &BTreeSet::from(["".to_string(), "artist".to_string(), "series".to_string()])
            );

            assert_eq!(archive.get_name(), "archive");
            assert_eq!(archive.get_hash_type().expect("read hash type"), HashType::Sha256);

            archive.delete_mapping(&hash, "blue eyes").expect("delete");

            assert_eq!(
                archive.get_tags(&hash).expect("tags").len(),
                2
            );

            archive.delete_tags(&hash).expect("delete tags");

            assert!(archive.get_tags(&hash).expect("tags").is_empty());
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn tag_archive_iteration_and_namespaces() {
        let path = temp_db("iteration");
        let hash_1 = vec![1u8; 32];
        let hash_2 = vec![2u8; 32];

        {
            let mut archive = HydrusTagArchive::new(&path).expect("archive");

            archive.set_mappings(&hash_1, ["a:one", "b:two"]).expect("set");
            archive.add_mapping(&hash_2, "a:three").expect("add");

            let mut hashes = archive.iterate_hashes().expect("hashes");
            hashes.sort();

            assert_eq!(hashes, vec![hash_1.clone(), hash_2.clone()]);

            let mappings = archive.iterate_mappings().expect("mappings");

            assert_eq!(mappings.len(), 2);
            assert_eq!(
                mappings[0].1,
                BTreeSet::from(["a:one".to_string(), "b:two".to_string()])
            );

            let tag_first = archive.iterate_mappings_tag_first().expect("tag first");

            assert_eq!(tag_first.len(), 3);

            archive
                .rebuild_namespaces(&BTreeSet::from(["b".to_string()]))
                .expect("rebuild");

            assert_eq!(
                archive.get_namespaces(),
                &BTreeSet::from(["".to_string(), "a".to_string()])
            );

            archive.delete_namespaces().expect("delete namespaces");

            assert_eq!(
                archive.get_namespaces(),
                &BTreeSet::from([String::new()])
            );

            archive.optimise().expect("optimise");
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn tag_archive_infers_hash_type() {
        let path = temp_db("infer");
        let hash = vec![1u8; 16];

        {
            let mut archive = HydrusTagArchive::new(&path).expect("archive");

            archive.add_mapping(&hash, "tag").expect("mapping");

            assert!(!archive.has_hash_type_set().expect("has hash type set"));
            assert_eq!(archive.get_hash_type().expect("hash type"), HashType::Md5);
            assert!(archive.has_hash_type_set().expect("has hash type set"));
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn tag_archive_without_hashes_cannot_guess() {
        let path = temp_db("empty");

        {
            let archive = HydrusTagArchive::new(&path).expect("archive");

            assert!(archive.get_hash_type().is_err());
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn tag_pair_archive_round_trip() {
        let path = temp_db("pairs");

        {
            let mut archive = HydrusTagPairArchive::new(&path).expect("archive");

            assert!(!archive.has_pair_type_set());
            assert!(archive.add_pair("a", "b").is_err());

            archive.set_pair_type(TagPairType::Siblings).expect("pair type");

            assert_eq!(archive.get_pair_type(), Some(TagPairType::Siblings));
            assert!(archive.has_pair_type_set());

            archive.add_pair("a", "b").expect("pair");
            archive.add_pair("b", "c").expect("pair");

            assert!(archive.has_pair("a", "b").expect("has pair"));
            assert!(!archive.has_pair("b", "a").expect("has pair"));

            let mut pairs = archive.iterate_pairs().expect("pairs");
            pairs.sort();

            assert_eq!(
                pairs,
                vec![
                    ("a".to_string(), "b".to_string()),
                    ("b".to_string(), "c".to_string())
                ]
            );

            archive.delete_pair("a", "b").expect("delete");

            assert!(!archive.has_pair("a", "b").expect("has pair"));

            archive.optimise().expect("optimise");
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn large_id_queries_are_chunked() {
        let path = temp_db("chunks");

        {
            let archive = HydrusTagArchive::new(&path).expect("archive");

            for value in 0u8..10 {
                archive
                    .connection
                    .execute(
                        "INSERT INTO hashes ( hash ) VALUES ( ? );",
                        params![vec![value; 32]],
                    )
                    .expect("insert hash");
            }

            let chunks = read_large_id_query_in_separate_chunks(
                &archive.connection,
                "SELECT hash_id FROM hashes;",
                4,
            )
            .expect("chunks");

            // Python's job_id is AUTOINCREMENT, so the first chunk covers ids
            // 1..=chunk_size while the loop starts at 0. That leaves a short
            // first chunk; the port preserves the behaviour.
            assert_eq!(chunks.len(), 3);
            assert_eq!(chunks[0], vec![1, 2, 3]);
            assert_eq!(chunks[1], vec![4, 5, 6, 7]);
            assert_eq!(chunks[2], vec![8, 9, 10]);
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }
}
