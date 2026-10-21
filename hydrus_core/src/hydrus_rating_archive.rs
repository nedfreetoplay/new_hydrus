//! Rust port of `hydrus/core/HydrusRatingArchive.py`.
//!
//! A rating is a float in `0.0..=1.0`, matching Python. Like
//! [`crate::hydrus_tag_archive`], the archive owns a plain SQLite connection
//! and is not thread-safe.

use std::fmt::Display;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_tag_archive::HashType;

pub use crate::hydrus_tag_archive::{
    HASH_TYPE_MD5, HASH_TYPE_SHA1, HASH_TYPE_SHA256, HASH_TYPE_SHA512,
};

fn db_error(context: impl Display, error: rusqlite::Error) -> HydrusException {
    HydrusException::new(ErrorKind::Db, format!("{context}: {error}"))
}

fn archive_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Db, message)
}

/// Python's `HydrusRatingArchive`.
pub struct HydrusRatingArchive {
    path: PathBuf,
    connection: Connection,
}

impl HydrusRatingArchive {
    /// `HydrusRatingArchive( path )`
    pub fn new(path: impl AsRef<Path>) -> HydrusResult<Self> {
        let path = path.as_ref().to_path_buf();
        let is_new = !path.exists();

        let connection = Connection::open(&path)
            .map_err(|error| db_error(format!("Could not open \"{}\"", path.display()), error))?;

        if is_new {
            connection
                .execute_batch(
                    "
                    CREATE TABLE hash_type ( hash_type INTEGER );

                    CREATE TABLE number_of_stars ( number_of_stars INTEGER );

                    CREATE TABLE ratings ( hash BLOB PRIMARY KEY, rating REAL );
                    ",
                )
                .map_err(|error| db_error("Could not initialise the rating archive", error))?;
        }

        Ok(Self { path, connection })
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
            .map_err(|error| db_error("Could not commit a big job", error))?;

        self.connection
            .execute_batch("VACUUM;")
            .map_err(|error| db_error("Could not vacuum the rating archive", error))
    }

    /// `delete_rating( hash )`
    pub fn delete_rating(&self, hash: &[u8]) -> HydrusResult<()> {
        self.connection
            .execute("DELETE FROM ratings WHERE hash = ?;", params![hash])
            .map_err(|error| db_error("Could not delete a rating", error))?;

        Ok(())
    }

    /// `get_hash_type`
    ///
    /// Python falls back to guessing from a `hashes` table that rating
    /// archives never create; the port reports the missing hash type instead
    /// of running a query that cannot succeed.
    pub fn get_hash_type(&self) -> HydrusResult<HashType> {
        let hash_type = self
            .connection
            .query_row("SELECT hash_type FROM hash_type;", [], |row| {
                row.get::<_, u8>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not read the hash type", error))?;

        match hash_type {
            Some(hash_type) => HashType::try_from_int(hash_type)
                .ok_or_else(|| archive_error("This archive has an unknown hash type!")),
            None => Err(archive_error(
                "This archive has no hash type set, and as it has no files, no hash type guess can be made.",
            )),
        }
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

    /// `get_number_of_stars`
    pub fn get_number_of_stars(&self) -> HydrusResult<u32> {
        self.connection
            .query_row("SELECT number_of_stars FROM number_of_stars;", [], |row| {
                row.get::<_, u32>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not read the number of stars", error))?
            .ok_or_else(|| archive_error("This rating archive has no number of stars set."))
    }

    /// `get_rating( hash )`
    pub fn get_rating(&self, hash: &[u8]) -> HydrusResult<Option<f64>> {
        self.connection
            .query_row("SELECT rating FROM ratings WHERE hash = ?;", params![hash], |row| {
                row.get::<_, f64>(0)
            })
            .optional()
            .map_err(|error| db_error("Could not read a rating", error))
    }

    /// `has_hash( hash )`
    pub fn has_hash(&self, hash: &[u8]) -> HydrusResult<bool> {
        self.connection
            .query_row("SELECT 1 FROM ratings WHERE hash = ?;", params![hash], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .map(|result| result.is_some())
            .map_err(|error| db_error("Could not look up a rating", error))
    }

    /// `iterate_ratings`. Python yields lazily; the port collects eagerly.
    pub fn iterate_ratings(&self) -> HydrusResult<Vec<(Vec<u8>, f64)>> {
        let mut statement = self
            .connection
            .prepare("SELECT hash, rating FROM ratings;")
            .map_err(|error| db_error("Could not prepare the rating query", error))?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|error| db_error("Could not read ratings", error))?;

        let mut ratings = Vec::new();

        for row in rows {
            ratings.push(row.map_err(|error| db_error("Could not read a rating", error))?);
        }

        Ok(ratings)
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

    /// `set_number_of_stars`
    pub fn set_number_of_stars(&self, number_of_stars: u32) -> HydrusResult<()> {
        self.connection
            .execute("DELETE FROM number_of_stars;", [])
            .map_err(|error| db_error("Could not clear the number of stars", error))?;

        self.connection
            .execute(
                "INSERT INTO number_of_stars ( number_of_stars ) VALUES ( ? );",
                params![number_of_stars],
            )
            .map_err(|error| db_error("Could not set the number of stars", error))?;

        Ok(())
    }

    /// `set_rating( hash, rating )`
    pub fn set_rating(&self, hash: &[u8], rating: f64) -> HydrusResult<()> {
        self.connection
            .execute(
                "REPLACE INTO ratings ( hash, rating ) VALUES ( ?, ? );",
                params![hash, rating],
            )
            .map_err(|error| db_error("Could not set a rating", error))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        let dir = crate::hydrus_temp::get_sub_temp_dir(&format!("rating_archive_{name}"))
            .expect("temp dir");

        dir.join("archive.db")
    }

    #[test]
    fn rating_archive_round_trip() {
        let path = temp_db("round_trip");
        let hash_1 = vec![1u8; 32];
        let hash_2 = vec![2u8; 32];

        {
            let archive = HydrusRatingArchive::new(&path).expect("archive");

            assert!(archive.get_hash_type().is_err());
            assert!(archive.get_number_of_stars().is_err());

            archive.set_hash_type(HashType::Sha256).expect("hash type");
            archive.set_number_of_stars(5).expect("stars");

            assert_eq!(archive.get_hash_type().expect("hash type"), HashType::Sha256);
            assert_eq!(archive.get_number_of_stars().expect("stars"), 5);
            assert_eq!(archive.get_name(), "archive");

            archive.set_rating(&hash_1, 0.75).expect("rating");
            archive.set_rating(&hash_2, 1.0).expect("rating");

            assert_eq!(archive.get_rating(&hash_1).expect("rating"), Some(0.75));
            assert_eq!(archive.get_rating(&hash_2).expect("rating"), Some(1.0));
            assert_eq!(archive.get_rating(&[3u8; 32]).expect("rating"), None);

            assert!(archive.has_hash(&hash_1).expect("has hash"));
            assert!(!archive.has_hash(&[3u8; 32]).expect("has hash"));

            let mut ratings = archive.iterate_ratings().expect("ratings");
            ratings.sort_by(|left, right| left.0.cmp(&right.0));

            assert_eq!(ratings, vec![(hash_1.clone(), 0.75), (hash_2.clone(), 1.0)]);

            archive.delete_rating(&hash_1).expect("delete");

            assert!(!archive.has_hash(&hash_1).expect("has hash"));

            archive.begin_big_job().expect("begin");
            archive.set_rating(&hash_1, 0.25).expect("rating");
            archive.commit_big_job().expect("commit");

            assert_eq!(archive.get_rating(&hash_1).expect("rating"), Some(0.25));
        }

        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }
}
